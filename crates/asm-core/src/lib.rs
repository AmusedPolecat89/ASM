#![deny(missing_docs)]
#![doc = "Core traits and data types for the ASM engine. See docs/phase1-api.md for the full contract."]

use std::collections::BTreeMap;
use std::iter::ExactSizeIterator;

use serde::{Deserialize, Serialize};

pub mod errors;
pub mod provenance;
pub mod rng;
mod types;

pub use errors::{AsmError, ErrorInfo};
pub use provenance::{RunProvenance, SchemaVersion};
pub use rng::{derive_substream_seed, RngHandle};
pub use types::Couplings;

/// Identifier for a node within a [`Hypergraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(u64);

impl NodeId {
    /// Creates a new identifier from its raw integer representation.
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw integer representation of the identifier.
    pub fn as_raw(&self) -> u64 {
        self.0
    }
}

/// Identifier for a hyperedge within a [`Hypergraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EdgeId(u64);

impl EdgeId {
    /// Creates a new identifier from its raw integer representation.
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw integer representation of the identifier.
    pub fn as_raw(&self) -> u64 {
        self.0
    }
}

/// Bounds on inbound and outbound degrees for a collection of nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DegreeBounds {
    /// Minimum inbound degree observed across all nodes.
    pub min_in_degree: Option<usize>,
    /// Maximum inbound degree observed across all nodes.
    pub max_in_degree: Option<usize>,
    /// Minimum outbound degree observed across all nodes.
    pub min_out_degree: Option<usize>,
    /// Maximum outbound degree observed across all nodes.
    pub max_out_degree: Option<usize>,
}

impl DegreeBounds {
    /// Creates an empty descriptor where no degree information is known yet.
    pub fn unknown() -> Self {
        Self {
            min_in_degree: None,
            max_in_degree: None,
            min_out_degree: None,
            max_out_degree: None,
        }
    }
}

/// Edge description returned by [`Hypergraph::hyperedge`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HyperedgeEndpoints {
    /// Source node identifiers for the hyperedge.
    pub sources: Box<[NodeId]>,
    /// Destination node identifiers for the hyperedge.
    pub destinations: Box<[NodeId]>,
}

/// Describes the structural contract for ASM hypergraphs.
pub trait Hypergraph: Send + Sync {
    /// Returns an iterator over all node identifiers.
    fn nodes(&self) -> Box<dyn ExactSizeIterator<Item = NodeId> + '_>;

    /// Returns an iterator over all edge identifiers.
    fn edges(&self) -> Box<dyn ExactSizeIterator<Item = EdgeId> + '_>;

    /// Returns the endpoints of the specified hyperedge.
    fn hyperedge(&self, edge: EdgeId) -> Result<HyperedgeEndpoints, AsmError>;

    /// Returns cached degree bounds for the graph.
    fn degree_bounds(&self) -> Result<DegreeBounds, AsmError>;

    /// Adds a new node to the hypergraph.
    fn add_node(&mut self) -> Result<NodeId, AsmError>;

    /// Adds a new hyperedge connecting the given endpoints.
    fn add_hyperedge(
        &mut self,
        sources: &[NodeId],
        destinations: &[NodeId],
    ) -> Result<EdgeId, AsmError>;

    /// Removes a node from the hypergraph.
    fn remove_node(&mut self, node: NodeId) -> Result<(), AsmError>;

    /// Removes a hyperedge from the hypergraph.
    fn remove_hyperedge(&mut self, edge: EdgeId) -> Result<(), AsmError>;
}

/// Opaque handle trait for constraint projector state snapshots.
pub trait ConstraintState: std::fmt::Debug + Send + Sync {}

impl<T> ConstraintState for T where T: std::fmt::Debug + Send + Sync {}

/// Summary metadata describing logical operator structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LogicalAlgebraSummary {
    /// Number of independent logical operators detected in the code.
    pub num_logical: usize,
    /// Labels associated with logical operators.
    pub labels: Vec<String>,
    /// Auxiliary metadata describing symmetries or grading.
    pub metadata: BTreeMap<String, String>,
}

