//! Exact builtin derive ancestry and the complete bounded generated member graph.
use super::*;

pub(super) struct Graph<'a> {
    nodes: BTreeMap<&'a str, &'a Expansion>,
    children: BTreeMap<&'a str, Vec<&'a Expansion>>,
    selected: BTreeSet<&'a str>,
}

impl<'a> Graph<'a> {
    pub(super) fn new(
        projection: &Projection,
        generated: &'a capture::UnitCapture,
        root: &Expansion,
    ) -> Result<Self, ProofError> {
        let mut graph = Self::index(generated)?;
        if graph.nodes.get(root.expansion.as_str()).copied() != Some(root) || root.parent.is_some()
        {
            return Err(ProofError::Contradictory(
                "Generated roster defining expansion differs",
            ));
        }
        let root = graph.nodes[root.expansion.as_str()];
        graph.selected.insert(root.expansion.as_str());

        graph.attributes(projection, root)?;
        graph.reject_detached(projection)?;
        Ok(graph)
    }

    fn index(generated: &'a capture::UnitCapture) -> Result<Self, ProofError> {
        let mut nodes = BTreeMap::new();
        let mut children: BTreeMap<&str, Vec<&Expansion>> = BTreeMap::new();
        for node in &generated.expansions {
            if node.expansion.is_empty() || nodes.insert(node.expansion.as_str(), node).is_some() {
                return Err(ProofError::Contradictory(
                    "Repeated generated expansion identity",
                ));
            }
            if let Some(parent) = &node.parent {
                children.entry(parent).or_default().push(node);
            }
        }

        Ok(Self {
            nodes,
            children,
            selected: BTreeSet::new(),
        })
    }

    fn attributes(
        &mut self,
        projection: &Projection,
        root: &'a Expansion,
    ) -> Result<(), ProofError> {
        let attributes = self
            .children
            .get(root.expansion.as_str())
            .cloned()
            .unwrap_or_default();

        if attributes.len() != projection.units().len() {
            return Err(ProofError::Contradictory(
                "Generated unit derive attribute roster differs",
            ));
        }

        let mut core = BTreeMap::new();
        for attribute in attributes {
            node(
                projection,
                root,
                attribute,
                (
                    capture::ExpansionKind::Attr,
                    "derive",
                    "core::derive",
                    TemplateRole::DeriveAttribute,
                ),
            )?;
            self.selected.insert(attribute.expansion.as_str());
            core_identity(&mut core, attribute)?;
            self.derive_triple(projection, attribute, &mut core)?;
        }
        Ok(())
    }

    fn derive_triple(
        &mut self,
        projection: &Projection,
        attribute: &Expansion,
        core: &mut BTreeMap<&'a str, &'a Definition>,
    ) -> Result<(), ProofError> {
        let root = self.nodes[attribute.parent.as_deref().unwrap()];
        let derives = self
            .children
            .get(attribute.expansion.as_str())
            .map(Vec::as_slice)
            .unwrap_or_default();

        for (index, child) in derives.iter().enumerate() {
            derive_node(projection, root, child)?;
            if derives[..index]
                .iter()
                .any(|other| other.name == child.name)
                || self.children.contains_key(child.expansion.as_str())
            {
                return Err(ProofError::Contradictory(
                    "Generated unit derive has surplus descendants",
                ));
            }
            select_derive(&mut self.selected, core, child)?;
        }

        let names = derives
            .iter()
            .map(|child| child.name.as_str())
            .collect::<BTreeSet<_>>();
        if names != BTreeSet::from(["Debug", "Clone", "Copy"]) {
            return Err(ProofError::Contradictory(
                "Generated unit derive triple is incomplete",
            ));
        }
        Ok(())
    }

