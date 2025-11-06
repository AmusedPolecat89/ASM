//! Structured error types shared across ASM crates.

use std::collections::BTreeMap;
use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Structured payload attached to every [`AsmError`] variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// Stable machine readable error code.
    pub code: String,
    /// Human readable diagnostic message.
    pub message: String,
    /// Contextual key value pairs (identifiers, sizes, etc.).
    #[serde(default)]
    pub context: BTreeMap<String, String>,
    /// Optional hint that may help the caller resolve the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl ErrorInfo {
    /// Creates a new error payload with the provided code and message.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            context: BTreeMap::new(),
            hint: None,
        }
    }

    /// Adds a context entry to the payload.
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Sets a human readable hint for remediation.
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// Canonical error type for the ASM engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[serde(tag = "family", content = "detail")]
pub enum AsmError {
    /// Hypergraph structural errors.
    #[error("graph error: {0}")]
    Graph(ErrorInfo),
    /// Constraint projector errors.
    #[error("code error: {0}")]
    Code(ErrorInfo),
    /// Renormalization group errors.
    #[error("rg error: {0}")]
    RG(ErrorInfo),
    /// Operator dictionary errors.
    #[error("dictionary error: {0}")]
    Dictionary(ErrorInfo),
    /// Randomness and seeding errors.
    #[error("rng error: {0}")]
    Rng(ErrorInfo),
    /// Serialization and schema errors.
    #[error("serde error: {0}")]
    Serde(ErrorInfo),
}

impl Display for ErrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (code: {})", self.message, self.code)?;
        if !self.context.is_empty() {
            write!(f, " | context: [")?;
            for (idx, (key, value)) in self.context.iter().enumerate() {
                if idx > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{key}={value}")?;
            }
            write!(f, "]")?;
        }
        if let Some(hint) = &self.hint {
            write!(f, " | hint: {hint}")?;
        }
        Ok(())
    }
}

impl AsmError {
    /// Returns a reference to the payload describing the error.
    pub fn info(&self) -> &ErrorInfo {
        match self {
            AsmError::Graph(info)
            | AsmError::Code(info)
            | AsmError::RG(info)
            | AsmError::Dictionary(info)
            | AsmError::Rng(info)
            | AsmError::Serde(info) => info,
        }
    }
}
