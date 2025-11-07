#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/run_full_suite.sh [options]

Options:
  --plan <path>         Landscape plan to execute (default: landscape/plans/full.yaml)
  --out <dir>           Root directory for reports (default: reports)
  --concurrency <N>     Parallelism for landscape runs (default: nproc)
  --skip-paper          Skip paper-pack and paper build steps
  --skip-web            Skip static site build
  --light               Use the medium plan and a reduced benchmark set
  -h, --help            Show this message
USAGE
}

PLAN_PATH="landscape/plans/full.yaml"
OUT_ROOT="reports"
CONCURRENCY="$(nproc)"
SKIP_PAPER=0
SKIP_WEB=0
SKIP_PAPER_FORCED=0
SKIP_WEB_FORCED=0
LIGHT=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --plan)
      [[ $# -ge 2 ]] || { echo "Missing argument for --plan" >&2; exit 1; }
      PLAN_PATH="$2"
      shift 2
      ;;
    --out)
      [[ $# -ge 2 ]] || { echo "Missing argument for --out" >&2; exit 1; }
      OUT_ROOT="$2"
      shift 2
      ;;
    --concurrency)
      [[ $# -ge 2 ]] || { echo "Missing argument for --concurrency" >&2; exit 1; }
      CONCURRENCY="$2"
      shift 2
      ;;
    --skip-paper)
      SKIP_PAPER=1
      SKIP_PAPER_FORCED=1
      shift
      ;;
    --skip-web)
      SKIP_WEB=1
      SKIP_WEB_FORCED=1
      shift
      ;;
    --light)
      LIGHT=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ "$LIGHT" -eq 1 ]]; then
  PLAN_PATH="landscape/plans/medium.yaml"
  if [[ "$SKIP_PAPER_FORCED" -eq 0 ]]; then
    SKIP_PAPER=1
  fi
  if [[ "$SKIP_WEB_FORCED" -eq 0 ]]; then
    SKIP_WEB=1
  fi
fi

if ! git_root=$(git rev-parse --show-toplevel 2>/dev/null); then
  echo "Error: script must be run within the git repository" >&2
  exit 1
fi
cd "$git_root"

if [[ ! -f Cargo.toml ]]; then
  echo "Error: Cargo.toml not found in repository root" >&2
  exit 1
fi

for bin in cargo python3 git; do
  if ! command -v "$bin" >/dev/null 2>&1; then
    echo "Required command '$bin' not found in PATH" >&2
    exit 1
  fi
done

if ! command -v asm-sim >/dev/null 2>&1; then
  for candidate in "$git_root/target/release/asm-sim" "$git_root/target/debug/asm-sim"; do
    if [[ -x "$candidate" ]]; then
      export PATH="$(dirname "$candidate"):$PATH"
      break
    fi
  done
fi

if ! command -v asm-sim >/dev/null 2>&1; then
  echo "asm-sim binary not found; building via cargo build --bin asm-sim"
  cargo build --quiet --bin asm-sim
  export PATH="$git_root/target/debug:$PATH"
  if ! command -v asm-sim >/dev/null 2>&1; then
    echo "Failed to locate asm-sim binary after build" >&2
    exit 1
  fi
fi

RUN_TS="$(date -u +"%Y%m%d_%H%M%S")"
ABS_OUT=$(python3 - "$OUT_ROOT" <<'PY'
import os, sys
print(os.path.abspath(sys.argv[1]))
PY
)
RUN_DIR="$ABS_OUT/run_${RUN_TS}"
mkdir -p "$RUN_DIR"

LOG_FILE="$RUN_DIR/log.txt"
touch "$LOG_FILE"
exec > >(tee -a "$LOG_FILE") 2>&1

BENCH_DIR="$RUN_DIR/benches"
ARTIFACT_DIR="$RUN_DIR/artifacts"
FIG_DIR="$RUN_DIR/figures"
mkdir -p "$BENCH_DIR" "$ARTIFACT_DIR" "$FIG_DIR"

