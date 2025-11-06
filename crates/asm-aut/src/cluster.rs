use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{AnalysisReport, ClusterOpts};

/// Cluster level summary describing membership and representatives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterInfo {
    /// Stable identifier assigned to the cluster.
    pub cluster_id: usize,
    /// Number of reports contained within the cluster.
    pub size: usize,
    /// Analysis hash of the member closest to the centroid.
    pub centroid_report_hash: String,
    /// Ordered list of analysis hashes belonging to the cluster.
    pub members: Vec<String>,
    /// Fractional occupancy of the cluster across all members.
    pub occupancy: f64,
}

pub(crate) fn cluster_reports(reports: &[AnalysisReport], opts: &ClusterOpts) -> Vec<ClusterInfo> {
    if reports.is_empty() {
        return Vec::new();
    }
    let k = opts.k.max(1).min(reports.len());
    let features: Vec<Vec<f64>> = reports.iter().map(feature_vector).collect();
    let mut centroids = initialise_centroids(&features, k, reports);
    let mut assignments = vec![0usize; reports.len()];

    for _ in 0..opts.max_iterations.max(1) {
        let updated = assign_clusters(&features, &centroids, &mut assignments);
        recompute_centroids(&features, &assignments, k, &mut centroids);
        if !updated {
            break;
        }
    }

    build_summary(reports, &features, &assignments, &centroids)
}

fn feature_vector(report: &AnalysisReport) -> Vec<f64> {
    let mut vector = Vec::new();
    let total_orbits: f64 = report.graph_aut.orbit_hist.iter().map(|&v| v as f64).sum();
    if total_orbits > 0.0 {
        for &entry in &report.graph_aut.orbit_hist {
            vector.push(entry as f64 / total_orbits);
        }
    }
    vector.push((report.graph_aut.order as f64 + 1.0).ln());
    vector.push((report.code_aut.order as f64 + 1.0).ln());
    vector.push(report.logical.rank_x as f64);
    vector.push(report.logical.rank_z as f64);
    vector.extend(report.spectral.laplacian_topk.iter().cloned());
    vector.extend(report.spectral.stabilizer_topk.iter().cloned());
    vector
}

fn initialise_centroids(
    features: &[Vec<f64>],
    k: usize,
    reports: &[AnalysisReport],
) -> Vec<Vec<f64>> {
    let mut indexed: Vec<(usize, &str)> = reports
        .iter()
        .enumerate()
        .map(|(idx, report)| (idx, report.hashes.analysis_hash.as_str()))
        .collect();
    indexed.sort_by(|a, b| a.1.cmp(b.1));
    let mut centroids = Vec::new();
    for (idx, _) in indexed.into_iter().take(k) {
        centroids.push(features[idx].clone());
    }
    centroids
}

fn assign_clusters(
    features: &[Vec<f64>],
    centroids: &[Vec<f64>],
    assignments: &mut [usize],
) -> bool {
    let mut changed = false;
    for (idx, feature) in features.iter().enumerate() {
        let mut best = 0usize;
        let mut best_dist = f64::INFINITY;
        for (cluster_idx, centroid) in centroids.iter().enumerate() {
            let dist = euclidean_distance(feature, centroid);
            if dist < best_dist {
                best = cluster_idx;
                best_dist = dist;
            }
        }
        if assignments[idx] != best {
            assignments[idx] = best;
            changed = true;
        }
    }
    changed
}

fn recompute_centroids(
    features: &[Vec<f64>],
    assignments: &[usize],
    k: usize,
    centroids: &mut Vec<Vec<f64>>,
) {
    let mut accumulators: BTreeMap<usize, Vec<f64>> = BTreeMap::new();
    let mut counts = vec![0usize; k];
    for (idx, feature) in features.iter().enumerate() {
        let cluster = assignments[idx];
        counts[cluster] += 1;
        let entry = accumulators
            .entry(cluster)
            .or_insert_with(|| vec![0.0; feature.len()]);
        for (slot, value) in feature.iter().enumerate() {
            if slot >= entry.len() {
                entry.resize(slot + 1, 0.0);
            }
            entry[slot] += value;
        }
    }
    for cluster in 0..k {
        if let Some(sum) = accumulators.get(&cluster) {
            let mut centroid = sum.clone();
            let denom = counts[cluster] as f64;
            if denom > 0.0 {
                for value in &mut centroid {
                    *value /= denom;
                }
            }
            if cluster < centroids.len() {
                centroids[cluster] = centroid;
            } else {
                centroids.push(centroid);
            }
        }
    }
}

fn build_summary(
    reports: &[AnalysisReport],
    features: &[Vec<f64>],
    assignments: &[usize],
    centroids: &[Vec<f64>],
) -> Vec<ClusterInfo> {
    let total = reports.len() as f64;
    let mut cluster_members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (idx, &cluster) in assignments.iter().enumerate() {
        cluster_members.entry(cluster).or_default().push(idx);
    }

    let mut summaries = Vec::new();
    for (cluster_id, members) in cluster_members {
        let size = members.len();
        let centroid_idx = select_representative(
            members.as_slice(),
            features,
            centroids[cluster_id].as_slice(),
        );
        let centroid_hash = reports[centroid_idx].hashes.analysis_hash.clone();
        let mut member_hashes: Vec<String> = members
            .iter()
            .map(|&idx| reports[idx].hashes.analysis_hash.clone())
            .collect();
        member_hashes.sort();
        summaries.push(ClusterInfo {
            cluster_id,
            size,
            centroid_report_hash: centroid_hash,
            members: member_hashes,
            occupancy: size as f64 / total,
        });
    }
    summaries.sort_by_key(|info| info.cluster_id);
    summaries
}

fn select_representative(members: &[usize], features: &[Vec<f64>], centroid: &[f64]) -> usize {
    let mut best_member = members[0];
    let mut best_dist = f64::INFINITY;
    for &idx in members {
        let dist = euclidean_distance(&features[idx], centroid);
        if dist < best_dist {
            best_dist = dist;
            best_member = idx;
        }
    }
    best_member
}

fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    let max_len = a.len().max(b.len());
    let mut sum = 0.0;
    for idx in 0..max_len {
        let va = if idx < a.len() { a[idx] } else { 0.0 };
        let vb = if idx < b.len() { b[idx] } else { 0.0 };
        sum += (va - vb).powi(2);
    }
    sum.sqrt()
}
