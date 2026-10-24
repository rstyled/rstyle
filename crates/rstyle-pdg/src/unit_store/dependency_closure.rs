//! Explicit dependency ownership, established only after the caller checks Cargo authority.
use super::*;
use serde_json::Value;

const ASSOCIATION: &str = "rstyle.rust.compiled-dependency-association/1";
const COMPILED: &str = "rstyle.rust.compiled-crate";

/// Original unit ordinals remain stable. Foreign bodies stay in their own Arc owner.
pub struct DependencyClosure {
    units: Vec<Arc<UnitStore>>,
}

impl DependencyClosure {
    /// The checker must independently reconstruct graph, invocation and emitted/loaded
    /// artifact associations. A serialized edge or compiler observation alone is not authority.
    /// Strict replay uses the same API after strict loading of the original owning bundles.
    pub fn from_units_with(
        units: Vec<UnitStore>,
        associations: &[Value],
        verify: impl FnOnce(&[UnitStore], &[Value]) -> Result<(), String>,
    ) -> Result<Self, String> {
        verify(&units, associations)?;
        let plan = Plan::checked(&units, associations)?;

        let mut pending: Vec<_> = units.into_iter().map(Some).collect();
        let mut completed: Vec<Option<Arc<UnitStore>>> = (0..pending.len()).map(|_| None).collect();
        for index in plan.order()? {
            let store = pending[index]
                .take()
                .ok_or("Missing closure source owner")?;
            completed[index] = Some(Arc::new(rebuild(store, index, &plan, &completed)?));
        }

        let units = completed
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or("Cyclic or incomplete compiled dependency closure")?;
        Ok(Self { units })
    }

    pub fn unit(&self, ordinal: usize) -> Option<&Arc<UnitStore>> {
        self.units.get(ordinal)
    }

    pub fn units(&self) -> &[Arc<UnitStore>] {
        &self.units
    }
}

struct Edge<'a> {
    dependency: usize,
    callee: usize,
    observation: &'a Value,
}
struct Inputs<'a> {
    edges: Vec<BTreeMap<&'a str, Edge<'a>>>,
    definitions: Vec<BTreeMap<&'a str, FunctionId>>,
    units: &'a [UnitStore],
}

struct Call<'a> {
    caller: usize,
    facts: &'a HirFnFacts,
    node: &'a HirNode,
}

struct SelectedTarget<'a> {
    edge: &'a Edge<'a>,
    definition: FunctionId,
}

impl Inputs<'_> {
    fn selected(
        &self,
        caller: usize,
        node: &HirNode,
    ) -> Result<Option<SelectedTarget<'_>>, String> {
        let Some(resolution) = crate::helper_calls::direct_resolution(node) else {
            return Ok(None);
        };
        if resolution["local"] != false {
            return Ok(None);
        }
        let Some(stable_id) = resolution["compiled_dependency"]["stable_crate_id"].as_str() else {
            return Ok(None);
        };
        let Some(edge) = self.edges[caller].get(stable_id) else {
            return Ok(None);
        };
        if resolution["compiled_dependency"] != edge.observation["compiled_dependency"] {
            return Err("External call contradicts checked compiled dependency observation".into());
        }

        Ok(target(self, edge.callee, &resolution)?
            .map(|definition| SelectedTarget { edge, definition }))
    }
}

struct Plan {
    dependencies: Vec<Vec<usize>>,
    namespace_associations: Vec<Vec<Value>>,
    links: Vec<Vec<BTreeMap<Site, ExternalTarget>>>,
    instances: Vec<BTreeMap<super::instance_links::Key, usize>>,
}

struct Schedule {
    remaining: Vec<usize>,
    callers: Vec<Vec<usize>>,
    ready: std::collections::VecDeque<usize>,
    order: Vec<usize>,
}

impl Schedule {
    fn new(dependencies: &[Vec<usize>]) -> Self {
        let remaining: Vec<_> = dependencies.iter().map(Vec::len).collect();
        let mut callers = vec![Vec::new(); remaining.len()];
        for (caller, dependencies) in dependencies.iter().enumerate() {
            for dependency in dependencies {
                callers[*dependency].push(caller);
            }
        }

        let ready = remaining
            .iter()
            .enumerate()
            .filter_map(|(index, count)| (*count == 0).then_some(index))
            .collect();
        let order = Vec::with_capacity(remaining.len());
        Self {
            remaining,
            callers,
            ready,
            order,
        }
    }

