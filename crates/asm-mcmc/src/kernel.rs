use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use asm_code::css::{self, CSSCode};
use asm_core::errors::ErrorInfo;
use asm_core::{AsmError, RngHandle};
use asm_graph::{canonical_hash as graph_hash, graph_to_json, HypergraphImpl};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::checkpoint::{self, CheckpointPayload};
use crate::config::{OutputConfig, RunConfig, ScoringWeights};
use crate::determinism;
use crate::energy::{self, EnergyBreakdown};
use crate::manifest::RunManifest;
use crate::metrics::{self, CoverageMetrics, MetricSample, MetricsRecorder};
use crate::moves_code;
use crate::moves_graph;
use crate::moves_worm;
use crate::tempering;

/// Kind of move performed by the sampler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MoveKind {
    /// Toggle support of a generator.
    GeneratorFlip,
    /// Row operation within the CSS code.
    RowOperation,
    /// Swap targets between two hyperedges.
    GraphSwapTargets,
    /// Retarget a hyperedge to a different node.
    GraphRetarget,
    /// Degree-aware resource balancing move.
    GraphResourceBalance,
    /// Logical worm/loop diagnostic.
    WormSample,
}

impl MoveKind {
    fn as_str(&self) -> &'static str {
        match self {
            MoveKind::GeneratorFlip => "generator-flip",
            MoveKind::RowOperation => "row-op",
            MoveKind::GraphSwapTargets => "graph-swap-targets",
            MoveKind::GraphRetarget => "graph-retarget",
            MoveKind::GraphResourceBalance => "graph-resource-balance",
            MoveKind::WormSample => "worm-sample",
        }
    }
}

/// Outcome of a proposal evaluated by the kernel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalOutcome {
    /// Move type that was attempted.
    pub move_kind: MoveKind,
    /// Whether the proposal was accepted.
    pub accepted: bool,
    /// Forward proposal probability reported by the move generator.
    pub forward_prob: f64,
    /// Reverse proposal probability reported by the move generator.
    pub reverse_prob: f64,
    /// Metropolis acceptance probability computed by the kernel.
    pub acceptance_prob: f64,
    /// Human readable description for debugging.
    pub description: String,
}

/// Summary returned to callers after a run completes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunSummary {
    /// Acceptance rates per move kind.
    pub acceptance_rates: BTreeMap<String, f64>,
    /// Temperatures assigned to each replica in the final ladder.
    pub replica_temperatures: Vec<f64>,
    /// Acceptance probability averages for ladder exchanges.
    pub exchange_acceptance: Vec<f64>,
    /// Coverage metrics captured during the run.
    pub coverage: CoverageMetrics,
    /// Crude effective sample size estimate.
    pub effective_sample_size: f64,
    /// Canonical hash of the coldest code state at the end of the run.
    pub final_code_hash: String,
    /// Canonical hash of the coldest graph state at the end of the run.
    pub final_graph_hash: String,
    /// Metrics CSV written during the run.
    pub metrics_path: Option<PathBuf>,
    /// Manifest path, if emitted.
    pub manifest_path: Option<PathBuf>,
    /// Checkpoint files produced during the run.
    pub checkpoints: Vec<PathBuf>,
    /// Metrics samples collected (useful for tests/diagnostics).
    pub samples: Vec<MetricSample>,
}

/// Internal state tracked per replica.
struct ReplicaState {
    temperature: f64,
    code: CSSCode,
    graph: HypergraphImpl,
    energy: EnergyBreakdown,
    accepted: BTreeMap<MoveKind, usize>,
    proposed: BTreeMap<MoveKind, usize>,
}

impl ReplicaState {
    fn new(
        temperature: f64,
        code: CSSCode,
        graph: HypergraphImpl,
        weights: &ScoringWeights,
    ) -> Result<Self, AsmError> {
        let energy = energy::score(&code, &graph, weights)?;
        Ok(Self {
            temperature,
            code,
            graph,
            energy,
            accepted: BTreeMap::new(),
            proposed: BTreeMap::new(),
        })
    }

    fn record(&mut self, kind: MoveKind, accepted: bool) {
        *self.proposed.entry(kind).or_insert(0) += 1;
        if accepted {
            *self.accepted.entry(kind).or_insert(0) += 1;
        }
    }
}