PLAN_ABS=$(python3 - "$PLAN_PATH" <<'PY'
import os, sys
print(os.path.abspath(sys.argv[1]))
PY
)

if [[ ! -f "$PLAN_ABS" ]]; then
  echo "Landscape plan not found: $PLAN_ABS" >&2
  exit 1
fi

cp "$PLAN_ABS" "$RUN_DIR/plan_used.yaml"

LANDSCAPE_OUT="runs/landscape/full"

cat <<EOF_LOG
============================================
ASM full suite run started: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
Run directory: $RUN_DIR
Plan used: $PLAN_ABS
Landscape output: $LANDSCAPE_OUT
Concurrency: $CONCURRENCY
Light mode: $LIGHT
Skip paper: $SKIP_PAPER
Skip web: $SKIP_WEB
============================================
EOF_LOG

echo "Git HEAD: $(git rev-parse HEAD)"
echo "Git status:" && git status --porcelain=v1

start_time=$(date +%s)

section() {
  local name="$1"
  echo
  echo "---------- $name ----------"
}

run_cmd() {
  local desc="$1"
  shift
  local cmd_start=$(date +%s)
  echo "Starting: $desc"
  "$@"
  local status=$?
  local cmd_end=$(date +%s)
  if [[ $status -ne 0 ]]; then
    echo "FAILED ($desc) after $((cmd_end - cmd_start))s" >&2
    exit $status
  fi
  echo "Completed: $desc in $((cmd_end - cmd_start))s"
}

section "Preflight: environment capture"
ENV_JSON="$RUN_DIR/env.json"
ASM_ENV_JSON="$ENV_JSON" ASM_PLAN_PATH="$PLAN_ABS" ASM_LIGHT_MODE="$LIGHT" ASM_CONCURRENCY="$CONCURRENCY" python3 - <<'PY'
import datetime
import json
import os
import platform
import subprocess
from pathlib import Path

def run(cmd):
    try:
        return subprocess.check_output(cmd, text=True).strip()
    except Exception:
        return None

def mem_total():
    try:
        with open("/proc/meminfo", "r", encoding="utf8") as handle:
            for line in handle:
                if line.startswith("MemTotal:"):
                    parts = line.split()
                    if len(parts) >= 2:
                        return int(parts[1]) * 1024
    except Exception:
        return None
    return None

def cpu_model():
    try:
        with open("/proc/cpuinfo", "r", encoding="utf8") as handle:
            for line in handle:
                if line.lower().startswith("model name"):
                    return line.split(":", 1)[1].strip()
    except Exception:
        return platform.processor() or None
    return None

def gpu_info():
    info = {}
    cmd = ["nvidia-smi", "--query-gpu=name,driver_version", "--format=csv,noheader"]
    try:
        output = subprocess.check_output(cmd, text=True, stderr=subprocess.DEVNULL)
        lines = [line.strip() for line in output.splitlines() if line.strip()]
        if lines:
            info["devices"] = lines
    except Exception:
        return None
    return info if info else None

now = datetime.datetime.utcnow().replace(microsecond=0)
git_head = run(["git", "rev-parse", "HEAD"])
git_tag = run(["git", "describe", "--tags", "--always"])
git_status = run(["git", "status", "--porcelain=v1"])

