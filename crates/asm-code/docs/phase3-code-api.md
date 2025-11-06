# Phase 3 – Constraint Projector & Dispersion API

This document specifies the public API provided by the `asm-code` crate and the
behavioural contracts required for Phase 3 of the ASM engine. The crate exports
`CSSCode`, an implementation of `asm_core::ConstraintProjector`, along with
supporting types for syndromes, defects, dispersion measurements, and
serialization.

## Public types and functions

### `CSSCode`

| Method | Purpose | Inputs | Outputs | Preconditions | Postconditions | Complexity | Errors |
| ------ | ------- | ------ | ------- | ------------- | -------------- | ---------- | ------ |
| `CSSCode::new` | Construct a code from sparse X/Z checks | `num_variables`, vectors of check indices, `SchemaVersion`, `RunProvenance` | `CSSCode` | Variable indices within range; no duplicate checks | Code is orthogonal and normalized; adjacency caches populated | O(n · w) where *w* is average check weight | `AsmError::Code` on invalid indices or orthogonality failure |
| `num_variables` | Number of physical variables | – | `usize` | – | – | O(1) | – |
| `num_constraints_x/z` | Number of X/Z stabilizers | – | `usize` | – | – | O(1) | – |
| `rank_x/z` | Rank of X/Z constraint sets | – | `usize` | Code constructed successfully | Pre-computed ranks returned | O(1) | – |
| `violations_for_state` | Compute violations for a single state | `&dyn ConstraintState` | `ViolationSet` | State length equals `num_variables` | Stable ordering of indices | O(m) with *m* the total touched checks | `AsmError::Code` on mismatched state length or foreign handle |
| `violations_for_states` | Batched violations | Slice of state handles | Vec of `ViolationSet` | Same as single-call per element | Batch order matches input order | O(k · m) amortized; caches reused | Propagates single state errors |
| `find_defects` | Extract irreducible defects | `ViolationSet` | Vec<`Defect`> | Violations in range | Deterministic ordering by species | O(v log v) | – |
| `species` | Retrieve species ID | `&Defect` | `SpeciesId` | – | – | O(1) | – |
| `canonical_hash` | Canonical structural hash | – | `String` | – | Hash covers version, structure | O(n · w) (cached inputs) | – |

Additional helpers include adjacency accessors (`x_adjacency`, `z_adjacency`)
and `species_support` for provenance-aware tooling.

### Syndrome helpers

`syndrome::compute_violations` computes violated X/Z constraint indices for a
bit-valued state. It requires exact length matching and runs in time linear in
the number of touched variables per check. The returned `ViolationSet` exposes
borrowed slices for X and Z indices, guaranteeing deterministic ordering.

### Defect utilities

* `ViolationSet` – grouped violation indices with stable slices.
* `Defect` – normalized defect descriptor with kind (`X`, `Z`, `Mixed`), support
  size, and deterministic species ID.
* `is_irreducible` – reports `true` for single-check supports.
* `fuse` – unions two defects (set semantics) producing a mixed defect when both
  sectors are involved.
* `species_from_pattern` – derives a `SpeciesId` from a normalized constraint
  pattern using SipHash with fixed keys.

### Dispersion probe

`dispersion::estimate_dispersion` measures deterministic proxy group velocity
curves for supplied species:

* Inputs: a `CSSCode`, a `Hypergraph`, a slice of `SpeciesId`, and
  `DispersionOptions { steps, tolerance }`.
* Behaviour: for each species, a velocity curve is produced using the cached
  species support size and degree bounds from the graph. The final velocity is
  averaged to derive a common limiting speed `c`.
* Output: `DispersionReport { per_species, common_c, residuals, diagnostics }`.
  Diagnostics report the sample count, a deterministic pseudo-confidence level,
  and the sum-of-squares residual error.
* Errors: `AsmError::Code` for empty `steps` vectors; `Hypergraph::degree_bounds`
  failures propagate as warnings by falling back to unknown bounds.

### Logical algebra summary

`analyze::logical_summary` populates `LogicalAlgebraSummary` with:

