//! Selected local byte-slice instances retain the original unit and callee owner.
use super::instances::{InstanceProof, OriginalInstance, capture_instance};
use super::*;
use crate::slice_length_instances::CheckedProjection;

pub(super) struct SliceInstanceRecord {
    pub(super) checked: CheckedProjection,
    origin: OriginalInstance<()>,
    generated: Option<Arc<crate::generated_sources::capture::UnitCapture>>,
}

impl SliceInstanceRecord {
    pub(super) fn original(&self) -> &HirFnFacts {
        let (owner, ordinal, ()) = self.origin.get();
        &owner.functions[ordinal]
    }

    pub(super) fn generated_source_capture(
        &self,
    ) -> Result<Option<Arc<crate::generated_sources::capture::UnitCapture>>, String> {
        Ok(self.generated.clone())
    }
}

struct Proof<'a> {
    owner: &'a UnitStore,
    ordinal: usize,
}

struct RequestedInstance {
    instance: Arc<UnitStore>,
    calls: Vec<CallSite>,
}

struct CallSite {
    caller: usize,
    site: Site,
}

impl InstanceProof for Proof<'_> {
    type Checked = CheckedProjection;
    type Argument = ();
    const LABEL: &'static str = "Checked slice-length instance";

    fn verify(&self, original: &HirFnFacts) -> Result<CheckedProjection, String> {
        let analysis = self.owner.analysis();
        let view = analysis
            .function(self.ordinal)
            .ok_or("Missing original slice owner ordinal")?;
        if !std::ptr::eq(view.facts(), original) {
            return Err("Slice projection input differs from its retained owning function".into());
        }
        crate::slice_length_instances::verify(&view, &self.owner.functions)
    }

    fn facts(checked: &CheckedProjection) -> &HirFnFacts {
        checked.facts()
    }

    fn argument(_: &CheckedProjection) {}

    fn install(
        self,
        store: &mut UnitStore,
        origin: OriginalInstance<()>,
        checked: CheckedProjection,
    ) -> Result<(), String> {
        let owner = Arc::clone(origin.get().0);
        let analysis = owner.analysis();
        let original = analysis
            .function(origin.get().1)
            .ok_or("Missing original slice-length source owner")?;
        let generated = original.generated_source_capture()?;

        link_constructor(store, &checked, &owner)?;
        store.dependencies.push(owner);
        store.slice_instance = Some(SliceInstanceRecord {
            checked,
            origin,
            generated,
        });
        Ok(())
    }
}

impl UnitStore {
    /// Attach only independently checked instances requested by actual local
    /// calls. Rejected observations leave the original excluded definitions in
    /// place; they never become partially supported bodies.
    pub fn with_slice_length_instances(self) -> Result<Self, String> {
        let selected = observed_definitions(&self);
        if selected.is_empty() {
            return Ok(self);
        }

        let owner = Arc::new(self);
        let mut instances = Vec::new();
        for ordinal in selected {
            let Ok(calls) = requested_calls(&owner, ordinal) else {
                continue;
            };
            if calls.is_empty() {
                continue;
            }
            if let Ok(instance) = Self::capture_slice_length_instance(Arc::clone(&owner), ordinal) {
                instances.push(RequestedInstance {
                    instance: Arc::new(instance),
                    calls,
                });
            }
        }
        if instances.is_empty() {
            return Arc::try_unwrap(owner)
                .map_err(|_| "Retained unexpected slice instance owner".into());
        }
        let selected = reusable_callers(&owner, &instances)?;
        let mut builder = recapture_builder(&owner, selected.is_some())?;
        for instance in instances {
            install_calls(&mut builder.store, instance);
        }
        builder.dependencies = dependency_analyses(&builder.store.dependencies);
        match selected {
            Some(selected) => builder.capture_functions(selected),
            None => builder.capture(),
        }
    }

    /// Verify a separate concrete body; no original inventory entry is replaced.
    pub fn capture_slice_length_instance(owner: Arc<Self>, ordinal: usize) -> Result<Self, String> {
        let original = Arc::clone(&owner);
        let captured = capture_instance(
            owner,
            ordinal,
            Proof {
                owner: &original,
                ordinal,
            },
        )?;
        require_certified(captured)
    }

