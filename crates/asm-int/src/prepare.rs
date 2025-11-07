use std::collections::BTreeSet;

use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::rng::{derive_substream_seed, RngHandle};
use asm_gauge::GaugeReport;
use asm_spec::{operators::OperatorEntry, SpectrumReport};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::hash::{round_f64, stable_hash_string};

fn prep_error(code: &str, message: impl Into<String>) -> AsmError {
    AsmError::Dictionary(ErrorInfo::new(code, message.into()))
}

fn default_basis() -> String {
    "modes".to_string()
}

/// Participant template variants supported by the preparation stage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrepTemplate {
    /// Two-body excitation template selecting the lowest momentum modes.
    TwoBody,
    /// Three-body excitation template selecting the lowest momentum modes.
    ThreeBody,
}

impl PrepTemplate {
    fn participant_count(&self) -> usize {
        match self {
            PrepTemplate::TwoBody => 2,
            PrepTemplate::ThreeBody => 3,
        }
    }
}

/// Declarative participant specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticipantSpec {
    /// Mode identifier taken from the spectrum operator indices.
    pub mode_id: usize,
    /// Momentum magnitude assigned to the participant.
    pub k: f64,
    /// Effective charge carried by the participant.
    pub charge: f64,
}

impl ParticipantSpec {
    fn from_entry(entry: &OperatorEntry, charge: f64) -> Self {
        Self {
            mode_id: entry.row,
            k: (entry.row as f64 + entry.col as f64 + 1.0) / 2.0,
            charge,
        }
    }
}

/// Preparation configuration controlling participant selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrepSpec {
    /// Basis label recorded alongside the prepared state.
    #[serde(default = "default_basis")]
    pub basis: String,
    /// Explicit participant definitions.
    #[serde(default)]
    pub participants: Vec<ParticipantSpec>,
    /// Optional template used when `participants` is empty.
    pub template: Option<PrepTemplate>,
    /// Overrides the default normalisation if provided.
    pub norm_override: Option<f64>,
}

impl Default for PrepSpec {
    fn default() -> Self {
        Self {
            basis: default_basis(),
            participants: Vec::new(),
            template: Some(PrepTemplate::TwoBody),
            norm_override: None,
        }
    }
}

/// Materialised participant with deterministic rounding applied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreparedParticipant {
    /// Mode identifier used during the interaction experiment.
    pub mode_id: usize,
    /// Momentum magnitude.
    pub k: f64,
    /// Assigned charge.
    pub charge: f64,
}

/// Deterministic prepared state descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreparedState {
    /// Basis used to interpret the participant list.
    pub basis: String,
    /// Participants included in the initial state.
    pub participants: Vec<PreparedParticipant>,
    /// Normalisation constant applied to the state.
    pub norm: f64,
    /// Stable hash of the preparation record.
    pub prep_hash: String,
}

fn build_from_template(
    spec: &SpectrumReport,
    template: &PrepTemplate,
    gauge: &GaugeReport,
) -> Result<Vec<ParticipantSpec>, AsmError> {
    if spec.operators.entries.is_empty() {
        return Err(prep_error(
            "missing-operators",
            "spectrum report does not contain operator entries",
        ));
    }
    if spec.graph_hash != gauge.graph_hash || spec.code_hash != gauge.code_hash {
        return Err(prep_error(
            "hash-mismatch",
            "spectrum and gauge reports describe different states",
        ));
    }

    let count = template.participant_count();
    let mut participants = Vec::new();
    let mut charges = vec![1.0; count];
    if count == 2 {
        charges[1] = -1.0;
    } else if count == 3 {
        charges[1] = -1.0;
        charges[2] = 0.0;
    }
    for (idx, entry) in spec.operators.entries.iter().take(count).enumerate() {
        participants.push(ParticipantSpec::from_entry(entry, charges[idx]));
    }
    Ok(participants)
}

fn validate_participants(
    spec: &SpectrumReport,
    participants: &[ParticipantSpec],
) -> Result<(), AsmError> {
    if participants.is_empty() {
        return Err(prep_error(
            "empty-participants",
            "at least one participant must be provided",
        ));
    }
    let mut seen = BTreeSet::new();
    let entry_count = spec.operators.entries.len();
    for part in participants {
        if part.mode_id >= entry_count {
            return Err(prep_error(
                "unknown-mode",
                format!("mode_id {} is out of range", part.mode_id),
            ));
        }
        if !part.k.is_finite() || !part.charge.is_finite() {
            return Err(prep_error(
                "non-finite",
                "participant momentum and charge must be finite",
            ));
        }
        if !seen.insert(part.mode_id) {
            return Err(prep_error(
                "duplicate-mode",
                format!("mode_id {} appears multiple times", part.mode_id),
            ));
        }
    }
    Ok(())
}

fn derive_norm(
    spec: &SpectrumReport,
    participants: &[ParticipantSpec],
    norm_override: Option<f64>,
) -> Result<f64, AsmError> {
    if let Some(norm) = norm_override {
        if norm <= 0.0 {
            return Err(prep_error(
                "invalid-norm",
                "norm override must be strictly positive",
            ));
        }
        return Ok(round_f64(norm));
    }
    let mut sum_sq = 0.0;
    for part in participants {
        let entry = &spec.operators.entries[part.mode_id];
        sum_sq += entry.weight * entry.weight + part.k * part.k;
    }
    Ok(round_f64(sum_sq.sqrt()))
}

fn assign_momenta(participants: &[ParticipantSpec], seed: u64) -> Vec<PreparedParticipant> {
    let mut rng = RngHandle::from_seed(seed);
    participants
        .iter()
        .map(|spec| {
            let noise = (rng.gen::<f64>() - 0.5) * 0.000_000_05;
            PreparedParticipant {
                mode_id: spec.mode_id,
                k: round_f64(spec.k + noise),
                charge: round_f64(spec.charge),
            }
        })
        .collect()
}

/// Builds a deterministic few-body initial state from the provided reports and configuration.
pub fn prepare_state(
    spec: &SpectrumReport,
    gauge: &GaugeReport,
    conf: &PrepSpec,
    seed: u64,
) -> Result<PreparedState, AsmError> {
    let participants = if !conf.participants.is_empty() {
        conf.participants.clone()
    } else if let Some(template) = &conf.template {
        build_from_template(spec, template, gauge)?
    } else {
        return Err(prep_error(
            "missing-participants",
            "no participants or template provided",
        ));
    };
    validate_participants(spec, &participants)?;

    let total_charge: f64 = participants.iter().map(|p| p.charge).sum();
    if round_f64(total_charge.abs()) > 1e-6 {
        return Err(prep_error(
            "charge-imbalance",
            "sum of participant charges must vanish within tolerance",
        ));
    }

    let norm = derive_norm(spec, &participants, conf.norm_override)?;
    let prep_seed = derive_substream_seed(seed, 1);
    let prepared = assign_momenta(&participants, prep_seed);
    let prep_hash = stable_hash_string(&(&conf.basis, &prepared, norm, seed))?;

    Ok(PreparedState {
        basis: conf.basis.clone(),
        participants: prepared,
        norm,
        prep_hash,
    })
}
