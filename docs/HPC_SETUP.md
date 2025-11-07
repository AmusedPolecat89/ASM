# High-Performance Setup & One-Shot Heavy Run

This guide explains how to provision a fresh machine, install the required toolchains, and execute the Phase 17 heavy runner. It targets large bare-metal or cloud instances that will execute the complete landscape and assertion pipelines in one go.

## Hardware & Operating System Requirements

- **CPU:** 16 physical cores (or more) with x86_64 or aarch64 support.
- **Memory:** ≥ 64 GB RAM to accommodate concurrent landscape jobs and caching.
- **Disk:** ≥ 200 GB of fast local storage for builds, intermediate runs, and artefacts.
- **GPU (optional):** NVIDIA GPU with recent drivers if you plan to accelerate plugin workloads; the runner operates without a GPU.
- **OS:** 64-bit Linux distribution (Ubuntu 22.04 LTS or comparable) with `systemd` and bash.

## Required System Packages

Install the base toolchain and headers:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev git python3 python3-pip

# Install Python plotting dependencies required by collect_results.py
python3 -m pip install --user matplotlib numpy
```

## Rust Toolchain

Install Rust via rustup and ensure the stable channel is default:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup default stable
rustc --version
cargo --version
```

## Repository Bootstrap

Clone the repository and pre-fetch dependencies so the heavy run does not spend time downloading crates:

```bash
git clone <your-repo-url>
cd <repo-directory>
cargo fetch
```

If you rely on community plugins or registries, install them before launching the suite.

## Running the Full Suite

The one-button entry point is [`scripts/run_full_suite.sh`](../scripts/run_full_suite.sh). It orchestrates build, tests, benches, heavy pipelines, and the aggregated report.

```bash
bash scripts/run_full_suite.sh \
  --plan landscape/plans/full.yaml \
  --out reports \
  --concurrency $(nproc)
```

### Optional Flags

- `--light` executes the medium landscape plan and skips the optional paper/site steps.
- `--skip-paper` or `--skip-web` disable the corresponding publishing stages.
- `--plan` chooses a custom landscape plan while retaining the same reporting layout.

## Outputs & Artefact Layout

After a successful run you will find:

- `report.md` at the repository root summarising the entire execution.
- `reports/run_<UTCYYYYMMDD_HHMMSS>/report.md` (identical content) with embedded relative figure paths.
- `reports/run_<…>/env.json` capturing hardware, toolchain, and git provenance.
- `reports/run_<…>/log.txt` containing the full, timestamped command log.
- `reports/run_<…>/figures/` with PNG/SVG charts (benchmarks, landscape stats, assertions).
- `reports/run_<…>/benches/` with copied Criterion summaries and `repro/phase*/bench_*.json` snapshots.
- `reports/run_<…>/artifacts/` aggregating SummaryReports, assertion outputs, paper/site bundles, and replication goldens.

You can re-run the aggregation step in isolation via:

```bash
python3 scripts/collect_results.py \
  --in reports/run_<…>/artifacts \
  --out reports/run_<…>
```

## Troubleshooting

- **Out-of-memory / thrashing:** Reduce concurrency with `--concurrency`, or switch to `--light` which executes the medium landscape plan and trims the benchmark set.
- **Slow Criterion benches:** Ensure the build uses `--release` (handled automatically) and that you have enough CPU headroom.
- **Assertion failures:** Inspect the failing checks under `runs/landscape/full/assertions/`. Adjust tolerances via `configs/phase15/policy_default.yaml` only after confirming the underlying physics output is sound.
- **Missing optional artefacts:** If you do not need paper or site generation, pass `--skip-paper` and/or `--skip-web` to avoid warnings about absent outputs.

With a clean workspace, rerunning the script is idempotent: the generated report will only differ by the UTC timestamp, and figures remain byte-identical as long as the upstream artefacts do not change.
