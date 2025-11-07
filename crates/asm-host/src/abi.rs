use std::os::raw::c_char;

use asm_core::errors::{AsmError, ErrorInfo};

pub const ASM_ABI_VERSION: u32 = 1;

/// Result returned by plugin entrypoints.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct AsmStatus {
    pub code: i32,
    pub message_len: usize,
}

impl AsmStatus {
    pub const OK: Self = Self {
        code: 0,
        message_len: 0,
    };

    pub fn is_ok(self) -> bool {
        self.code == 0
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Capability {
    Graph = 1 << 0,
    Code = 1 << 1,
    Spectrum = 1 << 2,
    Gauge = 1 << 3,
    Interact = 1 << 4,
    Rg = 1 << 5,
    Exp = 1 << 6,
}

impl Capability {
    pub fn flag(self) -> u32 {
        self as u32
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AbiString {
    pub ptr: *const c_char,
    pub len: usize,
}

impl AbiString {
    pub unsafe fn as_str<'a>(self) -> Result<&'a str, AsmError> {
        if self.ptr.is_null() {
            return Ok("");
        }
        let slice = std::slice::from_raw_parts(self.ptr as *const u8, self.len);
        std::str::from_utf8(slice).map_err(|err| {
            AsmError::Serde(ErrorInfo::new("asm_host.invalid_utf8", err.to_string()))
        })
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AsmPluginInfo {
    pub abi_version: u32,
    pub name: AbiString,
    pub version: AbiString,
    pub capabilities: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AsmPluginVTable {
    pub init: Option<extern "C" fn(*const u8, usize) -> AsmStatus>,
    pub graph_generate: Option<extern "C" fn(*const u8, usize, OutCallback) -> AsmStatus>,
    pub code_generate: Option<extern "C" fn(*const u8, usize, OutCallback) -> AsmStatus>,
    pub spectrum: Option<extern "C" fn(*const u8, usize, OutCallback) -> AsmStatus>,
    pub gauge: Option<extern "C" fn(*const u8, usize, OutCallback) -> AsmStatus>,
    pub interact: Option<extern "C" fn(*const u8, usize, OutCallback) -> AsmStatus>,
    pub shutdown: Option<extern "C" fn()>,
}

pub type OutCallback = extern "C" fn(*const u8, usize) -> AsmStatus;
