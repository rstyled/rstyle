//! Unit-local ownership and borrowed analysis. Dense IDs are handles, not names.
mod bundle;
pub use bundle::BUNDLE_SCHEMA;
pub use bundle::{read_diagnostic_snapshot, write_diagnostic_snapshot};
mod dependency_closure;
mod derived;
mod instances;
pub use dependency_closure::DependencyClosure;
mod manifest;
mod namespace;
mod source_analysis;
pub use manifest::{BundleManifest, VerifiedBundle, transport_checksum};
pub use namespace::UnitNamespace;
pub use source_analysis::{SourceAnalyses, SourceAnalysis};

use crate::{HirFnFacts, HirNode, helper_calls::Summary, producer::Pair};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
    rc::Rc,
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FunctionId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HelperId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GraphId(u32);

mod sealed {
    pub trait Sealed {}
    impl Sealed for crate::HirFnFacts {}
    impl Sealed for super::FunctionView<'_> {}
    impl<T: super::EvidenceFacts + ?Sized> Sealed for &T {}
}

/// Shared and standalone facts use one analysis implementation. Standalone
/// facts retain the supported strict legacy transport adapter. Sealing prevents
/// helper views from being substituted from a different owning unit.
pub trait EvidenceFacts: sealed::Sealed {
    fn facts(&self) -> &HirFnFacts;

    /// Only an immutable, separately verified instance owner supplies this
    /// witness. Raw frontend records and serialized flags supply none.
    fn scalar_instance(&self) -> Option<&crate::scalar_instances::CheckedProjection> {
        None
    }

    /// Check retained source inputs and their unit closure, without nominal type permission.
    fn verify_source_inputs(&self) -> Result<(), String> {
        crate::nominal_boundary::resolution::verify_source_inputs(self.facts())
    }

    /// Verify facts only; mutable graph documents never share this result.
    fn verify_data_frontend(&self) -> Result<(), String> {
        crate::data_frontend::check(self.facts())
    }

    /// Origin results concern immutable source nodes, never graph documents.
    fn reference_origins(
        &self,
        node: &HirNode,
        selectors: &[usize],
    ) -> Result<Vec<crate::data_origins::Origin>, String> {
        crate::data_reference_locals::compute_origins(node, selectors, self.facts())
    }

    /// Immutable receiver proofs are local to the owner, source mode and policy.
    fn receiver_inventory(
        &self,
        original_source: bool,
    ) -> Result<std::borrow::Cow<'_, crate::data_receivers::Capture>, String> {
        crate::data_receivers::check(self.facts(), original_source).map(std::borrow::Cow::Owned)
    }

    /// Check original scalar Option spelling in the exact lexical function
    /// owner. This supplies no semantic type or nominal ownership permission.
    fn verify_option_type(&self, path: &syn::TypePath, scalar: &str) -> Result<(), String> {
        crate::nominal_boundary::resolution::verify_option_type(self.facts(), path, scalar)
    }

    /// Resolve exact source/type evidence; shared views retain successful results
    /// only within their owning analysis and verification policy.
    fn nominal_resolution(
        &self,
    ) -> Result<crate::nominal_boundary::resolution::ResolutionView<'_>, String> {
        crate::nominal_boundary::resolution::from_facts(self.facts())
            .map(crate::nominal_boundary::resolution::ResolutionView::owned)
    }

    fn inherent_definition(
        &self,
    ) -> Result<Option<std::borrow::Cow<'_, crate::inherent_calls::Definition>>, String> {
        crate::inherent_calls::check_definition(self.facts())
            .map(|record| record.map(std::borrow::Cow::Owned))
    }

    fn native_width(&self) -> Result<bool, String> {
        crate::helper_calls::compute_native_width(self.facts())
    }

    fn frontend_reference(&self) -> Option<serde_json::Value> {
        None
    }

    fn external_helper(
        &self,
        _node: &HirNode,
        _callee: &dyn EvidenceFacts,
    ) -> Result<bool, String> {
        Ok(false)
    }

    fn helper(&self, _node: &HirNode) -> Result<Option<HelperView<'_>>, String> {
        Ok(None)
    }
}

