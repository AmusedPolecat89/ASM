use asm_code::defect::{fuse, is_irreducible, species_from_pattern, DefectKind};
use asm_code::{CSSCode, StateHandle};
use asm_core::{RunProvenance, SchemaVersion};

fn provenance() -> RunProvenance {
    RunProvenance {
        input_hash: "input".into(),
        graph_hash: "graph".into(),
        code_hash: String::new(),
        seed: 13,
        created_at: "2024-01-01T00:00:00Z".into(),
        tool_versions: Default::default(),
    }
}

fn build_code() -> CSSCode {
    CSSCode::new(
        4,
        vec![vec![0, 2], vec![1, 3]],
        vec![vec![0, 2], vec![1, 3]],
        SchemaVersion::new(1, 0, 0),
        provenance(),
    )
    .unwrap()
}

#[test]
fn defects_have_expected_species() {
    let code = build_code();
    let state = StateHandle::from_bits(vec![1, 1, 0, 0]).unwrap();
    let violations = code.violations_for_state(&state).unwrap();
    let defects = code.find_defects(&violations);

    let expected_x = species_from_pattern(asm_code::css::ConstraintKind::X, &[0]);
    let expected_z = species_from_pattern(asm_code::css::ConstraintKind::Z, &[0]);

    assert!(defects
        .iter()
        .any(|d| d.species == expected_x && d.kind == DefectKind::X));
    assert!(defects
        .iter()
        .any(|d| d.species == expected_z && d.kind == DefectKind::Z));
    assert!(defects.iter().all(is_irreducible));
}

#[test]
fn fusion_creates_mixed_defects() {
    let code = build_code();
    let state = StateHandle::from_bits(vec![1, 1, 0, 0]).unwrap();
    let violations = code.violations_for_state(&state).unwrap();
    let defects = code.find_defects(&violations);
    let x_defect = defects
        .iter()
        .find(|d| d.kind == DefectKind::X)
        .expect("x defect");
    let z_defect = defects
        .iter()
        .find(|d| d.kind == DefectKind::Z)
        .expect("z defect");

    let fused = fuse(x_defect, z_defect);
    assert_eq!(fused.kind, DefectKind::Mixed);
    assert!(!is_irreducible(&fused));
    assert_eq!(fused.support_size, 2);

    // Fused species should be stable and distinct from constituents.
    assert!(defects.iter().all(|d| d.species != fused.species));
    assert_eq!(x_defect.x_checks.len(), 1);
}
