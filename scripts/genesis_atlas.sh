#!/usr/bin/env bash
set -euo pipefail

# Orchestrates "The Genesis Atlas" multi-stage simulation pipeline.
# Stages (functions) can be invoked individually or run end-to-end via the
# `all` target.  Each stage delegates to `asm-sim` commands as documented in the
# repository README.  This script only wires the plumbing together; callers are
# expected to provision compute resources and data storage before launching a
# full Genesis Atlas survey.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ROOT="${ROOT_DIR}/runs/genesis_atlas"
PLAN_FILE="${ROOT_DIR}/landscape/plans/genesis_atlas_plan.yaml"
CANDIDATES_FILE="${RUN_ROOT}/stage1/fertile_candidates.json"
ASM_SIM_BIN=${ASM_SIM_BIN:-asm-sim}

mkdir -p "${RUN_ROOT}/stage1" "${RUN_ROOT}/stage2" "${RUN_ROOT}/stage3" "${RUN_ROOT}/stage4"

function stage1_plan() {
  local plan_out="${RUN_ROOT}/stage1/huge_survey.plan.yaml"
  local filter_src="${ROOT_DIR}/landscape/filters/default.yaml"
  local filter_dst_dir="${RUN_ROOT}/filters"
  cp "${PLAN_FILE}" "${plan_out}"
  mkdir -p "${filter_dst_dir}"
  cp "${filter_src}" "${filter_dst_dir}/default.yaml"
  echo "[Stage 1] Plan copied to ${plan_out}"
}

function stage1_run() {
  local plan_in="${RUN_ROOT}/stage1/huge_survey.plan.yaml"
  local run_dir="${RUN_ROOT}/stage1/raw"
  test -f "${plan_in}" || { echo "Stage 1 plan missing: ${plan_in}"; exit 1; }
  mkdir -p "${run_dir}"
  ${ASM_SIM_BIN} landscape run --plan "${plan_in}" --out "${run_dir}"
  scripts/genesis_atlas_pipeline.py filter \
    --report "${run_dir}/landscape_report.json" \
    --plan "${plan_in}" \
    --stage1-root "${run_dir}" \
    --out "${CANDIDATES_FILE}" \
    --min-gap 0.05 \
    --max-energy 0.0
  echo "[Stage 1] Fertile candidates recorded at ${CANDIDATES_FILE}"
}

function stage2_analyze() {
  test -f "${CANDIDATES_FILE}" || { echo "No candidates file at ${CANDIDATES_FILE}"; exit 1; }
  scripts/genesis_atlas_pipeline.py stage2 \
    --candidates "${CANDIDATES_FILE}" \
    --out "${RUN_ROOT}/stage2"
}

function stage3_interactions() {
  test -f "${CANDIDATES_FILE}" || { echo "No candidates file at ${CANDIDATES_FILE}"; exit 1; }
  scripts/genesis_atlas_pipeline.py stage3 \
    --candidates "${CANDIDATES_FILE}" \
    --out "${RUN_ROOT}/stage3"
}

function stage4_running() {
  local summary_path="${RUN_ROOT}/stage3/field_theory_summary.json"
  test -f "${summary_path}" || { echo "Missing field theory summary at ${summary_path}"; exit 1; }
  scripts/genesis_atlas_pipeline.py stage4 \
    --field-summary "${summary_path}" \
    --out "${RUN_ROOT}/stage4"
}

function stage2() {
  stage2_analyze
}

function stage3() {
  stage3_interactions
}

function stage4() {
  stage4_running
}

function all() {
  stage1_plan
  stage1_run
  stage2_analyze
  stage3_interactions
  stage4_running
}

case "${1:-help}" in
  stage1-plan) stage1_plan ;;
  stage1-run) stage1_run ;;
  stage2) stage2 ;;
  stage3) stage3 ;;
  stage4) stage4 ;;
  all) all ;;
  *)
    cat <<USAGE
Usage: ${0##*/} <target>
Targets:
  stage1-plan   Copy the curated landscape plan into the run directory.
  stage1-run    Execute the Primordial Census and record fertile candidates.
  stage2        Run the First Light pipeline (deep spectra + gauge reconstruction).
  stage3        Measure couplings via interaction experiments.
  stage4        Perform RG analysis and measure running couplings.
  all           Execute every stage sequentially.

Environment:
  ASM_SIM_BIN   Override the asm-sim binary (defaults to 'asm-sim').
USAGE
    ;;
esac
