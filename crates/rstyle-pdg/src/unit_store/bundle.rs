//! Versioned logical shared tables. Decoding moves each record into one owner;
//! dense IDs survive a round trip but are never global persistent identities.
use super::*;
use std::io::Write;
mod diagnostic;
mod instances;
mod product_instances;
mod reference_forwarders;
mod reference_instances;
pub use diagnostic::{read_diagnostic_snapshot, write_diagnostic_snapshot};
pub use instances::ScalarInstanceReplay;
pub use product_instances::ProductConstructorReplay;
pub use reference_forwarders::ReferenceForwarderReplay;
pub use reference_instances::ReferenceConstructorReplay;
mod function_sources;
mod graph_sources;

pub const BUNDLE_SCHEMA: &str = "rstyle.shared-unit/5";
const LEGACY_SCHEMA: &str = "rstyle.shared-unit/4";

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CallLink {
    site: Site,
    function: FunctionId,
}

#[derive(Serialize)]
struct Export<'a> {
    schema: &'static str,
    namespace: &'a UnitNamespace,
    sources: Vec<&'a str>,
    inherent_units: crate::frontend::option_bundle::Inventories<'a>,
    functions: Vec<function_sources::Export<'a>>,
    helpers: &'a [HelperRecord],
    graphs: &'a [Pair],
    roots: &'a [Option<GraphId>],
    links: Vec<Vec<CallLink>>,
    certified: &'a [Option<HelperId>],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    schema: String,
    namespace: UnitNamespace,
    sources: Vec<crate::SourceSnapshot>,
    #[serde(default, deserialize_with = "present_inventories")]
    inherent_units: Option<Vec<serde_json::Value>>,
    functions: Vec<function_sources::Import>,
    helpers: Vec<HelperRecord>,
    graphs: Vec<Pair>,
    roots: Vec<Option<GraphId>>,
    links: Vec<Vec<CallLink>>,
    certified: Vec<Option<HelperId>>,
}

fn present_inventories<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<serde_json::Value>>, D::Error> {
    Vec::deserialize(deserializer).map(Some)
}

struct Decoded {
    wire: Import,
    functions: Vec<HirFnFacts>,
}

impl Import {
    fn decode(bytes: &[u8]) -> Result<Decoded, String> {
        let mut wire: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if ![BUNDLE_SCHEMA, LEGACY_SCHEMA].contains(&wire.schema.as_str()) {
            return Err("Unsupported shared unit schema".into());
        }

        let mut inventories = crate::frontend::option_bundle::ImportedInventories::new(
            wire.inherent_units.take(),
            wire.schema == LEGACY_SCHEMA,
        )?;
        let mut sources = crate::source_table::Imported::new(&wire.sources);
        let functions = std::mem::take(&mut wire.functions)
            .into_iter()
            .map(|function| function.into_facts(&mut sources, &mut inventories))
            .collect::<Result<_, _>>()?;
        sources.finish()?;
        inventories.finish()?;

        Ok(Decoded { wire, functions })
    }
}

impl UnitStore {
    /// Stream shared records directly; no frontend/proof trees are expanded.
    pub fn write_bundle(&self, writer: impl Write) -> Result<(), String> {
        if self.has_instance() {
            return Err("Instance requires original-owner and projection replay".into());
        }
        if !self.dependencies.is_empty() {
            return Err("Dependency closure requires separate owning bundle replay".into());
        }
        let sources = function_sources::encode(&self.functions)?;
        let export = Export {
            schema: BUNDLE_SCHEMA,
            namespace: &self.namespace,
            sources: sources.snapshots,
            inherent_units: sources.inventories,
            functions: sources.functions,
            helpers: &self.helpers,
            graphs: &self.graphs,
            roots: &self.roots,
            links: self.links.iter().map(export_links).collect(),
            certified: &self.certified,
        };

        serde_json::to_writer(writer, &export).map_err(|error| error.to_string())
    }

    /// Decode one exact byte snapshot, check references and strict evidence.
    /// Imported records never inherit the ambient generated verification policy.
    pub fn from_bundle(bytes: &[u8]) -> Result<Self, String> {
        Self::from_bundle_with(bytes, |_| Ok(()))
    }

    /// Inspect borrowed raw frontend records against an independently selected
    /// unit before strict evidence validation, without decoding the bytes again.
    pub fn from_bundle_with(
        bytes: &[u8],
        inspect: impl FnOnce(&[HirFnFacts]) -> Result<(), String>,
    ) -> Result<Self, String> {
        Self::load_bundle(bytes, inspect, false)
    }