impl EvidenceFacts for HirFnFacts {
    fn facts(&self) -> &HirFnFacts {
        self
    }
}

impl<T: EvidenceFacts + ?Sized> EvidenceFacts for &T {
    fn scalar_instance(&self) -> Option<&crate::scalar_instances::CheckedProjection> {
        T::scalar_instance(*self)
    }

    fn verify_source_inputs(&self) -> Result<(), String> {
        T::verify_source_inputs(*self)
    }

    fn verify_data_frontend(&self) -> Result<(), String> {
        T::verify_data_frontend(*self)
    }

    fn reference_origins(
        &self,
        node: &HirNode,
        selectors: &[usize],
    ) -> Result<Vec<crate::data_origins::Origin>, String> {
        T::reference_origins(*self, node, selectors)
    }

    fn receiver_inventory(
        &self,
        original_source: bool,
    ) -> Result<std::borrow::Cow<'_, crate::data_receivers::Capture>, String> {
        T::receiver_inventory(*self, original_source)
    }

    fn verify_option_type(&self, path: &syn::TypePath, scalar: &str) -> Result<(), String> {
        T::verify_option_type(*self, path, scalar)
    }
    fn nominal_resolution(
        &self,
    ) -> Result<crate::nominal_boundary::resolution::ResolutionView<'_>, String> {
        T::nominal_resolution(*self)
    }
    fn inherent_definition(
        &self,
    ) -> Result<Option<std::borrow::Cow<'_, crate::inherent_calls::Definition>>, String> {
        T::inherent_definition(*self)
    }

    fn native_width(&self) -> Result<bool, String> {
        T::native_width(*self)
    }

    fn facts(&self) -> &HirFnFacts {
        T::facts(*self)
    }
    fn helper(&self, node: &HirNode) -> Result<Option<HelperView<'_>>, String> {
        T::helper(*self, node)
    }

    fn frontend_reference(&self) -> Option<serde_json::Value> {
        T::frontend_reference(*self)
    }

    fn external_helper(&self, node: &HirNode, callee: &dyn EvidenceFacts) -> Result<bool, String> {
        T::external_helper(*self, node, callee)
    }
}

impl Deref for dyn EvidenceFacts + '_ {
    type Target = HirFnFacts;
    fn deref(&self) -> &Self::Target {
        self.facts()
    }
}

impl Serialize for dyn EvidenceFacts + '_ {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.facts().serialize(serializer)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HelperRecord {
    function: FunctionId,
    pub(crate) summary: Summary,
    graph: GraphId,
    pub(crate) depth: u32,
}

struct CompletedHelper {
    function: FunctionId,
    summary: Summary,
    pair: Pair,
    depth: u32,
}

type Site = (u32, u32, u32);
type Proofs = RefCell<BTreeSet<(HelperId, bool)>>;

/// Immutable evidence tables. Each frontend and certified pair is owned once.
/// Private local IDs are resolved only through this owning unit's views.
pub struct UnitStore {
    namespace: UnitNamespace,
    functions: Vec<HirFnFacts>,
    helpers: Vec<HelperRecord>,
    graphs: Vec<Pair>,
    roots: Vec<Option<GraphId>>,
    links: Vec<BTreeMap<Site, FunctionId>>,
    certified: Vec<Option<HelperId>>,
    dependencies: Vec<Arc<UnitStore>>,
    external_links: Vec<BTreeMap<Site, ExternalTarget>>,
    instance: Option<instances::InstanceRecord>,
}

/// A proof cache belongs to this exact immutable store and this analysis only.
/// Policy partitions are never serialized and successful proofs alone enter it.
pub struct UnitAnalysis<'a> {
    store: StoreOwner<'a>,
    dependencies: Vec<Rc<UnitAnalysis<'static>>>,
    proofs: Proofs,
    // Immutable derived values; policy-specific proof success is stored above.
    native_widths: RefCell<BTreeMap<FunctionId, bool>>,
    derived: Box<[derived::Derived]>,
    sources: derived::Sources,
}

enum StoreOwner<'a> {
    Borrowed(&'a UnitStore),
    Shared(Arc<UnitStore>),
}

