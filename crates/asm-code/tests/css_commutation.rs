use asm_code::{css::ConstraintKind, CSSCode};
use asm_core::{AsmError, RunProvenance, SchemaVersion};

fn provenance() -> RunProvenance {
    RunProvenance {
        input_hash: "input".into(),
        graph_hash: "graph".into(),
        code_hash: String::new(),
        seed: 7,
        created_at: "2024-01-01T00:00:00Z".into(),
        tool_versions: Default::default(),
    }
}

#[test]
fn css_constraints_commute() {
    let code = CSSCode::new(
        4,
        vec![vec![0, 1], vec![2, 3]],
        vec![vec![0, 1], vec![2, 3]],
        SchemaVersion::new(1, 0, 0),
        provenance(),
    )
    .expect("valid CSS code");

    assert!(code.is_css_orthogonal());
    assert_eq!(code.num_constraints_x(), 2);
    assert_eq!(code.num_constraints_z(), 2);
    assert_eq!(code.rank_x(), 2);
    assert_eq!(code.rank_z(), 2);
    assert_eq!(code.canonical_hash().len(), 64);

    // ensure species lookup is deterministic for single-check defects
    let species_x = asm_code::defect::species_from_pattern(ConstraintKind::X, &[0]);
    let species_z = asm_code::defect::species_from_pattern(ConstraintKind::Z, &[0]);
    assert!(species_x.as_raw() != species_z.as_raw());
}

#[test]
fn css_orthogonality_failure() {
    let err = CSSCode::new(
        2,
        vec![vec![0]],
        vec![vec![0]],
        SchemaVersion::new(1, 0, 0),
        provenance(),
    )
    .expect_err("anticommuting checks should fail");

    match err {
        AsmError::Code(info) => {
            assert_eq!(info.code, "css-orthogonality-failed");
            assert!(info.context.contains_key("x_index"));
            assert!(info.context.contains_key("z_index"));
        }
        other => panic!("unexpected error variant: {:?}", other),
    }
}
