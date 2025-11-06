#![deny(missing_docs)]
#![doc = "Deterministic renormalisation group and operator dictionary utilities for ASM states."]

/// Deterministic node partitioning utilities.
pub mod block;
/// CSS-preserving contraction helpers.
pub mod contract;
/// Covariance analysis between RG and dictionary flows.
pub mod covariance;
/// Deterministic operator dictionary extraction.
pub mod dictionary;
/// Hypergraph coarsening helpers.
pub mod graph_coarse;
/// Canonical hashing helpers for RG artefacts.
pub mod hash;
/// CSS isometry evaluation utilities.
pub mod isometry;
/// RG and dictionary option structures.
pub mod params;
/// Serde helpers for JSON artefacts.
#[path = "serde.rs"]
pub mod serde_io;

use asm_code::CSSCode;
use asm_core::errors::AsmError;
use asm_graph::HypergraphImpl;
use serde::{Deserialize, Serialize};

use block::partition_nodes;
use contract::apply_contract;
use graph_coarse::coarsen_graph;
use hash::{hash_run, hash_step};

pub use covariance::{CovarianceDelta, CovarianceReport};
pub use dictionary::{CouplingIntervals, CouplingsReport, DictionaryProvenance};
pub use params::{CovarianceThresholds, DictOpts, RGOpts};

/// Borrowed reference to a code/graph pair used as RG input.
#[derive(Debug, Clone, Copy)]
pub struct StateRef<'a> {
    /// Underlying hypergraph for the state.
    pub graph: &'a HypergraphImpl,
    /// CSS stabiliser code associated with the state.
    pub code: &'a CSSCode,
}

/// Report describing a single RG step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RGStepReport {
    /// Canonical hash of the coarse graph.
    pub graph_hash: String,
    /// Canonical hash of the coarse code.
    pub code_hash: String,
    /// Scaling factor requested by the caller.
    pub scale_factor: usize,
    /// Fraction of constraints retained.
    pub kept_fraction: f64,
    /// Number of constraints removed during the step.
    pub lost_constraints: usize,
    /// Whether CSS structure was preserved.
    pub css_preserved: bool,
    /// Whether the procedure respected recorded symmetries.
    pub symmetry_equivariant: bool,
    /// Human readable notes about the step.
    pub notes: String,
    /// Canonical hash of the step metadata.
    pub step_hash: String,
}

/// Materialised RG step with associated coarse state.
#[derive(Debug)]
pub struct RGStep {
    /// Coarse grained hypergraph.
    pub graph: HypergraphImpl,
    /// Coarse grained CSS code.
    pub code: CSSCode,
    /// Structured metadata describing the transformation.
    pub report: RGStepReport,
}

/// Summary of an entire RG trajectory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RGRunReport {
    /// Hash of the initial graph.
    pub initial_graph_hash: String,
    /// Hash of the initial code.
    pub initial_code_hash: String,
    /// Hash of the final graph.
    pub final_graph_hash: String,
    /// Hash of the final code.
    pub final_code_hash: String,
    /// Per-step summaries recorded along the trajectory.
    pub steps: Vec<RGRunEntry>,
    /// Deterministic content addressed hash of the run report.
    pub run_hash: String,
}

/// Per-step summary included within [`RGRunReport`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RGRunEntry {
    /// Step index.
    pub index: usize,
    /// Scaling factor applied during the step.
    pub scale_factor: usize,
    /// Fraction of constraints retained.
    pub kept_fraction: f64,
    /// Number of constraints removed.
    pub lost_constraints: usize,
    /// Whether CSS structure was preserved.
    pub css_preserved: bool,
    /// Whether symmetry equivariance was maintained.
    pub symmetry_equivariant: bool,
    /// Canonical hash of the coarse graph.
    pub graph_hash: String,
    /// Canonical hash of the coarse code.
    pub code_hash: String,
    /// Deterministic hash of the step metadata.
    pub step_hash: String,
    /// Human readable notes emitted for the step.
    pub notes: String,
}