/// Runs the MCMC sampler from scratch with the provided configuration and seed.
pub fn run(
    config: &RunConfig,
    seed: u64,
    code: &CSSCode,
    graph: &HypergraphImpl,
) -> Result<RunSummary, AsmError> {
    let ladder = tempering::build_ladder(&config.ladder);
    let mut replicas = Vec::new();
    for (index, &temperature) in ladder.iter().enumerate() {
        let replica_code = clone_code(code);
        let replica_graph = graph.clone();
        replicas.push(ReplicaState::new(
            temperature,
            replica_code,
            replica_graph,
            &config.scoring,
        )?);
        // ensure deterministic seeds are at least exercised.
        let _ = determinism::replica_seed(seed, index);
    }
    run_with_replicas(config, seed, ladder, replicas, 0, config.sweeps)
}

/// Resumes a run from a checkpoint file.
pub fn resume(path: &Path) -> Result<RunSummary, AsmError> {
    let payload = CheckpointPayload::load(path)?;
    let states = checkpoint::restore_payload(&payload)?;
    if states.is_empty() {
        return Err(AsmError::Serde(
            ErrorInfo::new("empty-checkpoint", "checkpoint contained no replicas")
                .with_context("path", path.display().to_string()),
        ));
    }
    let ladder = tempering::build_ladder(&payload.config.ladder);
    let mut replicas = Vec::new();
    for (idx, (temperature, code, graph, energy)) in states.into_iter().enumerate() {
        let temp = ladder.get(idx).copied().unwrap_or(temperature);
        replicas.push(ReplicaState {
            temperature: temp,
            code,
            graph,
            energy,
            accepted: BTreeMap::new(),
            proposed: BTreeMap::new(),
        });
    }
    let start_sweep = payload.sweep.min(payload.config.sweeps);
    run_with_replicas(
        &payload.config,
        payload.master_seed,
        ladder,
        replicas,
        start_sweep,
        payload.config.sweeps,
    )
}

fn run_with_replicas(
    config: &RunConfig,
    seed: u64,
    ladder: Vec<f64>,
    mut replicas: Vec<ReplicaState>,
    start_sweep: usize,
    total_sweeps: usize,
) -> Result<RunSummary, AsmError> {
    let mut recorder = MetricsRecorder::new();
    let mut checkpoints = Vec::new();
    let output_layout = resolve_output_paths(&config.output);
    let mut exchange_totals = vec![0.0; ladder.len().saturating_sub(1)];
    let mut exchange_counts = vec![0usize; ladder.len().saturating_sub(1)];

    for sweep in start_sweep..total_sweeps {
        for (replica_index, replica) in replicas.iter_mut().enumerate() {
            perform_code_moves(config, seed, sweep, replica_index, replica)?;
            perform_graph_moves(config, seed, sweep, replica_index, replica)?;
            perform_worm_moves(config, seed, sweep, replica_index, replica, &mut recorder)?;
        }

        perform_tempering(
            seed,
            sweep,
            &mut replicas,
            &mut exchange_totals,
            &mut exchange_counts,
        );

        record_metrics(config, sweep, &mut recorder, &replicas)?;

        if config.checkpoint.interval > 0
            && (sweep + 1) % config.checkpoint.interval == 0
            && config.output.run_directory.is_some()
        {
            if let Some(path) = write_checkpoint(config, seed, sweep, &replicas, &output_layout)? {
                checkpoints.push(path);
                enforce_checkpoint_retention(&mut checkpoints, config.checkpoint.max_to_keep)?;
            }
        }
    }

    let cold = &replicas[0];
    let final_code_hash = cold.code.canonical_hash();
    let final_graph_hash = graph_hash(&cold.graph)?;

    let metrics_path = if let (Some(run_dir), Some(metrics_rel)) = (
        output_layout.run_directory.clone(),
        output_layout.metrics_file.clone(),
    ) {
        let path = run_dir.join(metrics_rel);
        recorder.write_csv(&path).map_err(|err| {
            AsmError::Serde(
                asm_core::errors::ErrorInfo::new("metrics-write", err.to_string())
                    .with_context("path", path.display().to_string()),
            )
        })?;
        Some(path)
    } else {
        None
    };

    if let (Some(run_dir), Some(end_state_dir)) = (
        output_layout.run_directory.clone(),
        output_layout.end_state_dir.clone(),
    ) {
        write_end_state(&cold.code, &cold.graph, &run_dir.join(end_state_dir))?;
    }

    let manifest_path = if let Some(run_dir) = output_layout.run_directory.clone() {
        let manifest_path = run_dir.join(output_layout.manifest_file.clone().unwrap_or_default());
        let manifest = RunManifest {
            config: config.clone(),
            master_seed: seed,
            seed_label: config.seed_policy.label.clone(),
            code_hash: final_code_hash.clone(),
            graph_hash: final_graph_hash.clone(),
            metrics_file: metrics_path
                .as_ref()
                .and_then(|path| path.strip_prefix(&run_dir).ok())
                .map(|rel| rel.to_path_buf()),
            checkpoints: checkpoints
                .iter()
                .filter_map(|path| {
                    path.strip_prefix(&run_dir)
                        .ok()
                        .map(|rel| rel.to_path_buf())
                })
                .collect(),
        };
        manifest.write(&manifest_path)?;
        Some(manifest_path)
    } else {
        None
    };

    let coverage = recorder.coverage();
    let effective_sample_size = if recorder.samples().is_empty() {
        0.0
    } else {
        recorder.samples().len() as f64 / (1.0 + coverage.average_jaccard)
    };

    let exchange_acceptance: Vec<f64> = exchange_totals
        .iter()
        .zip(exchange_counts.iter())
        .map(|(total, count)| {
            if *count == 0 {
                0.0
            } else {
                total / *count as f64
            }
        })
        .collect();

    Ok(RunSummary {
        acceptance_rates: aggregate_acceptance(&replicas),
        replica_temperatures: ladder,
        exchange_acceptance,
        coverage,
        effective_sample_size,
        final_code_hash,
        final_graph_hash,
        metrics_path,
        manifest_path,
        checkpoints,
        samples: recorder.samples().to_vec(),
    })
}

