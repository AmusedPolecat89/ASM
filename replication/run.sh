#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUT_DIR="${SCRIPT_DIR}/out"
EXPECTED_DIR="${SCRIPT_DIR}/expected"
CONFIG_DIR="${SCRIPT_DIR}/configs"
SEED_DIR="${SCRIPT_DIR}/seeds"

log() {
  echo "[replication] $*"
}

ensure_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    log "required tool '$1' is not installed"
    exit 1
  fi
}

for tool in cargo jq python3 sha256sum diff; do
  ensure_tool "$tool"
done

cd "$REPO_ROOT"

log "preparing workspace"
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

log "building asm-sim binary"
cargo build --bin asm-sim >/dev/null

asm_sim() {
  cargo run --quiet --bin asm-sim -- "$@"
}

SEEDS=(0 1)
CONFIG_FILE="${CONFIG_DIR}/short.yaml"

for seed in "${SEEDS[@]}"; do
  run_dir="${OUT_DIR}/run_seed${seed}"
  seed_manifest="${SEED_DIR}/state_seed${seed}.json"

  log "running sampler for seed ${seed}"
  asm_sim mcmc --config "$CONFIG_FILE" --in "$seed_manifest" --out "$run_dir"

  log "running dispersion analysis for seed ${seed}"
  asm_sim analyze --input "$run_dir" --out "${run_dir}/analysis/dispersion"

  log "running symmetry scan for seed ${seed}"
  asm_sim analyze --input "$run_dir" --out "${run_dir}/analysis/symmetry" --symmetry-scan --laplacian-topk 8 --stabilizer-topk 8

done

log "executing single-step RG flow"
RG_DIR="${OUT_DIR}/rg"
asm_sim rg --input "${OUT_DIR}/run_seed0" --steps 1 --scale 2 --seed 11 --out "$RG_DIR"

log "extracting effective couplings"
EXTRACT_DIR="${OUT_DIR}/extract"
asm_sim extract --input "${RG_DIR}/step_000" --out "$EXTRACT_DIR" --yukawa 3 --seed 7 --residual-tolerance 1e-6

log "estimating dispersion gap"
GAPS_DIR="${OUT_DIR}/gaps"
mkdir -p "$GAPS_DIR"
asm_sim gaps --input "${OUT_DIR}/run_seed0" --method dispersion --tolerance 0.03 --out "${GAPS_DIR}/dispersion.json"

log "estimating spectral gap"
asm_sim gaps --input "${OUT_DIR}/run_seed0" --method spectral --out "${GAPS_DIR}/spectral.json"

log "assembling runbook"
REPORT_DIR="${OUT_DIR}/report_bundle"
asm_sim report \
  --inputs "${OUT_DIR}/run_seed0" \
  --inputs "${OUT_DIR}/run_seed1" \
  --inputs "$RG_DIR" \
  --inputs "$EXTRACT_DIR" \
  --inputs "$GAPS_DIR" \
  --out "$REPORT_DIR"

if [[ -f "${REPORT_DIR}/report.md" ]]; then
  mv -f "${REPORT_DIR}/report.md" "${OUT_DIR}/report.md"
fi

log "computing verification digests"
python3 <<'PY'
import csv
import json
import pathlib

base = pathlib.Path('replication/out')
runs = ['run_seed0', 'run_seed1']

# Aggregate dispersion common_c outputs
common = {}
for run in runs:
    path = base / run / 'analysis' / 'dispersion' / 'common_c.json'
    with path.open() as fh:
        common[run] = json.load(fh)
(base / 'common_c.json').write_text(json.dumps(common, indent=2, sort_keys=True) + '\n')

# Aggregate metrics statistics
metrics = {}
for run in runs:
    metrics_path = base / run / 'metrics.csv'
    with metrics_path.open() as fh:
        reader = csv.DictReader(fh)
        energies = []
        last_row = None
        for row in reader:
            energies.append(float(row['energy']))
            last_row = row
    metrics[run] = {
        'energy': {
            'min': min(energies),
            'max': max(energies),
            'mean': sum(energies) / len(energies),
        },
        'last_energy': float(last_row['energy']) if last_row else None,
        'samples': len(energies),
    }
(base / 'metrics_digest.json').write_text(json.dumps(metrics, indent=2, sort_keys=True) + '\n')
PY

jq -S . "${GAPS_DIR}/dispersion.json" > "${OUT_DIR}/gaps_dispersion.json"
jq -S . "${GAPS_DIR}/spectral.json" > "${OUT_DIR}/gaps_spectral.json"
jq -S . "${EXTRACT_DIR}/couplings.json" > "${OUT_DIR}/rg_couplings.json"

{
  sha256sum "${OUT_DIR}/run_seed0/end_state/graph.json"
  sha256sum "${OUT_DIR}/run_seed1/end_state/graph.json"
} | awk '{print ($2 ~ /run_seed0/ ? "run_seed0" : "run_seed1") " " $1}' > "${OUT_DIR}/graph_hashes.txt"

{
  sha256sum "${OUT_DIR}/run_seed0/end_state/code.json"
  sha256sum "${OUT_DIR}/run_seed1/end_state/code.json"
} | awk '{print ($2 ~ /run_seed0/ ? "run_seed0" : "run_seed1") " " $1}' > "${OUT_DIR}/code_hashes.txt"

check_json() {
  local actual="$1"
  local expected="$2"
  local label="$3"
  if [[ ! -f "$expected" ]]; then
    log "missing expected artifact: $expected"
    exit 1
  fi
  local actual_tmp
  local expected_tmp
  actual_tmp="$(mktemp)"
  expected_tmp="$(mktemp)"
  jq -S '.' "$actual" >"$actual_tmp"
  jq -S '.' "$expected" >"$expected_tmp"
  if ! diff -u "$expected_tmp" "$actual_tmp" > /dev/null; then
    log "verification failed for $label"
    diff -u "$expected_tmp" "$actual_tmp" || true
    rm -f "$actual_tmp" "$expected_tmp"
    exit 1
  fi
  rm -f "$actual_tmp" "$expected_tmp"
}

check_text() {
  local actual="$1"
  local expected="$2"
  local label="$3"
  if [[ ! -f "$expected" ]]; then
    log "missing expected artifact: $expected"
    exit 1
  fi
  if ! diff -u "$expected" "$actual" > /dev/null; then
    log "verification failed for $label"
    diff -u "$expected" "$actual" || true
    exit 1
  fi
}

log "verifying outputs"
check_text "${OUT_DIR}/graph_hashes.txt" "${EXPECTED_DIR}/graph_hashes.txt" "graph hashes"
check_text "${OUT_DIR}/code_hashes.txt" "${EXPECTED_DIR}/code_hashes.txt" "code hashes"
check_json "${OUT_DIR}/common_c.json" "${EXPECTED_DIR}/common_c.json" "common_c"
check_json "${OUT_DIR}/metrics_digest.json" "${EXPECTED_DIR}/metrics_digest.json" "metrics digest"
check_json "${OUT_DIR}/gaps_dispersion.json" "${EXPECTED_DIR}/gaps_dispersion.json" "dispersion gap"
check_json "${OUT_DIR}/gaps_spectral.json" "${EXPECTED_DIR}/gaps_spectral.json" "spectral gap"
check_json "${OUT_DIR}/rg_couplings.json" "${EXPECTED_DIR}/rg_couplings.json" "RG couplings"

if [[ ! -f "${OUT_DIR}/report.md" ]]; then
  log "expected report not found at ${OUT_DIR}/report.md"
  exit 1
fi

log "replication completed successfully"
log "report available at ${OUT_DIR}/report.md"
