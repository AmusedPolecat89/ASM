use std::fmt;
use std::sync::Arc;

use asm_core::{AsmError, ConstraintState, ErrorInfo};

const STATE_TAG: u64 = 0x53544154455f4249; // "STATE_BI"

/// Deterministic opaque state handle storing binary assignments.
#[derive(Clone)]
pub struct StateHandle {
    tag: u64,
    bits: Arc<[u8]>,
}

impl StateHandle {
    /// Creates a new state handle from raw bits.
    pub fn from_bits(bits: impl Into<Vec<u8>>) -> Result<Self, AsmError> {
        let bits_vec: Vec<u8> = bits.into();
        if bits_vec.iter().any(|&b| b > 1) {
            let info = ErrorInfo::new("invalid-state-bit", "state bits must be 0 or 1");
            return Err(AsmError::Code(info));
        }
        Ok(Self {
            tag: STATE_TAG,
            bits: Arc::from(bits_vec),
        })
    }

    /// Returns the bits stored in the state handle.
    pub fn bits(&self) -> &[u8] {
        &self.bits
    }
}

impl fmt::Debug for StateHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateHandle")
            .field("len", &self.bits.len())
            .field("hash", &self.checksum())
            .finish()
    }
}

impl StateHandle {
    fn checksum(&self) -> u64 {
        self.bits
            .iter()
            .fold(0u64, |acc, bit| (acc << 1) ^ u64::from(*bit))
    }
}

/// Extracts the raw bit slice from a dynamic constraint state.
pub fn view_bits(state: &dyn ConstraintState) -> Result<&[u8], AsmError> {
    let data = state as *const dyn ConstraintState as *const StateHandle;
    unsafe {
        if data.is_null() {
            let info = ErrorInfo::new("null-state-handle", "constraint state pointer was null");
            return Err(AsmError::Code(info));
        }
        if (*data).tag != STATE_TAG {
            let info = ErrorInfo::new(
                "unknown-state-handle",
                "constraint state is not managed by asm-code",
            );
            return Err(AsmError::Code(info));
        }
        Ok((*data).bits())
    }
}
