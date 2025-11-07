use asm_host::{verify_abi_compat, AbiString, AsmPluginInfo, ASM_ABI_VERSION};

fn make_info(version: u32) -> AsmPluginInfo {
    AsmPluginInfo {
        abi_version: version,
        name: AbiString {
            ptr: std::ptr::null(),
            len: 0,
        },
        version: AbiString {
            ptr: std::ptr::null(),
            len: 0,
        },
        capabilities: 0,
    }
}

#[test]
fn rejects_mismatched_abi() {
    let info = make_info(ASM_ABI_VERSION + 1);
    let err = verify_abi_compat(&info).expect_err("expected mismatch");
    assert_eq!(err.info().code, "asm_host.abi_mismatch");
}

#[test]
fn accepts_matching_abi() {
    let info = make_info(ASM_ABI_VERSION);
    verify_abi_compat(&info).expect("compatible ABI");
}
