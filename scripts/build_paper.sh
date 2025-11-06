#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PAPER_DIR="${REPO_ROOT}/paper"

FIGURES_DIR="${PAPER_DIR}/figures"
mkdir -p "${FIGURES_DIR}"

python3 "${SCRIPT_DIR}/make_figures.py" \
  --replication "${REPO_ROOT}/replication/out" \
  --fixtures "${REPO_ROOT}/fixtures/phase10" \
  --figures "${FIGURES_DIR}"

COMMIT="$(git -C "${REPO_ROOT}" rev-parse HEAD 2>/dev/null || echo unknown)"
BUILD_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

if ! command -v pandoc >/dev/null 2>&1; then
  echo "[paper] pandoc is required to build the preprint" >&2
  exit 1
fi

PANDOC_OPTS=(
  "${PAPER_DIR}/main.md"
  "--citeproc"
  "--bibliography" "${PAPER_DIR}/refs.bib"
  "--metadata" "commit=${COMMIT}"
  "--metadata" "build_date=${BUILD_DATE}"
  "-o" "${PAPER_DIR}/paper.pdf"
)

pandoc "${PANDOC_OPTS[@]}"