fn perform_code_moves(
    config: &RunConfig,
    seed: u64,
    sweep: usize,
    replica_index: usize,
    replica: &mut ReplicaState,
) -> Result<(), AsmError> {
    let counts = &config.move_counts;
    for trial in 0..counts.generator_flips {
        let mut move_rng =
            RngHandle::from_seed(determinism::move_seed(seed, replica_index, sweep, trial));
        match moves_code::propose_generator_flip(&replica.code, &mut move_rng) {
            Ok(proposal) => {
                apply_code_proposal(
                    replica,
                    proposal,
                    MoveKind::GeneratorFlip,
                    &config.scoring,
                    &mut move_rng,
                )?;
            }
            Err(_) => replica.record(MoveKind::GeneratorFlip, false),
        }
    }
    for trial in 0..counts.row_ops {
        let mut move_rng = RngHandle::from_seed(determinism::move_seed(
            seed,
            replica_index,
            sweep,
            counts.generator_flips + trial,
        ));
        match moves_code::propose_row_operation(&replica.code, &mut move_rng) {
            Ok(proposal) => {
                apply_code_proposal(
                    replica,
                    proposal,
                    MoveKind::RowOperation,
                    &config.scoring,
                    &mut move_rng,
                )?;
            }
            Err(_) => replica.record(MoveKind::RowOperation, false),
        }
    }
    Ok(())
}

fn apply_code_proposal(
    replica: &mut ReplicaState,
    proposal: moves_code::CodeMoveProposal,
    kind: MoveKind,
    weights: &ScoringWeights,
    rng: &mut RngHandle,
) -> Result<(), AsmError> {
    let candidate_energy = energy::score(&proposal.candidate, &replica.graph, weights)?;
    let delta = candidate_energy.total - replica.energy.total;
    let acceptance = (-delta / replica.temperature.max(1e-9)).exp().min(1.0);
    let draw = rng.next_u64() as f64 / u64::MAX as f64;
    let accepted = draw < acceptance;
    replica.record(kind, accepted);
    if accepted {
        replica.code = proposal.candidate;
        replica.energy = candidate_energy;
    }
    Ok(())
}

fn perform_graph_moves(
    config: &RunConfig,
    seed: u64,
    sweep: usize,
    replica_index: usize,
    replica: &mut ReplicaState,
) -> Result<(), AsmError> {
    let counts = &config.move_counts;
    for trial in 0..counts.graph_rewires {
        let move_slot = counts.generator_flips + counts.row_ops + trial;
        let mut move_rng = RngHandle::from_seed(determinism::move_seed(
            seed,
            replica_index,
            sweep,
            move_slot,
        ));
        let kind = match trial % 3 {
            0 => MoveKind::GraphSwapTargets,
            1 => MoveKind::GraphRetarget,
            _ => MoveKind::GraphResourceBalance,
        };
        let result = match kind {
            MoveKind::GraphSwapTargets => {
                moves_graph::propose_swap_targets(&replica.graph, &mut move_rng)
            }
            MoveKind::GraphRetarget => moves_graph::propose_retarget(&replica.graph, &mut move_rng),
            MoveKind::GraphResourceBalance => {
                moves_graph::propose_resource_balanced(&replica.graph, &mut move_rng)
            }
            _ => unreachable!(),
        };
        match result {
            Ok(proposal) => {
                apply_graph_proposal(replica, proposal, kind, &config.scoring, &mut move_rng)?;
            }
            Err(_) => replica.record(kind, false),
        }
    }
    Ok(())
}