    /// Load explicitly trusted current-run compiler output. The caller must
    /// independently freeze the producer and inspect selected-unit context.
    /// This choice is an application capability, never a serialized flag.
    pub fn from_current_run(
        bytes: &[u8],
        manifest: &BundleManifest,
        inspect: impl FnOnce(&[HirFnFacts]) -> Result<(), String>,
    ) -> Result<Self, String> {
        manifest.verify(bytes)?;
        Self::load_bundle(bytes, inspect, true)
    }

    /// Import a sealed immutable snapshot without rehashing its bytes. The
    /// current-run trust and independent selected-unit inspection requirements
    /// are identical to `from_current_run`; this token only proves the seal.
    pub fn from_verified_current_run(
        bundle: &VerifiedBundle,
        inspect: impl FnOnce(&[HirFnFacts]) -> Result<(), String>,
    ) -> Result<Self, String> {
        Self::load_bundle(bundle.bytes(), inspect, true)
    }

    fn load_bundle(
        bytes: &[u8],
        inspect: impl FnOnce(&[HirFnFacts]) -> Result<(), String>,
        current_run: bool,
    ) -> Result<Self, String> {
        let import = crate::diagnostics::Stage::new("unit.decode", "bundle", None)
            .observe(|| Import::decode(bytes))?;
        crate::diagnostics::Stage::new("unit.inspect", "bundle", None)
            .observe(|| inspect(&import.functions))?;

        let store = crate::diagnostics::Stage::new("unit.structure", "bundle", None)
            .observe(|| import.into_store())?;
        if current_run {
            crate::verification_policy::generated(|| {
                crate::diagnostics::Stage::new("unit.verify", "bundle", None)
                    .observe(|| store.validate_evidence())
            })?;
        } else {
            crate::verification_policy::strict(|| {
                crate::diagnostics::Stage::new("unit.verify", "bundle", None)
                    .observe(|| store.validate_evidence())
            })?;
        }
        Ok(store)
    }

    fn validate_evidence(&self) -> Result<(), String> {
        let analysis = self.analysis();

        for index in 0..self.helpers.len() {
            let helper = HelperView {
                foreign: false,
                resolver: &analysis,
                id: local_id(index, HelperId)?,
            };
            crate::helper_calls::verify_shared_callee(&helper, &helper.frontend())
                .map_err(|error| format!("Imported helper {}: {error}", helper.frontend().name))?;
            helper.remember();
        }
        self.validate_root_evidence(&analysis)?;
        for (index, certified) in self.certified.iter().enumerate() {
            if certified.is_none()
                && let Some(view) = analysis.function(index)
                && crate::extraction_profile::ExtractionProfile::from_facts(&view)?
                    .supports("rust.verified-helper-calls")
                && crate::classify(&view).status == crate::EligibilityVerdict::Eligible
                && crate::helper_calls::certify_view(&view)
                    .map_err(|error| format!("Imported function {}: {error}", view.name))?
                    .is_some()
            {
                return Err("Incomplete unit helper certification inventory".into());
            }
        }
        Ok(())
    }
    fn validate_root_evidence(&self, analysis: &UnitAnalysis<'_>) -> Result<(), String> {
        for (index, root) in self.roots.iter().enumerate() {
            let view = analysis.function(index).ok_or("Missing graph frontend")?;
            let eligible = crate::classify(&view).status == crate::EligibilityVerdict::Eligible;
            if root.is_some() != eligible {
                return Err("Incomplete or contradictory root graph inventory".into());
            }
            if self.certified[index].is_none()
                && let Some(pair) = self.pair(index)
            {
                crate::producer::verify(pair, &view).map_err(|errors| errors.join("; "))?;
            }
        }
        Ok(())
    }
}

fn export_links(links: &BTreeMap<Site, FunctionId>) -> Vec<CallLink> {
    links
        .iter()
        .map(|(site, function)| CallLink {
            site: *site,
            function: *function,
        })
        .collect()
}