    pub fn slice_length_instance_origin(&self) -> Option<(&Arc<Self>, usize)> {
        self.slice_instance.as_ref().map(|record| {
            let (owner, ordinal, ()) = record.origin.get();
            (owner, ordinal)
        })
    }
}

fn reusable_callers(
    owner: &UnitStore,
    instances: &[RequestedInstance],
) -> Result<Option<Vec<FunctionId>>, String> {
    if owner.instance.is_some()
        || owner.slice_instance.is_some()
        || owner.reference_instance.is_some()
        || owner.reference_forwarder.is_some()
        || owner.product_instance.is_some()
    {
        return Ok(None);
    }

    let reverse = reverse_callers(&owner.links);
    let seeds = instances
        .iter()
        .flat_map(|instance| instance.calls.iter().map(|call| call.caller));
    let selected = caller_closure(&reverse, seeds);
    if selected
        .iter()
        .any(|&index| owner.certified[index].is_some() || owner.roots[index].is_some())
    {
        return Ok(None);
    }

    selected
        .into_iter()
        .map(|index| local_id(index, FunctionId))
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn reverse_callers(links: &[BTreeMap<Site, FunctionId>]) -> Vec<Vec<usize>> {
    let mut reverse = vec![Vec::new(); links.len()];
    for (caller, targets) in links.iter().enumerate() {
        for target in targets.values() {
            reverse[target.0 as usize].push(caller);
        }
    }
    reverse
}

/// Every local consumer of a changed call boundary must be reconsidered once.
fn caller_closure(reverse: &[Vec<usize>], seeds: impl IntoIterator<Item = usize>) -> Vec<usize> {
    let mut affected = vec![false; reverse.len()];
    let mut work = std::collections::VecDeque::from_iter(seeds);
    while let Some(callee) = work.pop_front() {
        if std::mem::replace(&mut affected[callee], true) {
            continue;
        }
        work.extend(reverse[callee].iter().copied());
    }

    affected
        .into_iter()
        .enumerate()
        .filter_map(|(index, selected)| selected.then_some(index))
        .collect()
}

fn recapture_builder(owner: &UnitStore, retain: bool) -> Result<Builder, String> {
    if !retain {
        let mut builder = Builder::in_namespace(owner.functions.clone(), owner.namespace)?;
        builder.store.dependencies = owner.dependencies.clone();
        builder.store.external_links = owner.external_links.clone();
        builder.store.namespace_associations = owner.namespace_associations.clone();
        return Ok(builder);
    }

    Ok(Builder {
        store: retained_tables(owner),
        dependencies: Vec::new(),
        pending: Vec::new(),
        proofs: RefCell::default(),
        native_widths: RefCell::default(),
        derived: derived::contexts(owner.functions.len()),
        sources: derived::Sources::default(),
    })
}

// Called only after the affected closure has no existing helper or root pair.
// Proof caches and derived analyses are rebuilt, while dense table IDs stay valid.
fn retained_tables(owner: &UnitStore) -> UnitStore {
    UnitStore {
        namespace: owner.namespace,
        functions: owner.functions.clone(),
        helpers: owner.helpers.clone(),
        graphs: owner.graphs.clone(),
        roots: owner.roots.clone(),
        links: owner.links.clone(),
        certified: owner.certified.clone(),
        dependencies: owner.dependencies.clone(),
        namespace_associations: owner.namespace_associations.clone(),
        external_links: owner.external_links.clone(),
        instance: None,
        slice_instance: None,
        reference_instance: None,
        reference_forwarder: None,
        product_instance: None,
    }
}

fn link_constructor(
    store: &mut UnitStore,
    checked: &CheckedProjection,
    owner: &UnitStore,
) -> Result<(), String> {
    let resolution = crate::helper_calls::direct_resolution(checked.error_call())
        .ok_or("Slice length instance lacks its original constructor resolution")?;
    let identity = resolution["definition_identity"]
        .as_str()
        .ok_or("Slice length constructor lacks its original identity")?;
    let target = definitions(&owner.functions)?
        .get(identity)
        .copied()
        .ok_or("Slice length constructor is outside its original owning unit")?;
    if owner.certified[target.0 as usize].is_none() {
        return Err("Slice length constructor has no verified original helper".into());
    }

    store.links[0].remove(&site(checked.error_call()));
    store.external_links[0].insert(
        site(checked.error_call()),
        ExternalTarget {
            dependency: 0,
            function: target,
        },
    );
    Ok(())
}

fn observed_definitions(store: &UnitStore) -> Vec<usize> {
    let mut selected = Vec::new();
    for (ordinal, original) in store.functions.iter().enumerate() {
        if !crate::extraction_profile::facts_support(
            original,
            crate::slice_length_instances::CAPABILITY,
        ) {
            continue;
        }
        let Some(record) = original
            .build
            .frontend_options
            .get("rstyle.rust.shared-slice-instance-projections")
        else {
            continue;
        };
        if record["outcome"]["status"] == "observed" {
            selected.push(ordinal);
        }
    }
    selected
}

fn requested_calls(owner: &UnitStore, ordinal: usize) -> Result<Vec<CallSite>, String> {
    let original = &owner.functions[ordinal];
    let mut selected = Vec::new();
    for (index, facts) in owner.functions.iter().enumerate() {
        for call in crate::helper_calls::call_nodes(facts) {
            if !crate::slice_length_instances::requested_call(original, facts, call)? {
                continue;
            }
            selected.push(CallSite {
                caller: index,
                site: site(call),
            });
        }
    }
    Ok(selected)
}

fn install_calls(store: &mut UnitStore, requested: RequestedInstance) {
    let dependency = store.dependencies.len();
    install_sites(store, requested.calls, dependency);
    store.dependencies.push(requested.instance);
}

fn install_sites(store: &mut UnitStore, calls: Vec<CallSite>, dependency: usize) {
    for call in calls {
        let target = ExternalTarget {
            dependency,
            function: FunctionId(0),
        };
        store.links[call.caller].remove(&call.site);
        store.external_links[call.caller].insert(call.site, target);
    }
}

fn require_certified(captured: UnitStore) -> Result<UnitStore, String> {
    if matches!(captured.certified.first(), Some(Some(_))) {
        return Ok(captured);
    }
    let analysis = captured.analysis();
    let facts = analysis
        .function(0)
        .ok_or("Missing captured slice-length function")?;

    let verdict = crate::classify(&facts);
    let reasons = certification_reasons(&facts, &verdict.diagnostics);

    Err(format!(
        "Slice-length instance {} has no complete verified callable summary ({:?}): {}",
        facts.name,
        verdict.status,
        if reasons.is_empty() {
            "source, graph, conditional origins or helper boundary did not certify"
        } else {
            &reasons
        },
    ))
}

fn certification_reasons(
    facts: &dyn EvidenceFacts,
    diagnostics: &[crate::HirDiagnostic],
) -> String {
    let mut reasons = diagnostics
        .iter()
        .map(|diagnostic| {
            format!(
                "{} at {:?}: {}",
                diagnostic.construct, diagnostic.span, diagnostic.reason
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let mut methods = Vec::new();
    crate::frontend::collect_kind(&facts.body, "expr_method_call", &mut methods);

    for method in methods {
        if let Err(error) = crate::call_summaries::resolve_for(method, facts) {
            reasons.push_str(&format!("; method {:?}: {error}", method.span));
        }
    }
    if reasons.is_empty()
        && let Err(errors) = crate::producer::produce(facts)
    {
        reasons = errors.join("; ");
    }
    reasons
}

#[cfg(test)]
mod closure_tests {
    use super::{FunctionId, caller_closure, reverse_callers};
    use std::collections::BTreeMap;

    #[test]
    fn recapture_follows_diamond_consumers_and_leaves_disconnected_calls() {
        let reverse = vec![vec![1, 2], vec![3], vec![3], vec![], vec![5], vec![]];
        assert_eq!(caller_closure(&reverse, [0, 0]), vec![0, 1, 2, 3]);
        assert_eq!(caller_closure(&reverse, [4]), vec![4, 5]);
        assert!(caller_closure(&reverse, []).is_empty());
    }

    #[test]
    fn reverse_local_links_terminates_cycles_and_includes_transitive_callers() {
        let links = vec![
            BTreeMap::from([((0, 0, 1), FunctionId(1))]),
            BTreeMap::from([((0, 1, 1), FunctionId(0))]),
            BTreeMap::from([((0, 2, 1), FunctionId(1))]),
            BTreeMap::new(),
        ];
        assert_eq!(caller_closure(&reverse_callers(&links), [0]), vec![0, 1, 2]);
    }
}