/// Trait for ASM constraint projectors.
pub trait ConstraintProjector: Send + Sync {
    /// Returns the number of physical variables in the code.
    fn num_variables(&self) -> usize;

    /// Returns the number of constraints enforced by the projector.
    fn num_constraints(&self) -> usize;

    /// Returns the effective rank of the constraint system.
    fn rank(&self) -> Result<usize, AsmError>;

    /// Checks which constraints are violated for the provided state handle.
    fn check_violations(&self, state: &dyn ConstraintState) -> Result<Box<[usize]>, AsmError>;

    /// Returns a lightweight summary of the logical algebra.
    fn logical_algebra_summary(&self) -> Result<LogicalAlgebraSummary, AsmError>;
}

/// Parameters supplied to an RG map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RGMapParameters {
    /// Integer label for deterministic substreams.
    pub substream: Option<u64>,
    /// Arbitrary user supplied options (tuning knobs, heuristics, etc.).
    pub options: BTreeMap<String, String>,
}

/// Outcome of applying an RG map.
/// Outcome of applying an RG map.
pub struct RGMapOutcome {
    /// Coarse-grained code.
    pub code: Box<dyn ConstraintProjector>,
    /// Coarse-grained graph.
    pub graph: Box<dyn Hypergraph>,
    /// Structured report describing the transformation.
    pub report: RGMapReport,
}

impl std::fmt::Debug for RGMapOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RGMapOutcome").finish_non_exhaustive()
    }
}

/// Structured report emitted by an RG map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RGMapReport {
    /// Coarse-graining scale factor applied to the lattice.
    pub scale_factor: f64,
    /// Estimated truncation error introduced by the RG step.
    pub truncation_estimate: Option<f64>,
    /// Flags describing which symmetries were preserved.
    pub symmetry_flags: BTreeMap<String, bool>,
    /// Whether equivariance with respect to specified symmetries holds.
    pub equivariance_flags: BTreeMap<String, bool>,
    /// Provenance information linking to parent runs.
    pub parent_provenance: BTreeMap<String, String>,
}

/// Trait for renormalization group maps operating on ASM codes.
pub trait RGMap: Send + Sync {
    /// Applies the RG map to the provided code and hypergraph.
    fn apply(
        &self,
        code: &dyn ConstraintProjector,
        graph: &dyn Hypergraph,
        params: &RGMapParameters,
    ) -> Result<RGMapOutcome, AsmError>;
}

/// Options controlling operator dictionary extraction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OperatorDictionaryOptions {
    /// Optional master seed controlling deterministic randomness.
    pub seed: Option<u64>,
    /// Substream index used when branching deterministic sequences.
    pub substream: Option<u64>,
    /// Arbitrary labels forwarded to the extractor.
    pub labels: BTreeMap<String, String>,
}

/// Diagnostics accompanying a dictionary extraction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct OperatorDiagnostics {
    /// Upper bound on the relative uncertainty of extracted couplings.
    pub maximum_relative_uncertainty: Option<f64>,
    /// Additional deterministic metadata emitted by the extractor.
    pub diagnostics: BTreeMap<String, String>,
}

/// Result of operator dictionary extraction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorDictionaryResult {
    /// Extracted coupling constants.
    pub couplings: Couplings,
    /// Deterministic diagnostics required by the API contract.
    pub diagnostics: OperatorDiagnostics,
}

/// Trait describing deterministic operator dictionary extraction.
pub trait OperatorDictionary: Send + Sync {
    /// Extracts effective couplings from the supplied code and graph.
    fn extract(
        &self,
        code: &dyn ConstraintProjector,
        graph: &dyn Hypergraph,
        opts: &OperatorDictionaryOptions,
    ) -> Result<OperatorDictionaryResult, AsmError>;
}