* `num_logical = num_variables - rank_x - rank_z` (saturating at zero).
* Stable labels `logical-<index>`.
* Metadata entries for `rank_x`, `rank_z`, and `num_variables`.

Errors arise only if the underlying code is non-orthogonal (defensive guard).

### Serialization

`serde::{to_json, from_json, to_bytes, from_bytes}` serialize the code into an
explicit schema containing:

* `schema_version` – copied verbatim from construction.
* `provenance` – cloned with `code_hash` populated if absent.
* `num_variables`, `rank_x`, `rank_z`.
* Normalized constraint lists with sorted variable indices.

Binary serialization wraps the JSON payload using `bincode` for stable
round-tripping. Deserialization rebuilds adjacency caches and rank metadata via
`hash::reconstruct`.

### Hashing

`hash::canonical_code_hash` computes a SHA-256 digest over:

1. Schema version `(major, minor, patch)`.
2. `num_variables`, `num_constraints_x/z`, `rank_x/z`.
3. Sorted constraints – length followed by ordered variable indices.

The resulting hexadecimal string is used throughout provenance tracking.

## Determinism policy

* All stateful operations accept either explicit seeds (embedded in
  `RunProvenance`) or deterministic structures (bit vectors, sorted constraint
  sets). The crate performs no internal random sampling.
* RNG interactions for higher phases should use the `RngHandle` from
  `asm-core`; Phase 3 components only consume deterministic inputs.
* Batched syndrome evaluations reuse cached adjacency information to preserve
  deterministic iteration order.
* Dispersion curves depend only on species support size, requested steps, and
  graph degree bounds, guaranteeing identical outputs for repeated runs with the
  same inputs.

## Error semantics

The crate never panics in response to user input. All validation failures use
`AsmError::Code` with structured `ErrorInfo` payloads:

| Code | Meaning | Context keys |
| ---- | ------- | ------------ |
| `variable-out-of-range` | Constraint references an invalid variable | `constraint_kind`, `constraint_index`, `num_variables` |
| `duplicate-constraint` | Duplicate X/Z stabilizer provided | `constraint_kind`, `constraint_index` |
| `css-orthogonality-failed` | Pair of constraints anticommute | `x_index`, `z_index` |
| `invalid-state-bit` | State bits were not 0/1 | – |
| `unknown-state-handle` | Provided state handle not created by `asm-code` | – |
| `null-state-handle` | Null pointer received for constraint state | – |
| `state-length-mismatch` | State size does not match code variables | `state_len`, `num_variables` |
| `empty-dispersion-steps` | Dispersion requested with zero steps | – |

Additional errors triggered by downstream crates (e.g. hypergraph degree bound
queries) bubble up unchanged.

## Serialization schema & canonical hash

* JSON payload fields are serialized in the order listed above, with constraint
  arrays sorted lexicographically.
* Binary payload is simply the UTF-8 JSON string wrapped by `bincode` to
  preserve ordering and textual readability.
* Canonical hashes ignore `RunProvenance` ordering except for the implicit code
  hash stored there. Field order is deterministic to ensure stability across
  platforms.

## Dispersion measurement settings

* Default sample steps: `[1, 2, 4]`.
* Default tolerance: `1e-6` used to compute a pseudo-confidence level.
* Residuals are absolute deviations from the averaged limiting speed. Tests
  assert that residuals remain bounded by the common velocity for synthetic
  fixtures.

## Determinism & reproducibility checklist

1. Build the code with normalized constraint lists.
2. Use `StateHandle::from_bits` to derive deterministic state snapshots.
3. Pass state handles by reference; the crate validates ownership via an
   internal tag to avoid aliasing non-owned data.
4. For repeated dispersion runs, reuse the same `species` slice and `steps`
   vector to receive byte-identical reports.

## Handover expectations

Phase 4 components can rely on:

* Stable adjacency queries via `x_adjacency`/`z_adjacency`.
* Deterministic defect extraction and fusion semantics.
* Canonical hashes and provenance wiring compatible with Phase 2 graphs.
* Batched syndrome evaluation returning `ViolationSet` structures with borrowed
  slices suitable for zero-copy scoring pipelines.
