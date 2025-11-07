use std::fs;
use std::path::PathBuf;

use asm_gauge::from_json_slice as gauge_from_slice;
use asm_int::{
    interact_full, serde::to_canonical_json_bytes, FitOpts, KernelMode, KernelOpts, MeasureOpts,
    ParticipantSpec, PrepSpec,
};
use asm_spec::from_json_slice as spec_from_slice;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn workspace_root() -> PathBuf {
    let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    candidate.canonicalize().unwrap_or(candidate)
}

fn load_reports() -> (asm_spec::SpectrumReport, asm_gauge::GaugeReport) {
    let base = workspace_root();
    let spectrum_bytes = fs::read(base.join("fixtures/phase11/t1_seed0/spectrum_report.json"))
        .expect("spectrum fixture");
    let gauge_bytes =
        fs::read(base.join("fixtures/phase12/t1_seed0/gauge_report.json")).expect("gauge fixture");
    let spectrum = spec_from_slice(&spectrum_bytes).expect("decode spectrum");
    let gauge = gauge_from_slice(&gauge_bytes).expect("decode gauge");
    (spectrum, gauge)
}

fn make_prep_spec(spec: &asm_spec::SpectrumReport) -> PrepSpec {
    let mut participants = Vec::new();
    for (idx, mode) in spec.dispersion.modes.iter().take(2).enumerate() {
        let charge = if idx % 2 == 0 { 1.0 } else { -1.0 };
        participants.push(ParticipantSpec {
            mode_id: mode.mode_id,
            k: mode.omega,
            charge,
        });
    }
    assert!(
        participants.len() == 2,
        "fixture requires at least two modes"
    );
    let mut prep = PrepSpec::default();
    prep.participants = participants;
    prep.template = None;
    prep
}

fn interact_benchmark(c: &mut Criterion) {
    let (spectrum, gauge) = load_reports();
    let prep = make_prep_spec(&spectrum);
    let mut kernel = KernelOpts::default();
    kernel.mode = KernelMode::Fast;
    kernel.steps = 64;
    let measure = MeasureOpts::default();
    let fit = FitOpts::default();

    c.bench_function("interact/light", |b| {
        b.iter(|| {
            let _ = interact_full(
                black_box(&spectrum),
                black_box(&gauge),
                black_box(&prep),
                black_box(&kernel),
                black_box(&measure),
                black_box(&fit),
                101,
            )
            .expect("interaction");
        });
    });

    let (_, _, _, _, report) =
        interact_full(&spectrum, &gauge, &prep, &kernel, &measure, &fit, 101).expect("interaction");
    let bench_dir = workspace_root().join("repro/phase13");
    fs::create_dir_all(&bench_dir).expect("bench dir");
    fs::write(
        bench_dir.join("bench_interact.json"),
        to_canonical_json_bytes(&report).expect("serialize"),
    )
    .expect("write bench report");
}

criterion_group!(benches, interact_benchmark);
criterion_main!(benches);