    fn complete(mut self) -> Result<Vec<usize>, String> {
        while let Some(unit) = self.ready.pop_front() {
            self.order.push(unit);
            self.advance(unit);
        }
        if self.order.len() != self.remaining.len() {
            return Err("Cyclic compiled dependency closure".into());
        }
        Ok(self.order)
    }

    fn advance(&mut self, unit: usize) {
        for caller in &self.callers[unit] {
            self.remaining[*caller] -= 1;
            if self.remaining[*caller] == 0 {
                self.ready.push_back(*caller);
            }
        }
    }
}

impl Plan {
    fn order(&self) -> Result<Vec<usize>, String> {
        Schedule::new(&self.dependencies).complete()
    }

    fn checked(units: &[UnitStore], associations: &[Value]) -> Result<Self, String> {
        unique_namespaces(units)?;
        let mut plan = Self {
            dependencies: vec![Vec::new(); units.len()],
            namespace_associations: vec![Vec::new(); units.len()],
            instances: vec![BTreeMap::new(); units.len()],
            links: units
                .iter()
                .map(|unit| vec![BTreeMap::new(); unit.functions.len()])
                .collect(),
        };

        let mut edges: Vec<BTreeMap<&str, Edge<'_>>> =
            (0..units.len()).map(|_| BTreeMap::new()).collect();
        for association in associations {
            let caller = ordinal(association, "caller", units)?;
            let callee = ordinal(association, "callee", units)?;
            check_association(association, &units[callee])?;
            let stable_id = association["compiled_dependency"]["stable_crate_id"]
                .as_str()
                .ok_or("Missing compiled dependency stable identity")?;
            let edge = Edge {
                dependency: plan.dependencies[caller].len(),
                callee,
                observation: association,
            };
            if caller == callee || edges[caller].insert(stable_id, edge).is_some() {
                return Err("Self or duplicate compiled dependency association".into());
            }
            plan.dependencies[caller].push(callee);
            plan.namespace_associations[caller].push(association["compiled_dependency"].clone());
        }

        let definitions = units
            .iter()
            .map(|unit| super::definitions(&unit.functions))
            .collect::<Result<_, _>>()?;
        let inputs = Inputs {
            edges,
            definitions,
            units,
        };
        for (caller, unit) in units.iter().enumerate() {
            for (function, facts) in unit.functions.iter().enumerate() {
                plan.function(&inputs, (caller, function), facts)?;
            }
        }
        Ok(plan)
    }

    fn function(
        &mut self,
        inputs: &Inputs<'_>,
        position: (usize, usize),
        facts: &HirFnFacts,
    ) -> Result<(), String> {
        let (caller, function) = position;
        if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.verified-cross-unit-scalar-helpers")
        {
            return Ok(());
        }
        for node in crate::helper_calls::call_nodes(facts) {
            let Some(selected) = inputs.selected(caller, node)? else {
                continue;
            };
            let call = Call {
                caller,
                facts,
                node,
            };
            let Some(target) = self.call_target(inputs, call, selected)? else {
                continue;
            };
            if self.links[caller][function]
                .insert(site(node), target)
                .is_some()
            {
                return Err("Ambiguous external helper call association".into());
            }
        }
        Ok(())
    }

    fn call_target(
        &mut self,
        inputs: &Inputs<'_>,
        call: Call<'_>,
        selected: SelectedTarget<'_>,
    ) -> Result<Option<ExternalTarget>, String> {
        let SelectedTarget { edge, definition } = selected;
        let Some(argument) = super::instance_links::argument(call.facts, call.node)? else {
            return Ok(Some(ExternalTarget {
                dependency: edge.dependency,
                function: definition,
            }));
        };
        if !super::instance_links::available(&inputs.units[edge.callee], definition, argument)? {
            return Ok(None);
        }

        let caller = call.caller;
        let next = self.instances[caller].len();
        let slot = *self.instances[caller]
            .entry((edge.callee, definition, argument))
            .or_insert(next);
        Ok(Some(ExternalTarget {
            dependency: self.dependencies[caller].len() + slot,
            function: FunctionId(0),
        }))
    }
}