impl Decoded {
    fn into_store(self) -> Result<UnitStore, String> {
        let store = Builder::in_namespace(self.functions, self.wire.namespace)?.store;

        if self.wire.certified.len() != store.functions.len() {
            return Err("Shared unit certification table disagrees".into());
        }
        if self.wire.links.len() != store.links.len() {
            return Err("Incomplete unit call link inventory".into());
        }
        for (wire, expected) in self.wire.links.into_iter().zip(&store.links) {
            validate_links(wire, expected)?;
        }
        u32::try_from(self.wire.helpers.len()).map_err(|_| "Oversized helper table")?;
        u32::try_from(self.wire.graphs.len()).map_err(|_| "Oversized graph table")?;
        let function_count = store.functions.len();
        let mut store = UnitStore {
            namespace: store.namespace,
            functions: store.functions,
            links: store.links,
            graphs: self.wire.graphs,
            roots: self.wire.roots,
            helpers: self.wire.helpers,
            certified: self.wire.certified,
            dependencies: Vec::new(),
            namespace_associations: Vec::new(),
            external_links: vec![BTreeMap::new(); function_count],
            instance: None,
            reference_instance: None,
            reference_forwarder: None,
            product_instance: None,
        };
        store.validate_roots()?;
        store.validate_certifications()?;
        store.validate_depths()?;
        graph_sources::share(&mut store);
        Ok(store)
    }
}

fn validate_links(
    wire: Vec<CallLink>,
    expected: &BTreeMap<Site, FunctionId>,
) -> Result<(), String> {
    let mut actual = BTreeMap::new();
    for link in wire {
        if actual.insert(link.site, link.function).is_some() {
            return Err("Duplicate unit helper call occurrence".into());
        }
    }
    if &actual != expected {
        return Err("Dangling, foreign or contradictory unit call link".into());
    }
    Ok(())
}

impl UnitStore {
    fn validate_roots(&self) -> Result<(), String> {
        if self.roots.len() != self.functions.len() {
            return Err("Incomplete unit graph root inventory".into());
        }
        let mut used = BTreeSet::new();
        for id in self.roots.iter().flatten() {
            if self.graphs.get(id.0 as usize).is_none() || !used.insert(*id) {
                return Err("Dangling or contradictory graph root reference".into());
            }
        }

        if used.len() != self.graphs.len() {
            return Err("Unreferenced unit graph record".into());
        }
        for helper in &self.helpers {
            if self.roots.get(helper.function.0 as usize) != Some(&Some(helper.graph)) {
                return Err("Helper graph disagrees with its function root".into());
            }
        }
        Ok(())
    }

    fn validate_certifications(&self) -> Result<(), String> {
        for (index, helper) in self.helpers.iter().enumerate() {
            let certified = self
                .certified
                .get(helper.function.0 as usize)
                .ok_or("Dangling helper frontend reference")?;
            if *certified != Some(local_id(index, HelperId)?) {
                return Err("Contradictory helper certification reference".into());
            }
        }
        for (index, id) in self.certified.iter().enumerate() {
            if let Some(id) = id {
                let helper = self
                    .helpers
                    .get(id.0 as usize)
                    .ok_or("Dangling certified helper reference")?;
                if helper.function.0 as usize != index {
                    return Err("Wrong-function helper certification reference".into());
                }
            }
        }
        Ok(())
    }

    fn validate_depths(&self) -> Result<(), String> {
        let mut depths = vec![None; self.helpers.len()];
        for _round in 0..=crate::helper_calls::MAX_DEPTH {
            for (index, helper) in self.helpers.iter().enumerate() {
                if depths[index].is_none() {
                    depths[index] = self.resolved_depth(helper.function, &depths)?;
                }
            }
        }
        for (helper, actual) in self.helpers.iter().zip(depths) {
            if actual != Some(helper.depth)
                || helper.depth as usize > crate::helper_calls::MAX_DEPTH
            {
                return Err("Cyclic, contradictory or over-budget helper dependency depth".into());
            }
        }
        Ok(())
    }

    fn resolved_depth(
        &self,
        function: FunctionId,
        depths: &[Option<u32>],
    ) -> Result<Option<u32>, String> {
        let mut depth = 0;
        for target in self.links[function.0 as usize].values() {
            let id = self.certified[target.0 as usize]
                .ok_or("Uncertified imported helper dependency")?;
            let Some(callee) = depths[id.0 as usize] else {
                return Ok(None);
            };
            depth = depth.max(
                callee
                    .checked_add(1)
                    .ok_or("Helper dependency depth overflow")?,
            );
        }
        Ok(Some(depth))
    }
}
