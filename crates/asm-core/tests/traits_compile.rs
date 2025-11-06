use asm_core::errors::{AsmError, ErrorInfo};
use asm_core::provenance::{RunProvenance, SchemaVersion};
use asm_core::rng::{derive_substream_seed, RngHandle};
use asm_core::{
    ConstraintProjector, DegreeBounds, EdgeId, HyperedgeEndpoints, Hypergraph,
    LogicalAlgebraSummary, NodeId, OperatorDiagnostics, OperatorDictionary,
    OperatorDictionaryOptions, OperatorDictionaryResult, RGMap, RGMapOutcome, RGMapParameters,
    RGMapReport,
};
use rand::RngCore;

#[derive(Default)]
struct DummyGraph;

impl Hypergraph for DummyGraph {
    fn nodes(&self) -> Box<dyn ExactSizeIterator<Item = NodeId> + '_> {
        Box::new(vec![NodeId::from_raw(0)].into_iter())
    }

    fn edges(&self) -> Box<dyn ExactSizeIterator<Item = EdgeId> + '_> {
        Box::new(vec![EdgeId::from_raw(0)].into_iter())
    }

    fn hyperedge(&self, _edge: EdgeId) -> Result<HyperedgeEndpoints, AsmError> {
        Ok(HyperedgeEndpoints {
            sources: vec![NodeId::from_raw(0)].into_boxed_slice(),
            destinations: vec![NodeId::from_raw(1)].into_boxed_slice(),
        })
    }

    fn degree_bounds(&self) -> Result<DegreeBounds, AsmError> {
        Ok(DegreeBounds {
            min_in_degree: Some(0),
            max_in_degree: Some(1),
            min_out_degree: Some(0),
            max_out_degree: Some(1),
        })
    }

    fn add_node(&mut self) -> Result<NodeId, AsmError> {
        Ok(NodeId::from_raw(2))
    }

    fn add_hyperedge(
        &mut self,
        _sources: &[NodeId],
        _destinations: &[NodeId],
    ) -> Result<EdgeId, AsmError> {
        Ok(EdgeId::from_raw(2))
    }

    fn remove_node(&mut self, _node: NodeId) -> Result<(), AsmError> {
        Ok(())
    }

    fn remove_hyperedge(&mut self, _edge: EdgeId) -> Result<(), AsmError> {
        Ok(())
    }
}

#[derive(Default)]
struct DummyProjector;

impl ConstraintProjector for DummyProjector {
    fn num_variables(&self) -> usize {
        1
    }

    fn num_constraints(&self) -> usize {
        1
    }

    fn rank(&self) -> Result<usize, AsmError> {
        Ok(1)
    }

    fn check_violations(
        &self,
        _state: &dyn asm_core::ConstraintState,
    ) -> Result<Box<[usize]>, AsmError> {
        Ok(vec![0].into_boxed_slice())
    }

    fn logical_algebra_summary(&self) -> Result<LogicalAlgebraSummary, AsmError> {
        Ok(LogicalAlgebraSummary {
            num_logical: 0,
            labels: vec![],
            metadata: Default::default(),
        })
    }
}

struct DummyRg;

impl RGMap for DummyRg {
    fn apply(
        &self,
        code: &dyn ConstraintProjector,
        graph: &dyn Hypergraph,
        params: &RGMapParameters,
    ) -> Result<RGMapOutcome, AsmError> {
        let _ = derive_substream_seed(params.substream.unwrap_or(0), 0);
        let _ = graph.nodes().len();
        Ok(RGMapOutcome {
            code: Box::new(DummyProjector::default()),
            graph: Box::new(DummyGraph::default()),
            report: RGMapReport {
                scale_factor: code.num_variables() as f64,
                truncation_estimate: Some(0.0),
                symmetry_flags: Default::default(),
                equivariance_flags: Default::default(),
                parent_provenance: Default::default(),
            },
        })
    }
}

struct DummyDictionary;

impl OperatorDictionary for DummyDictionary {
    fn extract(
        &self,
        _code: &dyn ConstraintProjector,
        _graph: &dyn Hypergraph,
        opts: &OperatorDictionaryOptions,
    ) -> Result<OperatorDictionaryResult, AsmError> {
        let seed = opts.seed.unwrap_or(0);
        let substream_seed = opts
            .substream
            .map(|index| derive_substream_seed(seed, index))
            .unwrap_or(seed);
        let provenance = RunProvenance {
            seed: substream_seed,
            ..RunProvenance::default()
        };
        let couplings = asm_core::Couplings {
            schema_version: SchemaVersion::default(),
            provenance,
            c_kin: 0.0,
            gauge: [0.0; 3],
            yukawa: vec![],
            lambda_h: 0.0,
            notes: Some("deterministic".into()),
        };
        Ok(OperatorDictionaryResult {
            couplings,
            diagnostics: OperatorDiagnostics::default(),
        })
    }
}

fn accepts_trait_objects(
    projector: &dyn ConstraintProjector,
    graph: &dyn Hypergraph,
    dict: &dyn OperatorDictionary,
    rg: &dyn RGMap,
) {
    let state = ();
    let _ = projector.check_violations(&state);
    let opts = OperatorDictionaryOptions::default();
    let _ = dict.extract(projector, graph, &opts).unwrap();
    let params = RGMapParameters::default();
    let _ = rg.apply(projector, graph, &params).unwrap();
}

#[test]
fn trait_objects_are_object_safe() {
    let projector: Box<dyn ConstraintProjector> = Box::new(DummyProjector::default());
    let graph: Box<dyn Hypergraph> = Box::new(DummyGraph::default());
    let dict: Box<dyn OperatorDictionary> = Box::new(DummyDictionary);
    let rg: Box<dyn RGMap> = Box::new(DummyRg);

    accepts_trait_objects(&*projector, &*graph, &*dict, &*rg);
}

#[test]
fn rng_handle_compiles() {
    let mut rng = RngHandle::from_seed(42);
    let _ = rng.next_u64();
}

#[test]
fn error_info_formatting() {
    let info = ErrorInfo::new("E001", "problem").with_context("node", "1");
    let err = AsmError::Graph(info.clone());
    assert_eq!(err.info(), &info);
}