impl Deref for StoreOwner<'_> {
    type Target = UnitStore;
    fn deref(&self) -> &UnitStore {
        match self {
            Self::Borrowed(store) => store,
            Self::Shared(store) => store,
        }
    }
}

#[derive(Clone, Copy)]
struct ExternalTarget {
    dependency: usize,
    function: FunctionId,
}

trait Resolver {
    fn dependencies(&self) -> &[Rc<UnitAnalysis<'static>>];
    fn tables(&self) -> Tables<'_>;
    fn derived(&self) -> &[derived::Derived];
    fn sources(&self) -> &derived::Sources;
    fn proofs(&self) -> &Proofs;
    fn native_widths(&self) -> &RefCell<BTreeMap<FunctionId, bool>>;
    fn namespace(&self) -> &UnitNamespace;
}

struct Tables<'a> {
    functions: &'a [HirFnFacts],
    helpers: &'a [HelperRecord],
    graphs: &'a [Pair],
    links: &'a [BTreeMap<Site, FunctionId>],
    certified: &'a [Option<HelperId>],
    external_links: &'a [BTreeMap<Site, ExternalTarget>],
    instance: Option<&'a instances::InstanceRecord>,
}

pub struct FunctionView<'a> {
    resolver: &'a dyn Resolver,
    id: FunctionId,
}

impl Deref for FunctionView<'_> {
    type Target = HirFnFacts;
    fn deref(&self) -> &Self::Target {
        self.facts()
    }
}

pub struct HelperView<'a> {
    foreign: bool,
    resolver: &'a dyn Resolver,
    id: HelperId,
}

impl EvidenceFacts for FunctionView<'_> {
    fn scalar_instance(&self) -> Option<&crate::scalar_instances::CheckedProjection> {
        (self.id.0 == 0)
            .then_some(self.resolver.tables().instance?)
            .map(|instance| &instance.checked)
    }

    fn verify_source_inputs(&self) -> Result<(), String> {
        self.source_inputs()
    }

    fn verify_data_frontend(&self) -> Result<(), String> {
        self.data_boundary()
    }

    fn reference_origins(
        &self,
        node: &HirNode,
        selectors: &[usize],
    ) -> Result<Vec<crate::data_origins::Origin>, String> {
        self.selected_origins(node, selectors)
    }

    fn receiver_inventory(
        &self,
        original_source: bool,
    ) -> Result<std::borrow::Cow<'_, crate::data_receivers::Capture>, String> {
        self.receivers(original_source)
    }

    fn verify_option_type(&self, path: &syn::TypePath, scalar: &str) -> Result<(), String> {
        self.option_type(path, scalar)
    }

    fn nominal_resolution(
        &self,
    ) -> Result<crate::nominal_boundary::resolution::ResolutionView<'_>, String> {
        self.nominal()
    }
    fn inherent_definition(
        &self,
    ) -> Result<Option<std::borrow::Cow<'_, crate::inherent_calls::Definition>>, String> {
        self.inherent()
    }

    fn native_width(&self) -> Result<bool, String> {
        if let Some(native) = self.resolver.native_widths().borrow().get(&self.id) {
            return Ok(*native);
        }
        let native = crate::helper_calls::compute_native_width(self)?;
        self.resolver
            .native_widths()
            .borrow_mut()
            .insert(self.id, native);
        Ok(native)
    }

    fn frontend_reference(&self) -> Option<serde_json::Value> {
        Some(
            serde_json::json!({"identity":"rstyle.unit.function-reference/2", "namespace":self.resolver.namespace(), "function":self.id}),
        )
    }

    fn facts(&self) -> &HirFnFacts {
        &self.resolver.tables().functions[self.id.0 as usize]
    }

    fn helper(&self, node: &HirNode) -> Result<Option<HelperView<'_>>, String> {
        let tables = self.resolver.tables();
        if let Some(target) = tables.links[self.id.0 as usize].get(&site(node)) {
            let id = tables.certified[target.0 as usize]
                .ok_or("Unit helper is unsupported, recursive or not yet certified")?;
            return Ok(Some(HelperView {
                resolver: self.resolver,
                id,
                foreign: false,
            }));
        }
        self.foreign(node).map(Some)
    }

    fn external_helper(&self, node: &HirNode, callee: &dyn EvidenceFacts) -> Result<bool, String> {
        let helper = self.foreign(node)?;
        Ok(std::ptr::eq(helper.frontend().facts(), callee.facts()))
    }
}