fn apply_graph_proposal(
    replica: &mut ReplicaState,
    proposal: moves_graph::GraphMoveProposal,
    kind: MoveKind,
    weights: &ScoringWeights,
    rng: &mut RngHandle,
) -> Result<(), AsmError> {
    let candidate_energy = energy::score(&replica.code, &proposal.candidate, weights)?;
    let delta = candidate_energy.total - replica.energy.total;
    let acceptance = (-delta / replica.temperature.max(1e-9)).exp().min(1.0);
    let draw = rng.next_u64() as f64 / u64::MAX as f64;
    let accepted = draw < acceptance;
    replica.record(kind, accepted);
    if accepted {
        replica.graph = proposal.candidate;
        replica.energy = candidate_energy;
    }
    Ok(())
}

fn perform_worm_moves(
    config: &RunConfig,
    seed: u64,
    sweep: usize,
    replica_index: usize,
    replica: &mut ReplicaState,
    recorder: &mut MetricsRecorder,
) -> Result<(), AsmError> {
    for trial in 0..config.move_counts.worm_moves {
        let move_slot = config.move_counts.generator_flips
            + config.move_counts.row_ops
            + config.move_counts.graph_rewires
            + trial;
        let mut move_rng = RngHandle::from_seed(determinism::move_seed(
            seed,
            replica_index,
            sweep,
            move_slot,
        ));
        match moves_worm::propose_worm(&replica.code, &replica.graph, &mut move_rng) {
            Ok(worm) => {
                recorder.note_worm_sample(worm.sample_hash);
                replica.record(MoveKind::WormSample, true);
            }
            Err(_) => replica.record(MoveKind::WormSample, false),
        }
    }
    Ok(())
}

fn perform_tempering(
    seed: u64,
    sweep: usize,
    replicas: &mut [ReplicaState],
    totals: &mut [f64],
    counts: &mut [usize],
) {
    if replicas.len() < 2 {
        return;
    }
    for pair in 0..replicas.len() - 1 {
        let mut rng = RngHandle::from_seed(determinism::exchange_seed(seed, sweep, pair));
        let (accept, prob) = tempering::attempt_exchange(
            replicas[pair].energy.total,
            replicas[pair].temperature,
            replicas[pair + 1].energy.total,
            replicas[pair + 1].temperature,
            &mut rng,
        );
        if let Some(total) = totals.get_mut(pair) {
            *total += prob;
        }
        if let Some(count) = counts.get_mut(pair) {
            *count += 1;
        }
        if accept {
            replicas.swap(pair, pair + 1);
        }
    }
}

fn record_metrics(
    config: &RunConfig,
    sweep: usize,
    recorder: &mut MetricsRecorder,
    replicas: &[ReplicaState],
) -> Result<(), AsmError> {
    if sweep < config.burn_in {
        return Ok(());
    }
    if ((sweep - config.burn_in) % config.thinning) != 0 {
        return Ok(());
    }
    for (replica_index, replica) in replicas.iter().enumerate() {
        let code_hash = replica.code.canonical_hash();
        let graph_hash = graph_hash(&replica.graph)?;
        let generator_signature = generator_signature(&replica.code);
        recorder.push_sample(
            MetricSample {
                sweep,
                replica: replica_index,
                temperature: replica.temperature,
                energy: replica.energy.clone(),
                accepted_moves: replica.accepted.values().copied().sum(),
                proposed_moves: replica.proposed.values().copied().sum(),
                code_hash,
                graph_hash,
            },
            generator_signature,
        );
    }
    Ok(())
}

fn generator_signature(code: &CSSCode) -> std::collections::BTreeSet<usize> {
    let (_, x_checks, z_checks, _, _, _, _) = css::into_parts(code);
    let mut signature = Vec::new();
    for (idx, constraint) in x_checks.iter().enumerate() {
        signature.push(idx + constraint.variables().len());
    }
    let offset = x_checks.len();
    for (idx, constraint) in z_checks.iter().enumerate() {
        signature.push(offset + idx + constraint.variables().len());
    }
    metrics::generator_support_from_constraints(&signature)
}

