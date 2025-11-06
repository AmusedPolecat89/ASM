use asm_code::{CSSCode, StateHandle};
use asm_core::{AsmError, ConstraintProjector, RunProvenance, SchemaVersion};

fn provenance() -> RunProvenance {
    RunProvenance {
        input_hash: "input".into(),
        graph_hash: "graph".into(),
        code_hash: String::new(),
        seed: 11,
        created_at: "2024-01-01T00:00:00Z".into(),
        tool_versions: Default::default(),
    }
}

fn build_code() -> CSSCode {
    CSSCode::new(
        4,
        vec![vec![0, 1], vec![2, 3]],
        vec![vec![0, 1], vec![2, 3]],
        SchemaVersion::new(1, 0, 0),
        provenance(),
    )
    .unwrap()
}

#[test]
fn syndrome_matches_expected_indices() {
    let code = build_code();
    let state = StateHandle::from_bits(vec![1, 0, 1, 1]).unwrap();
    let violations = code.violations_for_state(&state).unwrap();
    assert_eq!(violations.x(), &[0]);
    assert_eq!(violations.z(), &[0]);

    let all = code.check_violations(&state).unwrap();
    assert_eq!(all.as_ref(), &[0, code.num_constraints_x()]);
}

#[test]
fn batch_syndrome_uses_shared_work() {
    let code = build_code();
    let state_a = StateHandle::from_bits(vec![0, 0, 0, 0]).unwrap();
    let state_b = StateHandle::from_bits(vec![1, 0, 1, 0]).unwrap();
    let batch = code
        .violations_for_states(&[&state_a, &state_b])
        .expect("batch evaluation");
    assert_eq!(batch.len(), 2);
    assert!(batch[0].x().is_empty());
    assert!(batch[0].z().is_empty());
    assert_eq!(batch[1].x(), &[0, 1]);
    assert_eq!(batch[1].z(), &[0, 1]);
}

#[test]
fn mismatched_state_length_errors() {
    let code = build_code();
    let bad_state = StateHandle::from_bits(vec![1, 0]).unwrap();
    let err = code.violations_for_state(&bad_state).unwrap_err();
    if let AsmError::Code(info) = err {
        assert_eq!(info.code, "state-length-mismatch");
        assert!(info.context.contains_key("state_len"));
    } else {
        panic!("unexpected error variant");
    }
}
