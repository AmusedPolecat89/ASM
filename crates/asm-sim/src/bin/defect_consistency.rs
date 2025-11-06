use std::error::Error;
use std::fs;
use std::path::Path;

use asm_code::defect::Defect;
use asm_code::serde as code_serde;
use asm_code::{state::StateHandle, CSSCode};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct StateFixtures {
    states: Vec<Vec<u8>>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let code_path = args
        .next()
        .ok_or("usage: defect_consistency <code.json> <states.json> <out.csv>")?;
    let states_path = args
        .next()
        .ok_or("usage: defect_consistency <code.json> <states.json> <out.csv>")?;
    let out_path = args
        .next()
        .ok_or("usage: defect_consistency <code.json> <states.json> <out.csv>")?;

    let code_json = fs::read_to_string(&code_path)?;
    let code = code_serde::from_json(&code_json)?;
    let states_json = fs::read_to_string(&states_path)?;
    let fixtures: StateFixtures = serde_json::from_str(&states_json)?;

    let mut rows = Vec::new();
    evaluate_states("pass0", &code, &fixtures.states, &mut rows)?;

    let reloaded = reload_code(&code)?;
    evaluate_states("pass1", &reloaded, &fixtures.states, &mut rows)?;

    if let Some(parent) = Path::new(&out_path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = csv::Writer::from_path(out_path)?;
    file.write_record(["state_id", "pass", "defect_hashes", "species_ids"])?;
    for row in rows {
        file.write_record(row)?;
    }
    file.flush()?;

    Ok(())
}

fn evaluate_states(
    pass_label: &str,
    code: &CSSCode,
    states: &[Vec<u8>],
    rows: &mut Vec<[String; 4]>,
) -> Result<(), Box<dyn Error>> {
    for (idx, bits) in states.iter().enumerate() {
        let state = StateHandle::from_bits(bits.clone())?;
        let violations = code.violations_for_state(&state)?;
        let defects = code.find_defects(&violations);
        let summary = summarize_defects(&defects);
        let species = summarise_species(&defects);
        rows.push([idx.to_string(), pass_label.to_string(), summary, species]);
    }
    Ok(())
}

fn summarise_species(defects: &[Defect]) -> String {
    let mut species: Vec<String> = defects
        .iter()
        .map(|defect| format!("{:#x}", defect.species.as_raw()))
        .collect();
    species.sort();
    species.join(";")
}

fn summarize_defects(defects: &[Defect]) -> String {
    let mut entries: Vec<String> = defects
        .iter()
        .map(|defect| {
            let x = defect
                .x_checks
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join("-");
            let z = defect
                .z_checks
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join("-");
            format!("{}|{}|{}", defect.kind as u8, x, z)
        })
        .collect();
    entries.sort();
    entries.join(";")
}

fn reload_code(code: &CSSCode) -> Result<CSSCode, Box<dyn Error>> {
    let json = code_serde::to_json(code)?;
    let restored = code_serde::from_json(&json)?;
    Ok(restored)
}
