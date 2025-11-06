#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <plan.yaml> [--seed N]" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PLAN_PATH="$(cd "${REPO_ROOT}" && realpath "$1")"
shift

SEED=9001
while [[ $# -gt 0 ]]; do
  case "$1" in
    --seed)
      SEED="$2"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

PLAN_BASENAME="$(basename "$PLAN_PATH")"
PLAN_NAME="${PLAN_BASENAME%.*}"
OUT_DIR="${REPO_ROOT}/ablation/out/${PLAN_NAME}"
REGISTRY_PATH="${REPO_ROOT}/registry/asm.sqlite"
GOLDEN_PATH="${REPO_ROOT}/ablation/goldens/${PLAN_NAME}.gold.json"
DIFF_PATH="${OUT_DIR}/diff.json"
REPORT_PATH="${OUT_DIR}/ablation_report.json"

log() {
  echo "[ablation] $*"
}

ensure_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required tool: $1" >&2
    exit 1
  fi
}

for tool in cargo python3 jq diff; do
  ensure_tool "$tool"
done

rm -rf "${OUT_DIR}"
mkdir -p "${OUT_DIR}"

log "building asm-sim CLI"
cargo build --bin asm-sim >/dev/null

log "running ablation plan ${PLAN_NAME} with seed ${SEED}"
cargo run --quiet --bin asm-sim -- ablation \
  --plan "${PLAN_PATH}" \
  --seed "${SEED}" \
  --out "${OUT_DIR}" \
  --registry "${REGISTRY_PATH}"

if [[ ! -f "${REPORT_PATH}" ]]; then
  echo "expected report ${REPORT_PATH} was not created" >&2
  exit 1
fi

if [[ -f "${GOLDEN_PATH}" ]]; then
  log "comparing against golden ${GOLDEN_PATH}"
  python3 "${REPO_ROOT}/scripts/compare_to_golden.py" \
    --plan "${PLAN_PATH}" \
    --report "${REPORT_PATH}" \
    --golden "${GOLDEN_PATH}" \
    --diff "${DIFF_PATH}"
else
  log "no golden found for ${PLAN_NAME}; generating diff placeholder"
  python3 - "$DIFF_PATH" <<'PY'
import json
import pathlib
import sys
out = pathlib.Path(sys.argv[1])
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps({"status": "no_golden"}, indent=2) + "\n")
PY
fi

log "ablation complete → ${OUT_DIR}"
