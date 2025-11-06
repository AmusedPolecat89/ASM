use std::fs;
use std::path::PathBuf;
use std::sync::Once;

use asm_aut::AnalysisReport;
use asm_gauge::{analyze_gauge, to_canonical_json_bytes, GaugeOpts};
use asm_spec::{from_json_slice as spectrum_from_slice, SpectrumReport};
use criterion::{criterion_group, criterion_main, Criterion};

fn load_spectrum() -> SpectrumReport {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let path = base.join("fixtures/phase11/t1_seed0/spectrum_report.json");
    let bytes = fs::read(path).expect("read spectrum");
    spectrum_from_slice(&bytes).expect("decode spectrum")
}

fn load_analysis() -> AnalysisReport {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let path = base.join("fixtures/phase12/analysis/t1_seed0/analysis_report.json");
    let json = fs::read_to_string(path).expect("read analysis");
    serde_json::from_str(&json).expect("decode analysis")
}

fn ensure_baseline() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let spectrum = load_spectrum();
        let analysis = load_analysis();
        let opts = GaugeOpts {
            seed: 4242,
            ..GaugeOpts::default()
        };
        let report = analyze_gauge(&spectrum, &analysis, &spectrum.operators.info, &opts)
            .expect("gauge report");
        let bytes = to_canonical_json_bytes(&report).expect("json");
        let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let repro_dir = out_dir.join("repro/phase12");
        fs::create_dir_all(&repro_dir).expect("create repro dir");
        fs::write(repro_dir.join("bench_gauge.json"), bytes).expect("write baseline");
    });
}

fn bench_gauge(c: &mut Criterion) {
    ensure_baseline();
    let spectrum = load_spectrum();
    let analysis = load_analysis();
    c.bench_function("gauge_throughput", |b| {
        b.iter(|| {
            let opts = GaugeOpts {
                seed: 5252,
                ..GaugeOpts::default()
            };
            let _ = analyze_gauge(&spectrum, &analysis, &spectrum.operators.info, &opts)
                .expect("gauge report");
        });
    });
}

criterion_group!(benches, bench_gauge);
criterion_main!(benches);
