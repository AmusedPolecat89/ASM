# ASM Graph Engine — Phase 2 Contracts

This document captures the public API, invariants, and determinism policies for the
`asm-graph` crate. The crate provides a concrete implementation of the
[`asm_core::Hypergraph`](../../asm-core/src/lib.rs) trait together with deterministic
randomised constructors, curvature diagnostics, local rewiring moves, and
serialization facilities. All consumers of the graph engine must conform to the
contracts described here.

## Core type: `HypergraphImpl`

`HypergraphImpl` is a deterministic, directed hypergraph whose identifiers match the
`NodeId` and `EdgeId` types from `asm-core`. Identifiers are monotonically issued and
never reused unless a future `compact` routine is invoked. Mutating operations return
`AsmError::Graph` with structured context instead of panicking.

### Construction and configuration

| Method | Purpose | Inputs | Outputs | Notes |
| --- | --- | --- | --- | --- |
| `HypergraphImpl::new(config)` | Build an empty graph with the supplied configuration. | [`HypergraphConfig`](#hypergraphconfig) | `HypergraphImpl` | Configuration is cloned internally; callers retain ownership. |
| `HypergraphImpl::default()` | Convenience wrapper using [`HypergraphConfig::default`](#hypergraphconfig). | – | `HypergraphImpl` | Default config enables causal mode and balanced 2+2 uniformity. |
| `add_node()` | Append a new node. | – | `NodeId` | Returned identifier equals current node count prior to insertion. |
| `remove_node(node)` | Mark a node as tombstoned. | `NodeId` | – | Fails with `node-not-isolated` if incident edges remain. |
| `add_hyperedge(sources, destinations)` | Insert a directed hyperedge. | slices of `NodeId` | `EdgeId` | Rejects empty endpoints, duplicate IDs, duplicate hyperedges, degree-cap, uniformity, and causal violations. |
| `remove_hyperedge(edge)` | Remove an existing hyperedge. | `EdgeId` | – | Removal updates all degree indices and the signature index. |
| `overwrite_edge(edge, new_sources, new_destinations)` | Replace endpoints in-place. | `EdgeId`, slices of `NodeId` | – | Validates identical invariants as `add_hyperedge`. On failure the original edge is restored. |

Read-only helpers expose adjacency and degree information:

| Method | Description | Complexity | Errors |
| --- | --- | --- | --- |
| `nodes()` / `edges()` | Return `ExactSizeIterator`s over alive identifiers. | O(n) materialisation per call. | Never fails. |
| `hyperedge(edge)` | Fetches `HyperedgeEndpoints`. | O(k log n) (copy of endpoints). | `unknown-edge`. |
| `degree_bounds()` | Returns [`asm_core::DegreeBounds`]. | O(n log n). | – |
| `in_degree(node)` / `out_degree(node)` | Degree counters for a node. | O(log n) per query. | `unknown-node`. |
| `edges_touching(node)` | Sorted list of incident edges. | O(d log d). | `unknown-node`. |
| `outgoing_edges(node)` / `incoming_edges(node)` | Directional adjacency. | O(d log d). | `unknown-node`. |
| `src_of(edge)` / `dst_of(edge)` | Borrowed slices over stored endpoints. | O(1). | `unknown-edge`. |

### `HypergraphConfig`

```rust,ignore
pub struct HypergraphConfig {
    pub causal_mode: bool,
    pub max_in_degree: Option<usize>,
    pub max_out_degree: Option<usize>,
    pub k_uniform: Option<KUniformity>,
    pub schema_version: SchemaVersion,
}
```

* **Causal mode** rejects any mutation that would introduce a directed cycle when
  each hyperedge is interpreted as all-to-all arcs between its sources and destinations.
  Cycle detection is performed deterministically with depth-first search.
* **Degree limits** bound inbound and outbound degree at mutation time. All
  adjacency indices are updated atomically; no partial state is ever leaked.
* **`KUniformity`** options:
  * `Balanced { sources, destinations }` – every hyperedge must have exactly the
    specified counts in each direction.
  * `Total { total, min_sources }` – total arity must equal `total` while sources
    satisfy `>= min_sources` and destinations are implicitly `total - sources`.
* **Schema version** is stored in every serialized payload and forms part of the
  canonical hash derivation.

### Invariants

* Node and edge identifiers are never reused; removed entities become tombstones.
* Endpoint slices are deduplicated and stored in sorted order; duplicate hyperedges
  (same source/destination sets) are rejected with `duplicate-edge`.
* Degree caps are enforced eagerly (`out-degree-cap` / `in-degree-cap`).
* Uniformity violations produce `invalid-arity`.
* Attempting to remove a node with incident edges yields `node-not-isolated`.
* Determinism — identical seeds, operations, and orderings produce bit-identical
  structures and canonical hashes.

## Generators

Two deterministic constructors seed initial structures via [`asm_core::rng::RngHandle`]:

| Function | Description | Determinism | Errors |
| --- | --- | --- | --- |
| `gen_bounded_degree(n_nodes, degree_max, k_uniform, rng)` | Builds a causal graph honouring uniform arity and degree caps. | Fully deterministic relative to `rng`. | `empty-graph` when `n_nodes == 0`; otherwise propagates structural errors. |
| `gen_quasi_regular(n_nodes, degree_target, k_uniform, rng)` | Starts from `gen_bounded_degree` then performs inbound-degree balancing rewires. | Deterministic sequence of rewires per seed. | Bubble up from generator/rewires. |

Both functions respect the substream policy — callers must derive sub-seeds via
`asm_core::derive_substream_seed` when parallelising generation.

## Curvature diagnostics

*Edge curvature* uses a light-weight Forman surrogate:

```text
F(e) = 2 - (|src| + |dst|)
       + Σ_{s∈src} 1 / (1 + out_degree(s))
       + Σ_{t∈dst} 1 / (1 + in_degree(t))
```

*Node curvature* averages incident edge curvature contributions.

The `ollivier_lite_nodes` heuristic performs `iterations` rounds of symmetric
neighbour averaging over the 1-hop neighbourhood graph, initialised with
`1 / (1 + degree(node))`. At least one iteration is required; `zero-iterations`
errors otherwise. The procedure is deterministic because neighbours are sorted
before averaging.

## Rewiring moves

All rewiring helpers mutate the graph atomically and return the canonical hash
post-mutation. Validators (`*_dry_run`) perform the same checks on cloned graphs
without touching the original state.

| Function | Behaviour | Validation | Errors |
| --- | --- | --- | --- |
| `rewire_swap_targets(edge_a, edge_b)` | Exchanges destination sets between two edges. | Rejects identical edges early. | Propagates structural violations from `overwrite_edge`. |
| `rewire_retarget(edge, removed, added)` | Replaces a subset of destinations with new nodes. | Ensures `removed ⊆ dst(edge)` and `added` nodes exist. | `missing-destination`, `empty-destinations`, or underlying structural errors. |
| `rewire_resource_balanced(node, rng)` | Degree-aware tweak: picks a random outgoing edge and shifts a destination from the highest-loaded node towards the globally least-loaded node. | Skips when no improvement is possible. | Propagates invariants; deterministic for a fixed RNG stream. |

All rewiring helpers internally call `overwrite_edge`, ensuring identical
invariant checks as `add_hyperedge`. Failures never leave partially updated
structures.

## Serialization and canonical hashing

Serialization lives in `serialization.rs` and exposes:

* `graph_to_bytes` / `graph_from_bytes` – `bincode` payloads storing configuration,
  alive node flags, and per-edge endpoint vectors.
* `graph_to_json` / `graph_from_json` – human readable schema mirroring the binary format.

Every payload embeds `HypergraphConfig::schema_version`. Deserialization rejects
invalid structures (duplicate edges, cap violations, etc.) with `AsmError::Serde`
or `AsmError::Graph` carrying detailed context.

Canonical hashes are SHA-256 digests over:

1. `causal_mode`, degree caps, uniformity parameters, and schema version.
2. Alive node count.
3. Sorted edge signatures (sorted endpoints encoded as little-endian `u64`).

The resulting lowercase hexadecimal string feeds provenance tracking in later phases.

## Determinism policy

* All randomness flows through `RngHandle` and respects the substream hashing rule.
* Generators and rewires do not sample from global state; callers must pass seeded
  handles explicitly.
* Serialization, hashing, and curvature routines iterate in lexicographic order to
  ensure reproducibility across platforms.

## Performance expectations

* Adjacency queries (`src_of`, `dst_of`, degree lookups) are O(1)–O(log n) thanks to
  B-tree indices.
* Hyperedge additions/removals are O(k log n) in the number of touched endpoints.
* Generators run in O(n · degree_max) expected time.
* Curvature diagnostics are O(|E| log |V|) due to map lookups.
* Rewiring moves operate on local neighbourhoods in O(k log n).

## Testing & CI

The following invariants are covered by automated tests under
`crates/asm-graph/tests/`:

* Structural round-trips, ID stability, and serialization determinism.
* Causal mode enforcement versus non-causal configurations.
* Degree-cap validation and error surface coverage.
* Curvature sanity checks on canonical families (stars, chains, balanced graphs).
* Rewiring correctness and canonical hash stability.
* Property-based fuzzing of random generators and rewires.

Continuous integration must run `cargo fmt --all -- --check`,
`cargo clippy --all -- -D warnings`, and `cargo test --all`.
Benchmarks in `crates/asm-graph/benches/` provide baseline measurements and
emit artifacts into `repro/phase2/` when executed locally.