fn target(
    inputs: &Inputs<'_>,
    unit: usize,
    resolution: &Value,
) -> Result<Option<FunctionId>, String> {
    let identity = resolution["definition_identity"]
        .as_str()
        .ok_or("Missing external helper definition identity")?;
    let Some(target) = inputs.definitions[unit].get(identity).copied() else {
        return Ok(None);
    };
    Ok(Some(target))
}

fn ordinal(edge: &Value, side: &str, units: &[UnitStore]) -> Result<usize, String> {
    let index = edge[side]["unit"]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .filter(|index| *index < units.len())
        .ok_or("Invalid association unit ordinal")?;
    let namespace: UnitNamespace = serde_json::from_value(edge[side]["namespace"].clone())
        .map_err(|error| error.to_string())?;
    if namespace != units[index].namespace {
        return Err("Association namespace disagrees".into());
    }
    Ok(index)
}

fn check_association(edge: &Value, callee: &UnitStore) -> Result<(), String> {
    if edge["identity"] != ASSOCIATION || callee.functions.is_empty() {
        return Err("Unsupported or empty compiled dependency association".into());
    }
    for facts in &callee.functions {
        if facts.build.frontend_options.get(COMPILED) != Some(&edge["local_compiled_crate"]) {
            return Err("Compiled association disagrees with callee owner identity".into());
        }
    }

    let local = &edge["local_compiled_crate"];
    let foreign = &edge["compiled_dependency"];
    if local["identity"] != "rstyle.rust.compiled-crate/1"
        || ["identity", "crate_name", "stable_crate_id"]
            .iter()
            .any(|key| {
                local[*key].as_str().is_none_or(str::is_empty) || local[*key] != foreign[*key]
            })
        || foreign["instance"]["type_arguments"]
            .as_array()
            .is_none_or(|args| !args.is_empty())
        || foreign["instance"]["const_arguments"]
            .as_array()
            .is_none_or(|args| !args.is_empty())
    {
        return Err("Incomplete or contradictory compiled dependency identity".into());
    }
    Ok(())
}

fn unique_namespaces(units: &[UnitStore]) -> Result<(), String> {
    for (index, unit) in units.iter().enumerate() {
        if unit.has_instance() {
            return Err("Instances require checked caller-instance associations".into());
        }
        if !unit.dependencies.is_empty()
            || units[..index]
                .iter()
                .any(|other| other.namespace == unit.namespace)
        {
            return Err("Duplicate namespace or already linked closure owner".into());
        }
    }
    Ok(())
}

fn rebuild(
    store: UnitStore,
    index: usize,
    plan: &Plan,
    completed: &[Option<Arc<UnitStore>>],
) -> Result<UnitStore, String> {
    if plan.dependencies[index].is_empty() {
        return Ok(store);
    }

    let mut builder = Builder::in_namespace(store.functions, store.namespace)?;
    install_dependency_owners(&mut builder, index, plan, completed)?;

    builder.store.external_links = plan.links[index].clone();
    builder.dependencies = dependency_analyses(&builder.store.dependencies);
    builder.capture()
}

fn install_dependency_owners(
    builder: &mut Builder,
    index: usize,
    plan: &Plan,
    completed: &[Option<Arc<UnitStore>>],
) -> Result<(), String> {
    builder.store.dependencies = linked_dependencies(index, plan, completed)?;
    builder.store.namespace_associations = plan.namespace_associations[index]
        .iter()
        .enumerate()
        .map(
            |(dependency, observation)| dependency_namespaces::Association {
                dependency,
                compiled_dependency: observation.clone(),
            },
        )
        .collect();

    Ok(())
}

