use std::time::{Duration, Instant};

use asm_core::errors::{AsmError, ErrorInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxCaps {
    pub cpu_time_seconds: u64,
    pub max_rss_mb: u64,
    pub tmpdir_mb: u64,
    pub wall_seconds: u64,
}

impl SandboxCaps {
    pub fn relaxed() -> Self {
        Self {
            cpu_time_seconds: 600,
            max_rss_mb: 4096,
            tmpdir_mb: 1024,
            wall_seconds: 900,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SandboxEvent {
    CpuSeconds(u64),
    MemoryMb(u64),
    TmpUsageMb(u64),
    WallSeconds(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SandboxDecision {
    Continue,
    Exceeded {
        resource: &'static str,
        limit: u64,
        observed: u64,
    },
}

#[derive(Debug)]
pub struct SandboxGuard {
    caps: SandboxCaps,
    start: Instant,
    last_decision: SandboxDecision,
}

impl SandboxGuard {
    pub fn new(caps: SandboxCaps) -> Self {
        Self {
            caps,
            start: Instant::now(),
            last_decision: SandboxDecision::Continue,
        }
    }

    pub fn caps(&self) -> SandboxCaps {
        self.caps
    }

    pub fn observe(&mut self, event: SandboxEvent) -> SandboxDecision {
        let decision = match event {
            SandboxEvent::CpuSeconds(value) if value > self.caps.cpu_time_seconds => {
                SandboxDecision::Exceeded {
                    resource: "cpu",
                    limit: self.caps.cpu_time_seconds,
                    observed: value,
                }
            }
            SandboxEvent::MemoryMb(value) if value > self.caps.max_rss_mb => {
                SandboxDecision::Exceeded {
                    resource: "memory",
                    limit: self.caps.max_rss_mb,
                    observed: value,
                }
            }
            SandboxEvent::TmpUsageMb(value) if value > self.caps.tmpdir_mb => {
                SandboxDecision::Exceeded {
                    resource: "tmp",
                    limit: self.caps.tmpdir_mb,
                    observed: value,
                }
            }
            SandboxEvent::WallSeconds(value) if value > self.caps.wall_seconds => {
                SandboxDecision::Exceeded {
                    resource: "wall",
                    limit: self.caps.wall_seconds,
                    observed: value,
                }
            }
            _ => SandboxDecision::Continue,
        };
        self.last_decision = decision.clone();
        decision
    }

    pub fn last_decision(&self) -> &SandboxDecision {
        &self.last_decision
    }

    pub fn ensure_within(&self) -> Result<(), AsmError> {
        match &self.last_decision {
            SandboxDecision::Continue => Ok(()),
            SandboxDecision::Exceeded {
                resource,
                limit,
                observed,
            } => Err(AsmError::Rng(ErrorInfo::new(
                "asm_host.sandbox_limit",
                format!("sandbox exceeded {resource} limit {limit} with observed {observed}"),
            ))),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}
