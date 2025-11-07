use asm_host::{SandboxCaps, SandboxEvent, SandboxGuard};

#[test]
fn detects_cpu_violation() {
    let caps = SandboxCaps {
        cpu_time_seconds: 2,
        max_rss_mb: 512,
        tmpdir_mb: 128,
        wall_seconds: 5,
    };
    let mut guard = SandboxGuard::new(caps);
    let decision = guard.observe(SandboxEvent::CpuSeconds(3));
    match decision {
        asm_host::SandboxDecision::Exceeded { resource, .. } => assert_eq!(resource, "cpu"),
        other => panic!("unexpected decision: {other:?}"),
    }
    assert!(guard.ensure_within().is_err());
}

#[test]
fn stays_within_limits() {
    let caps = SandboxCaps {
        cpu_time_seconds: 5,
        max_rss_mb: 512,
        tmpdir_mb: 128,
        wall_seconds: 5,
    };
    let mut guard = SandboxGuard::new(caps);
    assert!(matches!(
        guard.observe(SandboxEvent::CpuSeconds(1)),
        asm_host::SandboxDecision::Continue
    ));
    assert!(guard.ensure_within().is_ok());
}