    fn reject_detached(&self, projection: &Projection) -> Result<(), ProofError> {
        for node in self.nodes.values() {
            let raw = &node.invocation.raw;
            if !self.selected.contains(node.expansion.as_str())
                && raw.path == projection.full_definition().path
                && raw.span.start >= projection.full_definition().span.start
                && raw.span.end <= projection.full_definition().span.end
            {
                return Err(ProofError::Contradictory(
                    "Unchecked detached generated-unit expansion",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn contains(&self, identity: &str) -> bool {
        self.selected.contains(identity)
    }

    fn get(&self, identity: &str, name: &str) -> Result<&'a Expansion, ProofError> {
        let node = self
            .nodes
            .get(identity)
            .copied()
            .ok_or(ProofError::Missing(
                "Missing generated unit derive expansion",
            ))?;
        if !self.contains(identity)
            || node.name != name
            || node.kind != capture::ExpansionKind::Derive
        {
            return Err(ProofError::Contradictory(
                "Generated unit derive selection differs",
            ));
        }
        Ok(node)
    }
}

fn select_derive<'a>(
    selected: &mut BTreeSet<&'a str>,
    core: &mut BTreeMap<&'a str, &'a Definition>,
    child: &'a Expansion,
) -> Result<(), ProofError> {
    core_identity(core, child)?;
    selected.insert(child.expansion.as_str());
    Ok(())
}

fn derive_node(
    projection: &Projection,
    root: &Expansion,
    child: &Expansion,
) -> Result<(), ProofError> {
    let (path, role) = match child.name.as_str() {
        "Debug" => ("core::fmt::Debug", TemplateRole::DeriveDebug),
        "Clone" => ("core::clone::Clone", TemplateRole::DeriveClone),
        "Copy" => ("core::marker::Copy", TemplateRole::DeriveCopy),
        _ => return Err(ProofError::Unsupported("Unexpected generated unit derive")),
    };

    node(
        projection,
        root,
        child,
        (capture::ExpansionKind::Derive, &child.name, path, role),
    )
}

fn node(
    projection: &Projection,
    root: &Expansion,
    node: &Expansion,
    (kind, name, path, role): (capture::ExpansionKind, &str, &str, TemplateRole),
) -> Result<(), ProofError> {
    let Some(definition) = &node.macro_definition else {
        return Err(ProofError::Missing(
            "Generated unit derive lacks compiler identity",
        ));
    };
    if node.kind != kind
        || node.name != name
        || node.definition.is_some()
        || node.edition != root.edition
        || node.local_inner_macros
        || node.invocation.raw != *projection.template(role)
        || node.invocation.marks != [mark(root, Transparency::SemiOpaque)]
        || definition.local
        || definition.crate_name != "core"
        || definition.path != path
        || definition.stable_crate_id.is_empty()
        || definition.definition_identity.is_empty()
    {
        return Err(ProofError::Contradictory(
            "Generated unit builtin derive identity/source differs",
        ));
    }
    Ok(())
}

fn core_identity<'a>(
    identities: &mut BTreeMap<&'a str, &'a Definition>,
    node: &'a Expansion,
) -> Result<(), ProofError> {
    let definition = node.macro_definition.as_ref().unwrap();
    if identities
        .values()
        .any(|other| other.stable_crate_id != definition.stable_crate_id)
        || identities
            .insert(&node.name, definition)
            .is_some_and(|other| other != definition)
    {
        return Err(ProofError::Contradictory(
            "Generated unit core identities differ across derives",
        ));
    }
    Ok(())
}

impl Context<'_, '_, '_> {
    fn lifetime(
        &mut self,
        path: String,
        parent: &Definition,
        role: TemplateRole,
    ) -> Result<(), ProofError> {
        let mut source = self.projection.template(role).clone();
        source.span.end = source.span.start + 1;
        self.inventory.take(Expected {
            kind: "LifetimeParam",
            path,
            parent,
            source: &source,
            mark: &self.mark,
        })?;
        Ok(())
    }
}

pub(super) fn units<'g>(
    projection: &Projection,
    inventory: &mut Inventory<'g>,
    graph: &Graph<'g>,
    module: &Definition,
) -> Result<(), ProofError> {
    let mut roster = Roster {
        projection,
        inventory,
        graph,
        module,
    };
    roster.all()
}

struct Roster<'a, 'p, 'g> {
    projection: &'p Projection,
    inventory: &'a mut Inventory<'g>,
    graph: &'a Graph<'g>,
    module: &'p Definition,
}