impl FunctionView<'_> {
    fn foreign(&self, node: &HirNode) -> Result<HelperView<'_>, String> {
        let tables = self.resolver.tables();
        let target = tables.external_links[self.id.0 as usize]
            .get(&site(node))
            .ok_or("Missing checked external helper association")?;

        let dependency = self
            .resolver
            .dependencies()
            .get(target.dependency)
            .ok_or("Missing external helper owner")?;
        let id = dependency.store.certified[target.function.0 as usize]
            .ok_or("External helper is unsupported or not certified")?;
        Ok(HelperView {
            resolver: dependency.as_ref(),
            id,
            foreign: true,
        })
    }
}

#[derive(Serialize)]
struct HelperReference<'a> {
    identity: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<&'a UnitNamespace>,
    helper: HelperId,
}

impl HelperView<'_> {
    pub fn summary(&self) -> &Summary {
        &self.record().summary
    }

    pub(crate) fn pair(&self) -> &Pair {
        &self.resolver.tables().graphs[self.record().graph.0 as usize]
    }

    pub(crate) fn frontend(&self) -> FunctionView<'_> {
        FunctionView {
            resolver: self.resolver,
            id: self.record().function,
        }
    }

    pub(crate) fn depth(&self) -> usize {
        self.record().depth as usize
    }

    pub(crate) fn reference(&self) -> serde_json::Value {
        let reference = HelperReference {
            identity: if self.foreign {
                "rstyle.helper.external-reference/1"
            } else {
                "rstyle.helper.unit-reference/1"
            },
            namespace: self.foreign.then(|| self.resolver.namespace()),
            helper: self.id,
        };
        serde_json::to_value(reference).expect("serialize owner-backed helper reference")
    }

    fn record(&self) -> &HelperRecord {
        &self.resolver.tables().helpers[self.id.0 as usize]
    }

    pub(crate) fn verified(&self) -> bool {
        self.resolver
            .proofs()
            .borrow()
            .contains(&(self.id, crate::verification_policy::is_generated()))
    }

    pub(crate) fn remember(&self) {
        self.resolver
            .proofs()
            .borrow_mut()
            .insert((self.id, crate::verification_policy::is_generated()));
    }
}

impl UnitStore {
    /// Takes raw compiler records by move. No nested proof is attached to a tree.
    pub fn capture(functions: Vec<HirFnFacts>) -> Result<Self, String> {
        crate::diagnostics::Stage::new("unit.capture", "unit", None)
            .observe(|| Builder::new(functions)?.capture())
    }

    /// Namespace for local references, independent of evidence contents.
    pub fn namespace(&self) -> &UnitNamespace {
        &self.namespace
    }

    pub fn functions(&self) -> &[HirFnFacts] {
        &self.functions
    }

    pub fn helper_count(&self) -> usize {
        self.helpers.len()
    }

    /// Resolve a root paired document without recreating or decoding it.
    pub fn pair(&self, index: usize) -> Option<&Pair> {
        let id = self.roots.get(index).copied().flatten()?;
        self.graphs.get(id.0 as usize)
    }

    pub fn graph_count(&self) -> usize {
        self.graphs.len()
    }

    fn push_graph(&mut self, pair: Pair) -> Result<GraphId, String> {
        let total = self
            .graphs
            .len()
            .checked_add(1)
            .ok_or("Oversized graph table")?;
        u32::try_from(total).map_err(|_| "Oversized graph table")?;
        let id = local_id(self.graphs.len(), GraphId)?;
        self.graphs.push(pair);

        Ok(id)
    }

    fn add_helper(&mut self, record: CompletedHelper) -> Result<(), String> {
        let id = local_id(self.helpers.len(), HelperId)?;
        let graph = self.push_graph(record.pair)?;
        let record = HelperRecord {
            function: record.function,
            summary: record.summary,
            graph,
            depth: record.depth,
        };

        self.publish_helper(id, record);
        Ok(())
    }

