//! Unit-local ownership and borrowed analysis. Dense IDs are handles, not names.
mod bundle;
mod expanded;
mod manifest;
pub use manifest::BundleManifest;

use crate::{HirFnFacts, HirNode, helper_calls::Summary, producer::Pair};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
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

    fn native_width(&self) -> Result<bool, String> {
        crate::helper_calls::compute_native_width(self.facts())
    }

    fn frontend_reference(&self) -> Option<serde_json::Value> {
        None
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
    identity: String,
    functions: Vec<HirFnFacts>,
    helpers: Vec<HelperRecord>,
    graphs: Vec<Pair>,
    roots: Vec<Option<GraphId>>,
    links: Vec<BTreeMap<Site, FunctionId>>,
    certified: Vec<Option<HelperId>>,
}

/// A proof cache belongs to this exact immutable store and this analysis only.
/// Policy partitions are never serialized and successful proofs alone enter it.
pub struct UnitAnalysis<'a> {
    store: &'a UnitStore,
    proofs: Proofs,
    // Immutable derived values; policy-specific proof success is stored above.
    native_widths: RefCell<BTreeMap<FunctionId, bool>>,
}

trait Resolver {
    fn tables(&self) -> Tables<'_>;
    fn proofs(&self) -> &Proofs;
    fn native_widths(&self) -> &RefCell<BTreeMap<FunctionId, bool>>;
    fn identity(&self) -> &str;
}

struct Tables<'a> {
    functions: &'a [HirFnFacts],
    helpers: &'a [HelperRecord],
    graphs: &'a [Pair],
    links: &'a [BTreeMap<Site, FunctionId>],
    certified: &'a [Option<HelperId>],
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
    resolver: &'a dyn Resolver,
    id: HelperId,
}

impl EvidenceFacts for FunctionView<'_> {
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
            serde_json::json!({"identity":"rstyle.unit.function-reference/1", "unit":self.resolver.identity(), "function":self.id}),
        )
    }

    fn facts(&self) -> &HirFnFacts {
        &self.resolver.tables().functions[self.id.0 as usize]
    }

    fn helper(&self, node: &HirNode) -> Result<Option<HelperView<'_>>, String> {
        let tables = self.resolver.tables();
        let target = tables.links[self.id.0 as usize]
            .get(&site(node))
            .ok_or("Missing resolved unit helper link")?;
        let id = tables.certified[target.0 as usize]
            .ok_or("Unit helper is unsupported, recursive or not yet certified")?;
        Ok(Some(HelperView {
            resolver: self.resolver,
            id,
        }))
    }
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
        serde_json::json!({"identity": "rstyle.helper.unit-reference/1", "helper": self.id})
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
        UnitAnalysis {
            store: self,
            proofs: RefCell::default(),
            native_widths: RefCell::default(),
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
    fn identity(&self) -> &str {
        &self.store.identity
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
        }
    }
}

struct Builder {
    store: UnitStore,
    pending: Vec<FunctionId>,
    proofs: Proofs,
    native_widths: RefCell<BTreeMap<FunctionId, bool>>,
}

impl Resolver for Builder {
    fn identity(&self) -> &str {
        &self.store.identity
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
    fn new(mut functions: Vec<HirFnFacts>) -> Result<Self, String> {
        for facts in &mut functions {
            facts.build.frontend_options.normalize_data();
        }
        u32::try_from(functions.len()).map_err(|_| "Oversized function table")?;
        let identity = unit_identity(&functions)?;
        let pending = (0..functions.len())
            .map(|index| local_id(index, FunctionId))
            .collect::<Result<Vec<_>, _>>()?;
        let links = links(&functions)?;
        let certified = vec![None; functions.len()];
        let roots = vec![None; functions.len()];

        Ok(Self {
            store: UnitStore {
                identity,
                functions,
                helpers: Vec::new(),
                graphs: Vec::new(),
                roots,
                links,
                certified,
            },
            pending,
            proofs: RefCell::default(),
            native_widths: RefCell::default(),
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
        let depth = self.store.links[function.0 as usize]
            .values()
            .map(|target| {
                self.store.certified[target.0 as usize]
                    .map(|id| self.store.helpers[id.0 as usize].depth + 1)
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);
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

fn unit_identity(functions: &[HirFnFacts]) -> Result<String, String> {
    use sha2::Digest;
    let bytes = serde_json::to_vec(functions).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", sha2::Sha256::digest(bytes)))
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
