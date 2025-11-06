use std::collections::BTreeSet;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::energy::EnergyBreakdown;

/// Per-sweep metrics stored for CSV export.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricSample {
    /// Sweep number (post burn-in) when the sample was recorded.
    pub sweep: usize,
    /// Replica index within the ladder.
    pub replica: usize,
    /// Temperature of the replica when sampling.
    pub temperature: f64,
    /// Energy breakdown for the replica state.
    pub energy: EnergyBreakdown,
    /// Number of accepted proposals within the sweep.
    pub accepted_moves: usize,
    /// Number of proposals issued within the sweep.
    pub proposed_moves: usize,
    /// Canonical hash of the code state.
    pub code_hash: String,
    /// Canonical hash of the graph state.
    pub graph_hash: String,
}

/// Aggregate coverage metrics summarising the exploration quality.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoverageMetrics {
    /// Number of unique structural hashes encountered during the run.
    pub unique_state_hashes: usize,
    /// Number of logical worm samples recorded.
    pub worm_samples: usize,
    /// Mean energy over the recorded samples.
    pub mean_energy: f64,
    /// Variance of the recorded energy values.
    pub energy_variance: f64,
    /// Average Jaccard similarity between consecutive generator supports.
    pub average_jaccard: f64,
}

impl CoverageMetrics {
    /// Returns an empty coverage descriptor.
    pub fn empty() -> Self {
        Self {
            unique_state_hashes: 0,
            worm_samples: 0,
            mean_energy: 0.0,
            energy_variance: 0.0,
            average_jaccard: 1.0,
        }
    }
}

/// Collects per-sweep metrics and computes aggregate coverage proxies.
#[derive(Debug, Default)]
pub struct MetricsRecorder {
    samples: Vec<MetricSample>,
    unique_hashes: IndexSet<String>,
    worm_hashes: IndexSet<String>,
    generator_history: Vec<BTreeSet<usize>>,
}

impl MetricsRecorder {
    /// Creates a new recorder instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a metrics sample together with the generator support set used for coverage.
    pub fn push_sample(&mut self, sample: MetricSample, generator_support: BTreeSet<usize>) {
        self.unique_hashes
            .insert(format!("{}::{}", sample.code_hash, sample.graph_hash));
        self.samples.push(sample);
        self.generator_history.push(generator_support);
    }

    /// Tracks a worm sample identified by its deterministic hash.
    pub fn note_worm_sample(&mut self, hash: String) {
        self.worm_hashes.insert(hash);
    }

    /// Returns an immutable view over the recorded samples.
    pub fn samples(&self) -> &[MetricSample] {
        &self.samples
    }

    /// Computes coverage metrics from the recorded data.
    pub fn coverage(&self) -> CoverageMetrics {
        if self.samples.is_empty() {
            return CoverageMetrics::empty();
        }
        let energies: Vec<f64> = self
            .samples
            .iter()
            .map(|sample| sample.energy.total)
            .collect();
        let mean_energy = energies.iter().sum::<f64>() / energies.len() as f64;
        let variance = if energies.len() > 1 {
            let mean_sq = energies.iter().map(|&e| e * e).sum::<f64>() / energies.len() as f64;
            (mean_sq - mean_energy * mean_energy).max(0.0)
        } else {
            0.0
        };

        let mut jaccard_sum = 0.0;
        let mut jaccard_count = 0usize;
        for pair in self.generator_history.windows(2) {
            if let [a, b] = pair {
                let intersection = a.intersection(b).count() as f64;
                let union = (a.len() + b.len()) as f64 - intersection;
                if union > 0.0 {
                    jaccard_sum += intersection / union;
                    jaccard_count += 1;
                }
            }
        }
        let average_jaccard = if jaccard_count > 0 {
            jaccard_sum / jaccard_count as f64
        } else {
            1.0
        };

        CoverageMetrics {
            unique_state_hashes: self.unique_hashes.len(),
            worm_samples: self.worm_hashes.len(),
            mean_energy,
            energy_variance: variance,
            average_jaccard,
        }
    }

    /// Writes the recorded metrics to a CSV file.
    pub fn write_csv<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        writeln!(
            file,
            "sweep,replica,temperature,energy,cmdl,spec,curv,accepted,proposed,code_hash,graph_hash"
        )?;
        for sample in &self.samples {
            writeln!(
                file,
                "{},{},{},{:.6},{:.6},{:.6},{:.6},{},{},{},{}",
                sample.sweep,
                sample.replica,
                sample.temperature,
                sample.energy.total,
                sample.energy.cmdl,
                sample.energy.spec,
                sample.energy.curv,
                sample.accepted_moves,
                sample.proposed_moves,
                sample.code_hash,
                sample.graph_hash
            )?;
        }
        Ok(())
    }
}

/// Builds a deterministic generator support set for coverage metrics.
pub fn generator_support_from_constraints(constraints: &[usize]) -> BTreeSet<usize> {
    constraints.iter().copied().collect()
}
