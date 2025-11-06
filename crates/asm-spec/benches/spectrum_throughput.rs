use std::fs;
use std::path::PathBuf;
use std::sync::Once;

use asm_code::{serde as code_serde, CSSCode};
use asm_graph::{graph_from_json, HypergraphImpl};
use asm_spec::{
    analyze_spectrum, to_canonical_json_bytes, CorrelSpec, DispersionSpec, ExcitationSpec, OpOpts,
    PropOpts, SpecOpts,
};
use criterion::{criterion_group, criterion_main, Criterion};

fn load_fixture() -> (CSSCode, HypergraphImpl) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let code_path = base.join("fixtures/validation_vacua/t1_seed0/end_state/code.json");
    let graph_path = base.join("fixtures/validation_vacua/t1_seed0/end_state/graph.json");
    let code_json = fs::read_to_string(code_path).expect("code fixture");
    let graph_json = fs::read_to_string(graph_path).expect("graph fixture");
    let code = code_serde::from_json(&code_json).expect("decode code");
    let graph = graph_from_json(&graph_json).expect("decode graph");
    (code, graph)
}

fn make_opts(seed: u64) -> SpecOpts {
    let mut dispersion = DispersionSpec::default();
    dispersion.k_points = 32;
    dispersion.modes = 2;
    SpecOpts {
        ops: OpOpts::default(),
        excitation: ExcitationSpec::default(),
        propagation: PropOpts {
            iterations: 16,
            tolerance: 1e-6,
            seed: seed + 1,
        },
        dispersion,
        correlation: CorrelSpec::default(),
        master_seed: seed,
        fit_tolerance: 1e-6,
    }
}

fn ensure_baseline() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let (code, graph) = load_fixture();
        let report = analyze_spectrum(&graph, &code, &make_opts(12001)).expect("spectrum");
        let bytes = to_canonical_json_bytes(&report).expect("json");
        let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("repro/phase11");
        fs::create_dir_all(&out_dir).expect("create dir");
        fs::write(out_dir.join("bench_spectrum.json"), bytes).expect("write baseline");
    });
}

fn bench_spectrum(c: &mut Criterion) {
    ensure_baseline();
    let (code, graph) = load_fixture();
    c.bench_function("spectrum_throughput", |b| {
        b.iter(|| {
            let _ = analyze_spectrum(&graph, &code, &make_opts(13001)).expect("spectrum");
        });
    });
}

criterion_group!(benches, bench_spectrum);
criterion_main!(benches);
