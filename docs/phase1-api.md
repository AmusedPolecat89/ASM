# ASM Core Phase 1 API

This document captures the Phase 1 engine contracts for the `asm-core` crate.
The traits and data types described here are stable once Phase 1 lands and can
only be changed via the RFC process.

## Hypergraph Trait

| Method | Purpose | Inputs | Outputs | Preconditions | Postconditions | Complexity | Errors |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `nodes()` | Enumerate node identifiers. | `&self` | `Box<dyn ExactSizeIterator<Item = NodeId>>` | Graph initialized. | Iterator covers every node exactly once with stable ordering. | `O(1)` to return, iterator `O(1)` per element. | `AsmError::Graph` for storage failures. |
| `edges()` | Enumerate hyperedge identifiers. | `&self` | `Box<dyn ExactSizeIterator<Item = EdgeId>>` | Graph initialized. | Iterator covers every edge exactly once with stable ordering. | `O(1)` to return, iterator `O(1)` per element. | `AsmError::Graph` for storage failures. |
| `hyperedge(eid)` | Query a hyperedge. | `EdgeId` | `HyperedgeEndpoints` | `eid` must exist. | Sources/destinations match construction order. | `O(1)` to `O(log n)` lookup. | `AsmError::Graph` with code `G_NOT_FOUND` if missing. |
| `degree_bounds()` | Cached degree extrema. | `&self` | `DegreeBounds` | Graph populated. | Bounds reflect all nodes; unknown values encoded as `None`. | `O(1)` | `AsmError::Graph` on stale cache. |
| `add_node()` | Append a node. | `&mut self` | `NodeId` | Graph mutable, causal invariants satisfied. | New `NodeId` unique and stable. | amortized `O(1)` | `AsmError::Graph` on allocation failure. |
| `add_hyperedge(sources, destinations)` | Append a hyperedge. | `&mut self`, slices of `NodeId` | `EdgeId` | Endpoints must exist; causal mode forbids directed cycles. | Edge inserted with stable `EdgeId`. | amortized `O(1)` | `AsmError::Graph` with codes `G_BAD_ENDPOINT` or `G_CAUSAL`. |
| `remove_node(node)` | Remove a node. | `NodeId` | `()` | Node exists and has no incident edges unless supported. | Node ID retired; not reused until compaction RFC. | `O(1)` to `O(log n)` | `AsmError::Graph` with `G_NOT_FOUND`, `G_STILL_REFERENCED`. |
| `remove_hyperedge(edge)` | Remove a hyperedge. | `EdgeId` | `()` | Edge exists. | Edge ID retired; not reused. | `O(1)` to `O(log n)` | `AsmError::Graph` with `G_NOT_FOUND`. |

### Hypergraph Invariants

* Identifiers are stable for the lifetime of the instance. No reuse occurs until
  a future "compact" RFC introduces explicit reindexing APIs.
* Implementations may provide a causal mode. When enabled, any `add_hyperedge`
  call that introduces a directed cycle **must** reject the change with
  `AsmError::Graph` code `G_CAUSAL`.
* Query methods are expected to complete in `O(1)`–`O(log n)` time.

## ConstraintProjector Trait

| Method | Purpose | Inputs | Outputs | Preconditions | Postconditions | Complexity | Errors |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `num_variables()` | Physical variable count. | `&self` | `usize` | Initialized code. | Returns static size. | `O(1)` | none |
| `num_constraints()` | Constraint count. | `&self` | `usize` | Initialized code. | Returns static size. | `O(1)` | none |
| `rank()` | Rank metadata. | `&self` | `usize` | Internal caches ready. | Returns full rank of constraint matrix. | `O(1)` | `AsmError::Code` (`C_UNINIT`, `C_INCONSISTENT`). |
| `check_violations(state)` | Batch constraint check. | `&dyn ConstraintState` | `Box<[usize]>` | `state` produced by projector. | Returned indices sorted ascending, deterministic. | `O(m)` where `m` is number of violated constraints. | `AsmError::Code` codes `C_BAD_STATE`, `C_UNSUPPORTED`. |
| `logical_algebra_summary()` | Lightweight algebra summary. | `&self` | `LogicalAlgebraSummary` | Code initialized. | Summary stable across identical inputs. | `O(1)` | `AsmError::Code` for inconsistent algebra. |

### Constraint Projector Notes

* The `ConstraintState` trait is intentionally opaque; implementations may
  expose concrete types in Phase 2. Users must only pass handles produced by the
  same projector instance.

## RGMap Trait

| Method | Purpose | Inputs | Outputs | Preconditions | Postconditions | Complexity | Errors |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `apply(code, graph, params)` | Coarse-grain a code/hypergraph pair. | `&dyn ConstraintProjector`, `&dyn Hypergraph`, `&RGMapParameters` | `RGMapOutcome` | Inputs share compatible provenance. | Returned code/graph encode coarse version and inherit deterministic provenance links. | Dominant cost implementation defined; control path `O(1)`. | `AsmError::RG` codes `R_INCOMPATIBLE`, `R_NUMERICS`, `R_CAUSAL`. |