env = {
    "timestamp_utc": now.isoformat() + "Z",
    "git": {
        "head": git_head,
        "describe": git_tag,
        "dirty": bool(git_status.strip()) if git_status is not None else None,
    },
    "rustc_version": run(["rustc", "--version"]),
    "cargo_version": run(["cargo", "--version"]),
    "asm_sim_version": run(["asm-sim", "version", "--long"]),
    "cpu": {
        "model": cpu_model(),
        "cores": os.cpu_count(),
    },
    "memory_bytes": mem_total(),
    "gpu": gpu_info(),
    "os": {
        "platform": platform.platform(),
        "kernel": run(["uname", "-r"]),
    },
    "container": "docker" if Path("/.dockerenv").exists() else None,
    "plan_path": os.path.abspath(os.environ.get("ASM_PLAN_PATH", "")) or None,
    "light_mode": os.environ.get("ASM_LIGHT_MODE", "0") == "1",
    "concurrency": os.environ.get("ASM_CONCURRENCY"),
}
with open(os.environ["ASM_ENV_JSON"], "w", encoding="utf8") as handle:
    json.dump(env, handle, indent=2, sort_keys=True)
PY

section "Build"
run_cmd "cargo fetch" cargo fetch
run_cmd "cargo build --workspace --release" cargo build --workspace --release
if [[ "$LIGHT" -eq 0 ]]; then
  run_cmd "cargo bench warmup" cargo bench --workspace --no-run
else
  echo "Skipping Criterion warm-up in light mode"
fi

section "Tests"
TEST_PASSED=0
run_cmd "cargo test --workspace --release" cargo test --workspace --release -- --nocapture
TEST_PASSED=1

section "Replication"
REPL_PASSED=0
run_cmd "replication suite" ./replication/run.sh
REPL_PASSED=1

section "Benchmarks"
if [[ "$LIGHT" -eq 1 ]]; then
  BENCH_COMMANDS=(
    "cargo bench -p asm-spec --bench spectrum_throughput"
    "cargo bench -p asm-gauge --bench gauge_throughput"
    "cargo bench -p asm-int --bench interact_throughput"
  )
else
  BENCH_COMMANDS=(
    "cargo bench -p asm-spec --bench spectrum_throughput"
    "cargo bench -p asm-gauge --bench gauge_throughput"
    "cargo bench -p asm-int --bench interact_throughput"
    "cargo bench -p asm-land --bench landscape_throughput"
    "cargo bench -p asm-host --bench plugin_overhead"
    "cargo bench -p asm-web --bench web_throughput"
    "cargo bench -p asm-thy --bench thy_assertions_throughput"
  )
fi
for cmd in "${BENCH_COMMANDS[@]}"; do
  run_cmd "$cmd" bash -lc "$cmd"
done

if compgen -G "repro/**/*.json" >/dev/null 2>&1; then
  while IFS= read -r -d '' file; do
    dest="$BENCH_DIR/$file"
    mkdir -p "$(dirname "$dest")"
    cp "$file" "$dest"
  done < <(find repro -type f -name '*.json' -print0)
fi
if [[ -d target/criterion ]]; then
  cp -R target/criterion "$BENCH_DIR/"
fi

section "Heavy landscape pipeline"
run_cmd "asm-sim landscape run" asm-sim landscape run --plan "$PLAN_ABS" --out "$LANDSCAPE_OUT" --resume --concurrency "$CONCURRENCY"
run_cmd "asm-sim landscape summarize" asm-sim landscape summarize --root "$LANDSCAPE_OUT" --filters landscape/filters/default.yaml --out "$LANDSCAPE_OUT/summary"
run_cmd "asm-sim landscape atlas" asm-sim landscape atlas --root "$LANDSCAPE_OUT" --out "$LANDSCAPE_OUT/atlas"
run_cmd "asm-sim assert-batch" asm-sim assert-batch --root "$LANDSCAPE_OUT" --policy configs/phase15/policy_default.yaml --out "$LANDSCAPE_OUT/assertions"

if [[ "$SKIP_PAPER" -eq 0 ]]; then
  section "Paper bundle"
  run_cmd "asm-sim paper-pack" asm-sim paper-pack --roots "$LANDSCAPE_OUT" fixtures/phase11 fixtures/phase12 analysis --plan configs/phase15/bundle.yaml --out paper/inputs
  if [[ -x scripts/build_paper.sh ]]; then
    run_cmd "build paper" bash -lc "scripts/build_paper.sh"
  else
    echo "Skipping paper build (scripts/build_paper.sh missing or not executable)"
  fi