impl<'g> Roster<'_, '_, 'g> {
    fn all(&mut self) -> Result<(), ProofError> {
        let mut attributes = BTreeSet::new();
        for unit in self.projection.units() {
            let parent = self.unit(unit.name())?;
            if !attributes.insert(parent) {
                return Err(ProofError::Contradictory(
                    "Generated units share or mix derive attribute ownership",
                ));
            }
        }
        Ok(())
    }

    fn unit(&mut self, unit: &str) -> Result<&'g String, ProofError> {
        let name = path(self.projection, unit);
        let mut parents = BTreeSet::new();
        for (derive, trait_path, role) in [
            ("Debug", "core::fmt::Debug", TemplateRole::DeriveDebug),
            ("Clone", "core::clone::Clone", TemplateRole::DeriveClone),
            ("Copy", "core::marker::Copy", TemplateRole::DeriveCopy),
        ] {
            let owner = format!("<{name} as {trait_path}>");
            let item = self.inventory.peek(
                "Impl { of_trait: true }",
                &owner,
                self.projection.template(role),
            )?;
            let expansion = self.graph.get(&item.origin.marks[0].expansion, derive)?;
            parents.insert(expansion.parent.as_ref().unwrap());
            self.implementation(&name, derive, expansion)?;
        }

        if parents.len() != 1 {
            return Err(ProofError::Contradictory(
                "Generated units share or mix derive attribute ownership",
            ));
        }
        Ok(parents.into_iter().next().unwrap())
    }

    fn implementation(
        &mut self,
        name: &str,
        derive: &str,
        expansion: &Expansion,
    ) -> Result<(), ProofError> {
        let mut context = Context {
            projection: self.projection,
            inventory: self.inventory,
            mark: mark(expansion, Transparency::Opaque),
        };
        context.derived(name, self.module, derive)
    }
}

struct DerivedMethod<'a> {
    owner: &'a str,
    name: &'a str,
    role: TemplateRole,
}

impl Context<'_, '_, '_> {
    fn derived(&mut self, unit: &str, module: &Definition, derive: &str) -> Result<(), ProofError> {
        let (role, method, trait_path) = match derive {
            "Debug" => (TemplateRole::DeriveDebug, "fmt", "core::fmt::Debug"),
            "Clone" => (TemplateRole::DeriveClone, "clone", "core::clone::Clone"),
            _ => (TemplateRole::DeriveCopy, "", "core::marker::Copy"),
        };

        let owner = format!("<{unit} as {trait_path}>");
        let implementation = self.take(TemplateRecord {
            kind: "Impl { of_trait: true }",
            path: owner.clone(),
            parent: module,
            role,
        })?;
        if method.is_empty() {
            return Ok(());
        }

        self.derived_member(
            &DerivedMethod {
                owner: &owner,
                name: method,
                role,
            },
            &implementation.definition,
        )?;
        if derive == "Clone" {
            self.take(TemplateRecord {
                kind: "Impl { of_trait: true }",
                path: format!("<{unit} as core::clone::TrivialClone>"),
                parent: module,
                role,
            })?;
        }
        Ok(())
    }

    fn derived_member(
        &mut self,
        method: &DerivedMethod<'_>,
        implementation: &Definition,
    ) -> Result<(), ProofError> {
        let function = self.take(TemplateRecord {
            kind: "AssocFn",
            path: format!("{}::{}", method.owner, method.name),
            parent: implementation,
            role: method.role,
        })?;
        self.derived_lifetimes(method, &function.definition)
    }

    fn derived_lifetimes(
        &mut self,
        method: &DerivedMethod<'_>,
        function: &Definition,
    ) -> Result<(), ProofError> {
        let lifetime = format!("{}::{}::'_", method.owner, method.name);
        self.lifetime(lifetime.clone(), function, method.role)?;

        if method.name == "fmt" {
            self.lifetime(lifetime.clone(), function, method.role)?;
            self.take(TemplateRecord {
                kind: "LifetimeParam",
                path: lifetime,
                parent: function,
                role: method.role,
            })?;
        }
        Ok(())
    }
}