    fn publish_helper(&mut self, id: HelperId, record: HelperRecord) {
        self.certified[record.function.0 as usize] = Some(id);
        self.roots[record.function.0 as usize] = Some(record.graph);
        self.helpers.push(record);
    }

    pub fn analysis(&self) -> UnitAnalysis<'_> {
        UnitAnalysis::new(StoreOwner::Borrowed(self))
    }
}

type AnalysisOwners = BTreeMap<usize, Rc<UnitAnalysis<'static>>>;

fn dependency_analyses(stores: &[Arc<UnitStore>]) -> Vec<Rc<UnitAnalysis<'static>>> {
    let mut owners = AnalysisOwners::new();
    stores
        .iter()
        .map(|store| shared_analysis(store, &mut owners))
        .collect()
}

fn shared_analysis(
    store: &Arc<UnitStore>,
    owners: &mut AnalysisOwners,
) -> Rc<UnitAnalysis<'static>> {
    let key = Arc::as_ptr(store) as usize;
    if let Some(analysis) = owners.get(&key) {
        return Rc::clone(analysis);
    }

    let dependencies = store
        .dependencies
        .iter()
        .map(|store| shared_analysis(store, owners))
        .collect();
    let analysis = Rc::new(UnitAnalysis {
        store: StoreOwner::Shared(Arc::clone(store)),
        dependencies,
        proofs: RefCell::default(),
        native_widths: RefCell::default(),
        derived: derived::contexts(store.functions.len()),
        sources: derived::Sources::default(),
    });

    owners.insert(key, Rc::clone(&analysis));
    analysis
}

impl<'a> UnitAnalysis<'a> {
    fn new(store: StoreOwner<'a>) -> Self {
        let dependencies = dependency_analyses(&store.dependencies);
        let derived = derived::contexts(store.functions.len());
        Self {
            store,
            dependencies,
            proofs: RefCell::default(),
            native_widths: RefCell::default(),
            derived,
            sources: derived::Sources::default(),
        }
    }
}

impl UnitAnalysis<'_> {
    /// Resolve a root from the same store as this context's function views.
    pub fn pair(&self, index: usize) -> Option<&Pair> {
        self.store.pair(index)
    }

    /// Index selection is scoped to this store; no unqualified handle is accepted.
    pub fn function(&self, index: usize) -> Option<FunctionView<'_>> {
        self.store.functions.get(index)?;
        Some(FunctionView {
            resolver: self,
            id: FunctionId(u32::try_from(index).ok()?),
        })
    }
}

impl Resolver for UnitAnalysis<'_> {
    fn dependencies(&self) -> &[Rc<UnitAnalysis<'static>>] {
        &self.dependencies
    }
    fn sources(&self) -> &derived::Sources {
        &self.sources
    }
    fn derived(&self) -> &[derived::Derived] {
        &self.derived
    }
    fn namespace(&self) -> &UnitNamespace {
        &self.store.namespace
    }
    fn tables(&self) -> Tables<'_> {
        self.store.tables()
    }
    fn native_widths(&self) -> &RefCell<BTreeMap<FunctionId, bool>> {
        &self.native_widths
    }
    fn proofs(&self) -> &Proofs {
        &self.proofs
    }
}

impl UnitStore {
    fn tables(&self) -> Tables<'_> {
        Tables {
            functions: &self.functions,
            helpers: &self.helpers,
            graphs: &self.graphs,
            links: &self.links,
            certified: &self.certified,
            external_links: &self.external_links,
            instance: self.instance.as_ref(),
        }
    }
}

struct Builder {
    store: UnitStore,
    dependencies: Vec<Rc<UnitAnalysis<'static>>>,
    pending: Vec<FunctionId>,
    proofs: Proofs,
    native_widths: RefCell<BTreeMap<FunctionId, bool>>,
    derived: Box<[derived::Derived]>,
    sources: derived::Sources,
}