/// Materialised RG trajectory.
#[derive(Debug)]
pub struct RGRun {
    /// Per-step coarse states and metadata.
    pub steps: Vec<RGStep>,
    /// Summary report describing the full trajectory.
    pub report: RGRunReport,
}

/// Applies a single RG step to the provided state.
pub fn rg_step(graph: &HypergraphImpl, code: &CSSCode, opts: &RGOpts) -> Result<RGStep, AsmError> {
    let partition = partition_nodes(graph, opts)?;
    let contracted = apply_contract(code, &partition)?;
    let coarse_graph = coarsen_graph(graph)?;

    let graph_hash = asm_graph::canonical_hash(&coarse_graph.graph)?;
    let code_hash = asm_code::hash::canonical_code_hash(&contracted.code);
    let notes = format!(
        "blocks={} scale={}",
        partition.blocks().len(),
        opts.scale_factor
    );

    let mut report = RGStepReport {
        graph_hash,
        code_hash,
        scale_factor: opts.scale_factor,
        kept_fraction: contracted.summary.kept_fraction,
        lost_constraints: contracted.summary.lost_constraints,
        css_preserved: contracted.summary.css_preserved,
        symmetry_equivariant: true,
        notes,
        step_hash: String::new(),
    };
    report.step_hash = hash_step(&report)?;

    Ok(RGStep {
        graph: coarse_graph.graph,
        code: contracted.code,
        report,
    })
}

/// Runs a deterministic RG trajectory for `steps` iterations.
pub fn rg_run(input: &StateRef, steps: usize, opts: &RGOpts) -> Result<RGRun, AsmError> {
    let mut current_graph = clone_graph(input.graph)?;
    let mut current_code = clone_code(input.code);

    let initial_graph_hash = asm_graph::canonical_hash(&current_graph)?;
    let initial_code_hash = asm_code::hash::canonical_code_hash(&current_code);

    let mut run_steps = Vec::new();
    let mut entries = Vec::new();
    for index in 0..steps {
        let step = rg_step(&current_graph, &current_code, opts)?;
        let next_graph = clone_graph(&step.graph)?;
        let next_code = clone_code(&step.code);
        entries.push(RGRunEntry {
            index,
            scale_factor: step.report.scale_factor,
            kept_fraction: step.report.kept_fraction,
            lost_constraints: step.report.lost_constraints,
            css_preserved: step.report.css_preserved,
            symmetry_equivariant: step.report.symmetry_equivariant,
            graph_hash: step.report.graph_hash.clone(),
            code_hash: step.report.code_hash.clone(),
            step_hash: step.report.step_hash.clone(),
            notes: step.report.notes.clone(),
        });
        current_graph = next_graph;
        current_code = next_code;
        run_steps.push(step);
    }

    let final_graph_hash = asm_graph::canonical_hash(&current_graph)?;
    let final_code_hash = asm_code::hash::canonical_code_hash(&current_code);

    let mut report = RGRunReport {
        initial_graph_hash,
        initial_code_hash,
        final_graph_hash,
        final_code_hash,
        steps: entries,
        run_hash: String::new(),
    };
    report.run_hash = hash_run(&report)?;

    Ok(RGRun {
        steps: run_steps,
        report,
    })
}

/// Clones a hypergraph using the deterministic serializer.
fn clone_graph(graph: &HypergraphImpl) -> Result<HypergraphImpl, AsmError> {
    Ok(coarsen_graph(graph)?.graph)
}

/// Clones a CSS code using canonical parts.
fn clone_code(code: &CSSCode) -> CSSCode {
    let (num_variables, x_checks, z_checks, schema, provenance, rank_x, rank_z) =
        asm_code::css::into_parts(code);
    asm_code::css::from_parts(
        num_variables,
        x_checks,
        z_checks,
        schema,
        provenance,
        rank_x,
        rank_z,
    )
}