else
  echo "Skipping paper bundle"
fi

if [[ "$SKIP_WEB" -eq 0 ]]; then
  section "Static site build"
  if [[ -f registry/asm.sqlite ]]; then
    run_cmd "asm-sim web build" asm-sim web build --registry registry/asm.sqlite --config configs/phase16/site.yaml --out site/dist
  else
    echo "Registry database not found; skipping site build"
  fi
else
  echo "Skipping web build"
fi

section "Collate artefacts"
copy_file() {
  local src="$1"
  local dest="$ARTIFACT_DIR/$1"
  if [[ -e "$src" ]]; then
    mkdir -p "$(dirname "$dest")"
    cp "$src" "$dest"
  else
    echo "Warning: missing artifact $src" >&2
  fi
}

copy_tree_if_exists() {
  local src="$1"
  if [[ -d "$src" ]]; then
    while IFS= read -r -d '' file; do
      local dest="$ARTIFACT_DIR/$file"
      mkdir -p "$(dirname "$dest")"
      cp "$file" "$dest"
    done < <(find "$src" -type f -print0)
  fi
}

copy_file "$LANDSCAPE_OUT/summary/SummaryReport.json"
copy_file "$LANDSCAPE_OUT/atlas/manifest.json"
copy_file "$LANDSCAPE_OUT/assertions/index.json"
if [[ -d "$LANDSCAPE_OUT/assertions" ]]; then
  while IFS= read -r -d '' report; do
    dest="$ARTIFACT_DIR/$report"
    mkdir -p "$(dirname "$dest")"
    cp "$report" "$dest"
  done < <(find "$LANDSCAPE_OUT/assertions" -type f -name 'assert_report.json' -print0)
fi

if compgen -G "$LANDSCAPE_OUT/assertions/*.json" >/dev/null 2>&1; then
  :
fi

if compgen -G "repro/**/*.json" >/dev/null 2>&1; then
  while IFS= read -r -d '' file; do
    dest="$ARTIFACT_DIR/$file"
    mkdir -p "$(dirname "$dest")"
    cp "$file" "$dest"
  done < <(find repro -type f -name '*.json' -print0)
fi

if [[ -d replication/expected ]]; then
  copy_tree_if_exists "replication/expected"
fi

if [[ -f paper/build/main.pdf ]]; then
  copy_file "paper/build/main.pdf"
fi
if [[ -d paper/figures ]]; then
  copy_tree_if_exists "paper/figures"
fi
if [[ -d site/dist && "$SKIP_WEB" -eq 0 ]]; then
  copy_tree_if_exists "site/dist"
fi

STATUS_JSON="$ARTIFACT_DIR/status.json"
ASM_STATUS_JSON="$STATUS_JSON" ASM_TEST_STATUS="$TEST_PASSED" ASM_REPL_STATUS="$REPL_PASSED" python3 - <<'PY'
import json
import os
status = {
    "tests_passed": os.environ["ASM_TEST_STATUS"] == "1",
    "replication_passed": os.environ["ASM_REPL_STATUS"] == "1",
}
with open(os.environ["ASM_STATUS_JSON"], "w", encoding="utf8") as handle:
    json.dump(status, handle, indent=2, sort_keys=True)
PY

section "Collect results"
python3 scripts/collect_results.py --in "$ARTIFACT_DIR" --out "$RUN_DIR"

ROOT_REPORT="report.md"
if [[ -f "$RUN_DIR/report.md" ]]; then
  sed "s#](figures/#](reports/run_${RUN_TS}/figures/#g" "$RUN_DIR/report.md" > "$ROOT_REPORT"
fi

total_end=$(date +%s)
echo
echo "Run completed successfully in $((total_end - start_time))s"
echo "Report available at $RUN_DIR/report.md and $(pwd)/report.md"