impl Resolver for Builder {
    fn dependencies(&self) -> &[Rc<UnitAnalysis<'static>>] {
        &self.dependencies
    }
    fn sources(&self) -> &derived::Sources {
        &self.sources
    }
    fn derived(&self) -> &[derived::Derived] {
        &self.derived
    }
    fn namespace(&self) -> &UnitNamespace {
        &self.store.namespace
    }
    fn tables(&self) -> Tables<'_> {
        self.store.tables()
    }
    fn native_widths(&self) -> &RefCell<BTreeMap<FunctionId, bool>> {
        &self.native_widths
    }
    fn proofs(&self) -> &Proofs {
        &self.proofs
    }
}

impl Builder {
    fn new(functions: Vec<HirFnFacts>) -> Result<Self, String> {
        Self::in_namespace(functions, UnitNamespace::fresh()?)
    }

    fn in_namespace(functions: Vec<HirFnFacts>, namespace: UnitNamespace) -> Result<Self, String> {
        let functions = derived::normalize(functions)?;
        u32::try_from(functions.len()).map_err(|_| "Oversized function table")?;
        let pending = (0..functions.len())
            .map(|index| local_id(index, FunctionId))
            .collect::<Result<Vec<_>, _>>()?;
        let links = links(&functions)?;
        let certified = vec![None; functions.len()];
        let roots = vec![None; functions.len()];
        let derived = derived::contexts(functions.len());

        Ok(Self {
            store: UnitStore {
                namespace,
                functions,
                helpers: Vec::new(),
                graphs: Vec::new(),
                roots,
                links,
                certified,
                dependencies: Vec::new(),
                external_links: vec![BTreeMap::new(); pending.len()],
                instance: None,
            },
            dependencies: Vec::new(),
            pending,
            proofs: RefCell::default(),
            native_widths: RefCell::default(),
            derived,
            sources: derived::Sources::default(),
        })
    }

    fn capture(mut self) -> Result<UnitStore, String> {
        for _depth in 0..=crate::helper_calls::MAX_DEPTH {
            let completed = self.prepare()?;
            if completed.is_empty() {
                break;
            }
            self.install(completed)?;
        }
        for id in (0..self.store.functions.len()).map(|index| local_id(index, FunctionId)) {
            let id = id?;
            self.depth(id)?;
            crate::helper_reads::check_budgets(&FunctionView {
                resolver: &self,
                id,
            })?;
        }
        self.complete_roots()?;
        Ok(self.store)
    }

    fn complete_roots(&mut self) -> Result<(), String> {
        for index in 0..self.store.functions.len() {
            if self.store.roots[index].is_some() {
                continue;
            }
            let view = FunctionView {
                resolver: self,
                id: local_id(index, FunctionId)?,
            };
            if crate::classify(&view).status == crate::EligibilityVerdict::Eligible {
                let pair = crate::producer::produce(&view).map_err(|errors| errors.join("; "))?;
                let graph = self.store.push_graph(pair)?;
                self.store.roots[index] = Some(graph);
            }
        }
        Ok(())
    }

    fn prepare(&self) -> Result<Vec<CompletedHelper>, String> {
        let mut completed = Vec::new();
        for id in &self.pending {
            let depth = self.depth(*id)?;
            let view = FunctionView {
                resolver: self,
                id: *id,
            };
            if crate::extraction_profile::ExtractionProfile::from_facts(&view)?
                .supports("rust.verified-helper-calls")
                && crate::classify(&view).status == crate::EligibilityVerdict::Eligible
                && let Some((summary, pair)) = crate::helper_calls::certify_view(&view)?
            {
                completed.push(CompletedHelper {
                    function: *id,
                    summary,
                    pair,
                    depth,
                });
            }
        }
        Ok(completed)
    }

