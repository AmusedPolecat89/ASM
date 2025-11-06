use asm_aut::AnalysisReport;
use asm_core::errors::{AsmError, ErrorInfo};
use asm_spec::{operators::OperatorsInfo, SpectrumReport};
use serde::{Deserialize, Serialize};

use crate::closure::{check_closure, ClosureOpts, ClosureReport};
use crate::decomp::{decompose, DecompOpts, DecompReport};
use crate::hash::stable_hash_string;
use crate::rep::{build_rep, RepOpts};
use crate::ward::{ward_check, WardOpts, WardReport};

fn gauge_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Serde(ErrorInfo::new(code, message))
}

fn commit_string() -> String {
    option_env!("GIT_COMMIT_HASH")
        .or_else(|| option_env!("VERGEN_GIT_SHA"))
        .map(|value| value.to_string())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}

fn default_seed() -> u64 {
    0
}

/// Provenance metadata recorded in [`GaugeReport`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GaugeProvenance {
    /// Source commit or crate version used to produce the report.
    pub commit: String,
    /// Master deterministic seed controlling representation sampling.
    pub seed: u64,
    /// Closure tolerance applied during the analysis.
    pub closure_tol: f64,
    /// Ward tolerance applied during the analysis.
    pub ward_tol: f64,
}

/// Aggregate gauge analysis output for a single state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GaugeReport {
    /// Content addressed hash of the entire gauge analysis.
    pub analysis_hash: String,
    /// Canonical hash of the input graph, forwarded from the spectrum report.
    pub graph_hash: String,
    /// Canonical hash of the input code, forwarded from the spectrum report.
    pub code_hash: String,
    /// Stable hash of the representation matrices.
    pub rep_hash: String,
    /// Closure diagnostics.
    pub closure: ClosureReport,
    /// Factor decomposition diagnostics.
    pub decomp: DecompReport,
    /// Ward-style commutator diagnostics.
    pub ward: WardReport,
    /// Provenance metadata describing the deterministic knobs.
    pub provenance: GaugeProvenance,
}

/// Aggregated options controlling gauge analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GaugeOpts {
    /// Representation construction options.
    #[serde(default)]
    pub rep: RepOpts,
    /// Closure analysis options.
    #[serde(default)]
    pub closure: ClosureOpts,
    /// Decomposition analysis options.
    #[serde(default)]
    pub decomp: DecompOpts,
    /// Ward check options.
    #[serde(default)]
    pub ward: WardOpts,
    /// Master deterministic seed overriding representation defaults.
    #[serde(default = "default_seed")]
    pub seed: u64,
}

impl Default for GaugeOpts {
    fn default() -> Self {
        Self {
            rep: RepOpts::default(),
            closure: ClosureOpts::default(),
            decomp: DecompOpts::default(),
            ward: WardOpts::default(),
            seed: default_seed(),
        }
    }
}

fn make_provenance(opts: &GaugeOpts) -> GaugeProvenance {
    GaugeProvenance {
        commit: commit_string(),
        seed: opts.seed,
        closure_tol: opts.closure.tolerance,
        ward_tol: opts.ward.relative_tol,
    }
}

fn apply_seed_override(mut rep_opts: RepOpts, seed: u64) -> RepOpts {
    if seed != 0 {
        rep_opts.seed = Some(seed);
    }
    rep_opts
}

/// Performs a full gauge analysis using the Phase 11 spectrum artefacts and automorphism report.
pub fn analyze_gauge(
    spectrum: &SpectrumReport,
    aut: &AnalysisReport,
    ops: &OperatorsInfo,
    gopts: &GaugeOpts,
) -> Result<GaugeReport, AsmError> {
    if spectrum.graph_hash != aut.hashes.graph_hash {
        return Err(gauge_error(
            "hash-mismatch",
            "spectrum and automorphism reports refer to different graphs",
        ));
    }
    if spectrum.code_hash != aut.hashes.code_hash {
        return Err(gauge_error(
            "hash-mismatch",
            "spectrum and automorphism reports refer to different codes",
        ));
    }

    let rep_opts = apply_seed_override(gopts.rep.clone(), gopts.seed);
    let rep = build_rep(spectrum, aut, &rep_opts)?;
    let rep_hash = stable_hash_string(&rep)?;
    let closure = check_closure(&rep, &gopts.closure)?;
    let decomp = decompose(&rep, &gopts.decomp)?;
    let ward = ward_check(&rep, ops, &gopts.ward)?;
    let provenance = make_provenance(gopts);

    let mut report = GaugeReport {
        analysis_hash: String::new(),
        graph_hash: spectrum.graph_hash.clone(),
        code_hash: spectrum.code_hash.clone(),
        rep_hash,
        closure,
        decomp,
        ward,
        provenance,
    };

    report.analysis_hash = stable_hash_string(&(
        &report.graph_hash,
        &report.code_hash,
        &report.rep_hash,
        &report.closure,
        &report.decomp,
        &report.ward,
        &report.provenance,
    ))?;

    Ok(report)
}
