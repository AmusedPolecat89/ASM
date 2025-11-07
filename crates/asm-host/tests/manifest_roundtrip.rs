use asm_host::{from_json_slice, to_canonical_json_bytes, PluginManifest};

#[test]
fn manifest_roundtrip_is_stable() {
    let manifest = PluginManifest {
        name: "graph_smallworld".into(),
        version: "0.1.0".into(),
        abi_version: asm_host::ASM_ABI_VERSION,
        capabilities: vec!["graph".into()],
        minimum_workspace: Some("0.16".into()),
        license: "MIT".into(),
        description: Some("demo plugin".into()),
    };
    manifest.validate().expect("valid manifest");
    let toml = toml::to_string(&manifest).expect("serialize");
    let parsed: PluginManifest = toml::from_str(&toml).expect("parse");
    assert_eq!(manifest, parsed);
    let bytes = to_canonical_json_bytes(&manifest).expect("json");
    let parsed_json: PluginManifest = from_json_slice(&bytes).expect("roundtrip");
    assert_eq!(parsed_json, manifest);
}