fn write_checkpoint(
    config: &RunConfig,
    seed: u64,
    sweep: usize,
    replicas: &[ReplicaState],
    layout: &ResolvedOutput,
) -> Result<Option<PathBuf>, AsmError> {
    let run_dir = match &layout.run_directory {
        Some(dir) => dir,
        None => return Ok(None),
    };
    let checkpoint_dir = run_dir.join(layout.checkpoint_dir.clone().unwrap_or_default());
    let path = checkpoint::checkpoint_path(&checkpoint_dir, sweep + 1);
    let replica_refs: Vec<_> = replicas
        .iter()
        .map(|replica| {
            (
                replica.temperature,
                &replica.code,
                &replica.graph,
                &replica.energy,
            )
        })
        .collect();
    let payload = checkpoint::build_payload(sweep + 1, config, seed, &replica_refs)?;
    payload.store(&path)?;
    Ok(Some(path))
}

fn enforce_checkpoint_retention(
    paths: &mut Vec<PathBuf>,
    max_to_keep: usize,
) -> Result<(), AsmError> {
    if paths.len() <= max_to_keep {
        return Ok(());
    }
    let mut removed = Vec::new();
    while paths.len() > max_to_keep {
        removed.push(paths.remove(0));
    }
    for path in removed {
        std::fs::remove_file(&path).map_err(|err| {
            AsmError::Serde(
                asm_core::errors::ErrorInfo::new("checkpoint-remove", err.to_string())
                    .with_context("path", path.display().to_string()),
            )
        })?;
    }
    Ok(())
}

fn aggregate_acceptance(replicas: &[ReplicaState]) -> BTreeMap<String, f64> {
    let mut totals = BTreeMap::<MoveKind, (usize, usize)>::new();
    for replica in replicas {
        for (kind, proposed) in &replica.proposed {
            let entry = totals.entry(*kind).or_insert((0, 0));
            entry.0 += *proposed;
        }
        for (kind, accepted) in &replica.accepted {
            let entry = totals.entry(*kind).or_insert((0, 0));
            entry.1 += *accepted;
        }
    }
    totals
        .into_iter()
        .map(|(kind, (proposed, accepted))| {
            let rate = if proposed == 0 {
                0.0
            } else {
                accepted as f64 / proposed as f64
            };
            (kind.as_str().to_string(), rate)
        })
        .collect()
}

#[derive(Default)]
struct ResolvedOutput {
    run_directory: Option<PathBuf>,
    metrics_file: Option<PathBuf>,
    manifest_file: Option<PathBuf>,
    checkpoint_dir: Option<PathBuf>,
    end_state_dir: Option<PathBuf>,
}

fn resolve_output_paths(config: &OutputConfig) -> ResolvedOutput {
    if config.run_directory.is_none() {
        ResolvedOutput::default()
    } else {
        ResolvedOutput {
            run_directory: config.run_directory.clone(),
            metrics_file: Some(config.metrics_file.clone()),
            manifest_file: Some(config.manifest_file.clone()),
            checkpoint_dir: Some(config.checkpoint_dir.clone()),
            end_state_dir: Some(config.end_state_dir.clone()),
        }
    }
}

fn write_end_state(code: &CSSCode, graph: &HypergraphImpl, dir: &Path) -> Result<(), AsmError> {
    std::fs::create_dir_all(dir).map_err(|err| {
        AsmError::Serde(
            asm_core::errors::ErrorInfo::new("end-state-mkdir", err.to_string())
                .with_context("path", dir.display().to_string()),
        )
    })?;
    let code_json = asm_code::serde::to_json(code)?;
    let graph_json = graph_to_json(graph)?;
    let code_path = dir.join("code.json");
    let graph_path = dir.join("graph.json");
    std::fs::write(&code_path, code_json).map_err(|err| {
        AsmError::Serde(
            asm_core::errors::ErrorInfo::new("end-state-code-write", err.to_string())
                .with_context("path", code_path.display().to_string()),
        )
    })?;
    std::fs::write(&graph_path, graph_json).map_err(|err| {
        AsmError::Serde(
            asm_core::errors::ErrorInfo::new("end-state-graph-write", err.to_string())
                .with_context("path", graph_path.display().to_string()),
        )
    })?;
    Ok(())
}

fn clone_code(code: &CSSCode) -> CSSCode {
    let (vars, x_checks, z_checks, schema, provenance, rank_x, rank_z) = css::into_parts(code);
    css::from_parts(vars, x_checks, z_checks, schema, provenance, rank_x, rank_z)
}
