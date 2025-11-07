use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::metrics::JobKpi;

/// Deterministic histogram descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Histogram {
    /// Bin edges (inclusive of the left edge, exclusive of the right edge except the last bin).
    pub edges: Vec<f64>,
    /// Counts recorded per bin.
    pub counts: Vec<u64>,
}

/// Quantile summary for a single metric.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantiles {
    /// 5th percentile estimate.
    pub q05: f64,
    /// Median (50th percentile) estimate.
    pub q50: f64,
    /// 95th percentile estimate.
    pub q95: f64,
}

/// Correlation descriptor between a fixed metric pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Correlations {
    /// Pearson correlation coefficient.
    pub pearson: f64,
    /// Spearman rank correlation coefficient.
    pub spearman: f64,
}

/// Aggregate statistics extracted from a set of KPIs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatsSummary {
    /// Histograms keyed by metric name.
    pub histograms: BTreeMap<String, Histogram>,
    /// Quantile summaries keyed by metric name.
    pub quantiles: BTreeMap<String, Quantiles>,
    /// Correlations keyed by metric pair name.
    pub correlations: BTreeMap<String, Correlations>,
}

impl StatsSummary {
    /// Builds a deterministic summary for the provided KPI collection.
    pub fn from_kpis(kpis: &[JobKpi]) -> Self {
        let mut histograms = BTreeMap::new();
        histograms.insert(
            "c_est".to_string(),
            histogram(kpis, |kpi| kpi.c_est, 0.4, 1.6, 6),
        );
        histograms.insert(
            "gap_proxy".to_string(),
            histogram(kpis, |kpi| kpi.gap_proxy, 0.0, 0.4, 5),
        );

        let mut quantiles = BTreeMap::new();
        quantiles.insert("c_est".to_string(), quantile_summary(kpis, |kpi| kpi.c_est));
        quantiles.insert(
            "gap_proxy".to_string(),
            quantile_summary(kpis, |kpi| kpi.gap_proxy),
        );

        let mut correlations = BTreeMap::new();
        correlations.insert(
            "c_est_vs_gap".to_string(),
            correlation_summary(kpis, |kpi| kpi.c_est, |kpi| kpi.gap_proxy),
        );

        Self {
            histograms,
            quantiles,
            correlations,
        }
    }
}

fn histogram<F>(kpis: &[JobKpi], map: F, start: f64, end: f64, bins: usize) -> Histogram
where
    F: Fn(&JobKpi) -> f64,
{
    let mut edges = Vec::with_capacity(bins + 1);
    let step = if bins == 0 {
        1.0
    } else {
        (end - start) / bins as f64
    };
    for idx in 0..=bins {
        edges.push(start + idx as f64 * step);
    }
    let mut counts = vec![0u64; bins];
    for value in kpis.iter().map(map) {
        let mut bin = ((value - start) / step).floor() as isize;
        if bin < 0 {
            bin = 0;
        }
        if bin as usize >= bins {
            bin = (bins as isize) - 1;
        }
        counts[bin as usize] += 1;
    }
    Histogram { edges, counts }
}

fn quantile_summary<F>(kpis: &[JobKpi], map: F) -> Quantiles
where
    F: Fn(&JobKpi) -> f64,
{
    let mut values: Vec<f64> = kpis.iter().map(map).collect();
    if values.is_empty() {
        return Quantiles {
            q05: f64::NAN,
            q50: f64::NAN,
            q95: f64::NAN,
        };
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Quantiles {
        q05: percentile(&values, 0.05),
        q50: percentile(&values, 0.5),
        q95: percentile(&values, 0.95),
    }
}

fn percentile(values: &[f64], quantile: f64) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    let position = quantile * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        values[lower]
    } else {
        let weight = position - lower as f64;
        values[lower] * (1.0 - weight) + values[upper] * weight
    }
}

fn correlation_summary<F, G>(kpis: &[JobKpi], xf: F, yf: G) -> Correlations
where
    F: Fn(&JobKpi) -> f64,
    G: Fn(&JobKpi) -> f64,
{
    if kpis.is_empty() {
        return Correlations {
            pearson: f64::NAN,
            spearman: f64::NAN,
        };
    }
    let xs: Vec<f64> = kpis.iter().map(&xf).collect();
    let ys: Vec<f64> = kpis.iter().map(&yf).collect();
    Correlations {
        pearson: pearson(&xs, &ys),
        spearman: pearson(&rank(&xs), &rank(&ys)),
    }
}

fn pearson(xs: &[f64], ys: &[f64]) -> f64 {
    let len = xs.len();
    if len == 0 {
        return f64::NAN;
    }
    let mean_x = xs.iter().sum::<f64>() / len as f64;
    let mean_y = ys.iter().sum::<f64>() / len as f64;
    let mut num = 0.0;
    let mut denom_x = 0.0;
    let mut denom_y = 0.0;
    for (x, y) in xs.iter().zip(ys.iter()) {
        let dx = x - mean_x;
        let dy = y - mean_y;
        num += dx * dy;
        denom_x += dx * dx;
        denom_y += dy * dy;
    }
    if denom_x == 0.0 || denom_y == 0.0 {
        return 0.0;
    }
    num / (denom_x.sqrt() * denom_y.sqrt())
}

fn rank(values: &[f64]) -> Vec<f64> {
    let mut pairs: Vec<(usize, f64)> = values.iter().cloned().enumerate().collect();
    pairs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let mut ranks = vec![0.0; values.len()];
    let mut idx = 0;
    while idx < pairs.len() {
        let start = idx;
        let value = pairs[idx].1;
        while idx < pairs.len() && pairs[idx].1 == value {
            idx += 1;
        }
        let rank_value = (start + idx - 1) as f64 / 2.0 + 1.0;
        for &(original_idx, _) in &pairs[start..idx] {
            ranks[original_idx] = rank_value;
        }
    }
    ranks
}
