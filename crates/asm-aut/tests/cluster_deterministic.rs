use asm_aut::{analyze_state, cluster, ClusterOpts, ScanOpts};
use asm_core::AsmError;

mod fixtures;

#[test]
fn clustering_is_stable() -> Result<(), AsmError> {
    let names = ["t1_seed0", "t1_seed1", "t3_worm", "t3_noworm"];
    let mut reports = Vec::new();
    for name in names {
        let fixture = fixtures::load_fixture(name)?;
        let provenance = fixtures::provenance_from_manifest(&fixture.manifest);
        let mut opts = ScanOpts::default();
        opts.provenance = Some(provenance);
        reports.push(analyze_state(&fixture.graph, &fixture.code, &opts)?);
    }

    let cluster_opts = ClusterOpts {
        k: 2,
        max_iterations: 8,
        seed: 0xA5A5,
    };
    let summary_a = cluster(&reports, &cluster_opts);
    let summary_b = cluster(&reports, &cluster_opts);
    assert_eq!(summary_a.clusters.len(), summary_b.clusters.len());
    for (a, b) in summary_a.clusters.iter().zip(summary_b.clusters.iter()) {
        assert_eq!(a.cluster_id, b.cluster_id);
        assert_eq!(a.members, b.members);
    }
    assert!(summary_a.clusters.len() >= 2);
    Ok(())
}
