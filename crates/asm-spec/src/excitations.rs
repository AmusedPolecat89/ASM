use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::rng::RngHandle;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::operators::Operators;

fn excitation_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Code(ErrorInfo::new(code, message))
}

fn default_support() -> usize {
    3
}

/// Canonical excitation families supported by the spectrum analysis pipeline.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ExcitationKind {
    /// Localised defect probe with minimal support.
    LocalDefect,
    /// Plane-wave excitation sampling a deterministic momentum grid.
    PlaneWave,
    /// Low-weight random probe seeded deterministically.
    RandomLowWeight,
}

#[allow(clippy::derivable_impls)]
impl Default for ExcitationKind {
    fn default() -> Self {
        ExcitationKind::LocalDefect
    }
}

/// Describes how excitations should be seeded prior to propagation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExcitationSpec {
    /// Excitation family to instantiate.
    #[serde(default)]
    pub kind: ExcitationKind,
    /// Desired support size for the excitation.
    #[serde(default = "default_support")]
    pub support: usize,
    /// Optional plane-wave index used when `kind = PlaneWave`.
    #[serde(default)]
    pub plane_wave_k: Option<usize>,
}

impl Default for ExcitationSpec {
    fn default() -> Self {
        Self {
            kind: ExcitationKind::LocalDefect,
            support: default_support(),
            plane_wave_k: None,
        }
    }
}

fn ensure_support_size(nodes: usize, requested: usize) -> Result<usize, AsmError> {
    if nodes == 0 {
        return Err(excitation_error(
            "no-nodes",
            "cannot seed excitations without available nodes",
        ));
    }
    if requested == 0 {
        return Err(excitation_error(
            "invalid-support",
            "excitation support must be at least one",
        ));
    }
    if requested > nodes {
        return Err(excitation_error(
            "support-too-large",
            format!("requested support {requested} exceeds available nodes {nodes}"),
        ));
    }
    Ok(requested)
}

fn select_local_defect(operators: &Operators, support: usize) -> Vec<u64> {
    let mut nodes: Vec<_> = operators.node_degrees.iter().collect();
    nodes.sort_by(|a, b| b.degree.cmp(&a.degree).then_with(|| a.node.cmp(&b.node)));
    nodes
        .into_iter()
        .take(support)
        .map(|entry| entry.node)
        .collect()
}

fn select_plane_wave(operators: &Operators, support: usize, mode: usize) -> Vec<u64> {
    let mut nodes: Vec<_> = operators
        .node_degrees
        .iter()
        .map(|entry| entry.node)
        .collect();
    nodes.sort();
    let len = nodes.len();
    let offset = if len == 0 { 0 } else { mode % len };
    (0..support)
        .map(|idx| nodes[(offset + idx) % len])
        .collect()
}

fn select_random_low_weight(operators: &Operators, support: usize, seed: u64) -> Vec<u64> {
    let mut nodes: Vec<_> = operators
        .node_degrees
        .iter()
        .map(|entry| entry.node)
        .collect();
    nodes.sort();
    let len = nodes.len();
    let mut rng = RngHandle::from_seed(seed);
    for i in 0..support.min(len) {
        let remaining = len - i;
        let choice = (rng.next_u64() as usize) % remaining + i;
        nodes.swap(i, choice);
    }
    nodes.into_iter().take(support.min(len)).collect()
}

pub(crate) fn excitation_support(
    operators: &Operators,
    spec: &ExcitationSpec,
    seed: u64,
) -> Result<Vec<u64>, AsmError> {
    let support = ensure_support_size(operators.node_degrees.len(), spec.support)?;
    let nodes = match spec.kind {
        ExcitationKind::LocalDefect => select_local_defect(operators, support),
        ExcitationKind::PlaneWave => {
            select_plane_wave(operators, support, spec.plane_wave_k.unwrap_or(0))
        }
        ExcitationKind::RandomLowWeight => select_random_low_weight(operators, support, seed),
    };
    Ok(nodes)
}