### RGMap Report Structure

`RGMapOutcome` contains:

* `code`: boxed projector implementing the contract.
* `graph`: boxed hypergraph implementing the contract.
* `report`: metadata with fields
  * `scale_factor` – coarse-graining ratio.
  * `truncation_estimate` – numeric error estimate.
  * `symmetry_flags` – map from symmetry label to preservation flag.
  * `equivariance_flags` – map describing equivariance status.
  * `parent_provenance` – map of hashes linking to parent runs (`graph`, `code`, etc.).

## OperatorDictionary Trait

| Method | Purpose | Inputs | Outputs | Preconditions | Postconditions | Complexity | Errors |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `extract(code, graph, opts)` | Produce effective couplings. | `&dyn ConstraintProjector`, `&dyn Hypergraph`, `&OperatorDictionaryOptions` | `OperatorDictionaryResult` | Inputs deterministic and share provenance. | `Couplings` payload stable under identical seeds, diagnostics deterministic. | Implementation-defined, should be dominated by solver complexity. | `AsmError::Dictionary` codes `D_INCOMPATIBLE`, `D_NUMERICS`, `D_UNSUPPORTED`. |

`OperatorDictionaryResult` bundles the extracted `Couplings` together with
`OperatorDiagnostics` describing uncertainty bounds and additional metadata.

## Data Types

### Couplings

```text
schema_version: SchemaVersion
provenance: RunProvenance
c_kin: f64
gauge: [f64; 3]
yukawa: Vec<f64]
lambda_h: f64
notes: Option<String>
```

* Serialized using Serde with field order as listed above.
* Canonical hashes must serialize fields in this order with UTF-8 keys and
  sorted provenance metadata.

### RunProvenance

```text
input_hash: String
graph_hash: String
code_hash: String
seed: u64
created_at: String
tool_versions: BTreeMap<String, String>
```

Hashes must be computed over canonical, sorted keys. `tool_versions` keys use
lexicographic ordering.

### SchemaVersion

Three-part semantic version `(major, minor, patch)` encoded as unsigned 32-bit
integers.

## Determinism & RNG Policy

* Every randomized method accepts either a `seed: u64` or a mutable
  `RngHandle` derived from `RngHandle::from_seed`.
* Substreams are derived using `derive_substream_seed(master_seed, index)` which
  hashes `(master_seed, index)` with SipHash-1-3 and zero keys.
* Tests must be reproducible across runs and operating systems.
* Implementations **must not** read from non-deterministic sources (OS RNG,
  clocks, etc.) unless explicitly wrapped through `RngHandle`.

## Error Semantics

* User input validation failures return `AsmError`, never panic.
* Every error contains a `family`, stable `code`, `message`, context map, and
  optional hint.
* Recommended codes include:
  * Graph: `G_NOT_FOUND`, `G_BAD_ENDPOINT`, `G_CAUSAL`, `G_STILL_REFERENCED`.
  * Code: `C_UNINIT`, `C_INCONSISTENT`, `C_BAD_STATE`, `C_UNSUPPORTED`.
  * RG: `R_INCOMPATIBLE`, `R_NUMERICS`, `R_CAUSAL`.
  * Dictionary: `D_INCOMPATIBLE`, `D_NUMERICS`, `D_UNSUPPORTED`.
  * RNG: `RN_INVALID_SEED`, `RN_DERIVATION_FAILED`.
  * Serde: `S_SCHEMA_MISMATCH`, `S_VERSION_UNSUPPORTED`.

Context keys should reference identifiers (`node_id`, `edge_id`), sizes
(`expected_rank`, `actual_rank`), or textual reasons (`reason`).

## Serialization Policy

* All public data types implement Serde with explicit `SchemaVersion` fields.
* Canonical hashes must serialize fields using the documented field order and
  UTF-8 encoding, then hash the resulting bytes with a deterministic algorithm
  defined in a future RFC.
* Adding new fields requires bumping the semantic version and providing default
  behavior for older readers.

## Performance Expectations

* Query-oriented trait methods (enumeration, metadata lookups) should run in
  `O(1)` or `O(log n)` time.
* Batch operations accept slices and return slice-backed collections to avoid
  unnecessary copying in future phases.
* Memory allocations must be deterministic and bounded for identical inputs.

## Testing Requirements

* Trait object tests ensure all traits remain object-safe.
* Determinism tests validate RNG reproducibility.
* Serde tests assert round-trip stability.
* Error surface tests enforce code and context coverage.

## Determinism Sign-off Placeholder

Phase 2–6 leads should record their review status in `docs/DECISIONS.md` once
all contracts are approved.