fn linked_dependencies(
    index: usize,
    plan: &Plan,
    completed: &[Option<Arc<UnitStore>>],
) -> Result<Vec<Arc<UnitStore>>, String> {
    let mut dependencies = plan.dependencies[index]
        .iter()
        .map(|target| {
            completed[*target]
                .clone()
                .ok_or_else(|| "Missing completed dependency owner".into())
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut instances = vec![None; plan.instances[index].len()];

    for ((unit, definition, argument), slot) in &plan.instances[index] {
        let owner = Arc::clone(
            completed[*unit]
                .as_ref()
                .ok_or("Missing original instance owner")?,
        );
        instances[*slot] = Some(
            super::instance_links::capture(owner, *definition, *argument)?
                .ok_or("Projected scalar instance became unavailable during immutable replay")?,
        );
    }
    dependencies.extend(
        instances
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or("Incomplete scalar instance dependency inventory")?,
    );
    Ok(dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn dependency_schedule_orders_a_diamond_and_rejects_cycles() {
        let plan = Plan {
            dependencies: vec![vec![1, 2], vec![3], vec![3], vec![]],
            namespace_associations: Vec::new(),
            links: Vec::new(),
            instances: Vec::new(),
        };
        assert_eq!(plan.order().unwrap(), vec![3, 1, 2, 0]);
        let cycle = Plan {
            dependencies: vec![vec![1], vec![0]],
            namespace_associations: Vec::new(),
            links: Vec::new(),
            instances: Vec::new(),
        };
        assert!(cycle.order().unwrap_err().contains("Cyclic"));
    }

    #[test]
    fn diamond_dependency_proofs_share_one_owner_per_analysis() {
        let leaf = Arc::new(UnitStore::capture(Vec::new()).unwrap());
        let mut left = UnitStore::capture(Vec::new()).unwrap();
        left.dependencies.push(Arc::clone(&leaf));
        let mut right = UnitStore::capture(Vec::new()).unwrap();
        right.dependencies.push(leaf);
        let mut root = UnitStore::capture(Vec::new()).unwrap();
        root.dependencies = vec![Arc::new(left), Arc::new(right)];
        let first = root.analysis();
        let shared = &first.dependencies[0].dependencies[0];
        assert!(Rc::ptr_eq(shared, &first.dependencies[1].dependencies[0]));
        let independent = root.analysis();
        assert!(!Rc::ptr_eq(
            shared,
            &independent.dependencies[0].dependencies[0]
        ));
        assert!(root.write_bundle(Vec::new()).is_err());
    }

    #[test]
    fn authority_rejection_precedes_untrusted_association_parsing() {
        let called = Cell::new(false);
        let result = DependencyClosure::from_units_with(
            vec![UnitStore::capture(Vec::new()).unwrap()],
            &[serde_json::json!({"caller":"forged"})],
            |_, _| {
                called.set(true);
                Err("independent Cargo artifact check failed".into())
            },
        );
        assert!(called.get());
        assert_eq!(
            result.err().unwrap(),
            "independent Cargo artifact check failed"
        );
    }

    #[test]
    fn duplicate_namespace_cannot_create_an_alternative_foreign_owner() {
        let first = UnitStore::capture(Vec::new()).unwrap();
        let mut bytes = Vec::new();
        first.write_bundle(&mut bytes).unwrap();
        let duplicate = UnitStore::from_bundle(&bytes).unwrap();
        let result = DependencyClosure::from_units_with(vec![first, duplicate], &[], |_, _| Ok(()));
        assert!(result.err().unwrap().contains("Duplicate namespace"));
    }

    #[test]
    fn unassociated_owners_keep_ordinals_and_plain_bundle_replay() {
        let first = UnitStore::capture(Vec::new()).unwrap();
        let second = UnitStore::capture(Vec::new()).unwrap();
        let expected = [first.namespace, second.namespace];
        let closure = DependencyClosure::from_units_with(vec![first, second], &[], |_, edges| {
            assert!(edges.is_empty());
            Ok(())
        })
        .unwrap();
        assert_eq!(closure.units().len(), 2);
        for (index, namespace) in expected.iter().enumerate() {
            let store = closure.unit(index).unwrap();
            assert_eq!(store.namespace(), namespace);
            let mut bytes = Vec::new();
            store.write_bundle(&mut bytes).unwrap();
            assert_eq!(
                UnitStore::from_bundle(&bytes).unwrap().namespace(),
                namespace
            );
        }
        assert!(closure.unit(2).is_none());
    }

    #[test]
    fn association_namespace_must_match_the_selected_ordinal() {
        let units = [
            UnitStore::capture(Vec::new()).unwrap(),
            UnitStore::capture(Vec::new()).unwrap(),
        ];
        let edge = serde_json::json!({"caller":{"unit":0,"namespace":units[1].namespace}});
        assert!(
            ordinal(&edge, "caller", &units)
                .unwrap_err()
                .contains("namespace")
        );
        let edge = serde_json::json!({"caller":{"unit":2,"namespace":units[0].namespace}});
        assert!(
            ordinal(&edge, "caller", &units)
                .unwrap_err()
                .contains("ordinal")
        );
    }
}