    fn depth(&self, function: FunctionId) -> Result<u32, String> {
        let mut depth = self.store.links[function.0 as usize]
            .values()
            .map(|target| {
                self.store.certified[target.0 as usize]
                    .map(|id| self.store.helpers[id.0 as usize].depth + 1)
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);
        for target in self.store.external_links[function.0 as usize].values() {
            let owner = &self.store.dependencies[target.dependency];
            if let Some(helper) = owner.certified[target.function.0 as usize] {
                depth = depth.max(owner.helpers[helper.0 as usize].depth + 1);
            }
        }
        if depth as usize > crate::helper_calls::MAX_DEPTH {
            return Err("Helper dependencies exceed explicit depth budget 8".into());
        }
        Ok(depth)
    }

    fn install(&mut self, completed: Vec<CompletedHelper>) -> Result<(), String> {
        let total = self
            .store
            .helpers
            .len()
            .checked_add(completed.len())
            .ok_or("Oversized helper table")?;
        u32::try_from(total).map_err(|_| "Oversized helper table")?;

        for record in completed {
            self.store.add_helper(record)?;
        }
        self.pending
            .retain(|id| self.store.certified[id.0 as usize].is_none());
        Ok(())
    }
}

fn local_id<T>(index: usize, constructor: impl FnOnce(u32) -> T) -> Result<T, String> {
    let id = u32::try_from(index).map_err(|_| "Oversized unit evidence table")?;
    Ok(constructor(id))
}

fn site(node: &HirNode) -> Site {
    (node.hir.krate, node.hir.owner, node.hir.local)
}

fn links(functions: &[HirFnFacts]) -> Result<Vec<BTreeMap<Site, FunctionId>>, String> {
    let definitions = definitions(functions)?;
    functions
        .iter()
        .map(|facts| function_links(facts, &definitions))
        .collect()
}

fn definitions(functions: &[HirFnFacts]) -> Result<BTreeMap<&str, FunctionId>, String> {
    let mut definitions = BTreeMap::new();
    for (index, facts) in functions.iter().enumerate() {
        if let Some(identity) = facts
            .build
            .frontend_options
            .get("definition_identity")
            .and_then(serde_json::Value::as_str)
            && (identity.is_empty()
                || definitions
                    .insert(identity, local_id(index, FunctionId)?)
                    .is_some())
        {
            return Err("Missing or duplicate resolved helper definition identity".into());
        }
    }
    Ok(definitions)
}

fn function_links(
    facts: &HirFnFacts,
    definitions: &BTreeMap<&str, FunctionId>,
) -> Result<BTreeMap<Site, FunctionId>, String> {
    let helpers = crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.verified-helper-calls");
    let mut links = BTreeMap::new();
    for node in crate::helper_calls::call_nodes(facts) {
        let Some(resolution) = crate::helper_calls::direct_resolution(node) else {
            continue;
        };
        if !resolution[crate::helper_calls::KEY].is_null() {
            return Err("Unit capture requires raw facts without nested helper proofs".into());
        }
        if helpers
            && resolution["local"] == true
            && let Some(target) = resolution["definition_identity"]
                .as_str()
                .and_then(|key| definitions.get(key))
            && links.insert(site(node), *target).is_some()
        {
            return Err("Duplicate helper call occurrence identity".into());
        }
    }
    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_proofs_are_analysis_and_policy_local() {
        let store = UnitStore::capture(Vec::new()).unwrap();
        let analysis = store.analysis();
        // Cache mechanics do not resolve this synthetic handle into a record.
        let helper = HelperView {
            foreign: false,
            resolver: &analysis,
            id: HelperId(0),
        };
        crate::verification_policy::generated(|| helper.remember());
        assert!(!helper.verified());
        assert!(crate::verification_policy::generated(|| helper.verified()));
        helper.remember();
        assert!(helper.verified());
        let independent = store.analysis();
        let independent_helper = HelperView {
            foreign: false,
            resolver: &independent,
            id: HelperId(0),
        };
        assert!(!independent_helper.verified());
        assert!(!crate::verification_policy::generated(|| {
            independent_helper.verified()
        }));
    }

    #[test]
    fn unwind_restores_policy_without_conferring_strict_proof() {
        let store = UnitStore::capture(Vec::new()).unwrap();
        let analysis = store.analysis();
        let helper = HelperView {
            foreign: false,
            resolver: &analysis,
            id: HelperId(0),
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::verification_policy::generated(|| {
                helper.remember();
                panic!("unwind verification scope");
            });
        }));
        assert!(result.is_err());
        assert!(!crate::verification_policy::is_generated());
        assert!(!helper.verified());
        assert!(crate::verification_policy::generated(|| helper.verified()));
    }
}
