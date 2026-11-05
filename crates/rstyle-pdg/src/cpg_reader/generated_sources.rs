//! The unit's generated-source capture, rebuilt from what its documents record: the
//! expansions and definitions of its local bang macros, and each generated function's
//! owner, implementation and node sites. The capture selects and walks them exactly as
//! the frontend's own pass does, so its expansion closure and outcomes are the same.

use super::unit::Documents;
use crate::generated_sources::capture::{
    Definition, Emitted, Expansion, ExpansionKind, IDENTITY, Mark, NodeOrigin, Observation, Origin,
    Outcome, UNIT_IDENTITY, UnitCapture,
};
use crate::nominal_boundary::census::expansions::FileSpan;
use crate::{HirFnFacts, HirNode};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// The unit extension recording its macro expansions and generated definitions.
pub const UNIT_EXTENSION: &str = "rust:generated-sources";
/// The function extension recording a generated function's sites.
pub const FUNCTION_EXTENSION: &str = "rust:generated-source";
/// Requests the capture outside the profiles that read it.
pub const DIAGNOSTIC_ENV: &str = "RSTYLE_DIAGNOSTIC_GENERATED_SOURCES";

/// Where a span is, as the frontend locates it: a dummy span, a span it could not
/// locate, or its original file span with the expansion marks of its context.
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Site {
    Dummy {
        #[allow(dead_code)]
        dummy: bool,
    },
    Failed {
        error: String,
    },
    Located {
        raw: FileSpan,
        marks: Vec<Mark>,
    },
}

/// A value the frontend observed, or why it could not.
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Fallible<T> {
    Failed { error: String },
    Observed(T),
}

impl<T: Clone> Fallible<T> {
    fn get(&self) -> Result<T, String> {
        match self {
            Self::Failed { error } => Err(error.clone()),
            Self::Observed(value) => Ok(value.clone()),
        }
    }
}

/// One expansion as the compiler records it.
#[derive(Deserialize)]
pub(super) struct Record {
    parent: Option<String>,
    call_site: Site,
    /// The local macro definition's span; absent for a macro defined elsewhere.
    definition: Option<Fallible<FileSpan>>,
    kind: Fallible<ExpansionKind>,
    name: String,
    macro_definition: Option<Definition>,
    edition: String,
    local_inner_macros: bool,
}

/// A local definition with a local bang root, or whose root could not be found.
#[derive(Deserialize)]
struct Candidate {
    definition: Definition,
    root: Fallible<String>,
    parent: Option<Definition>,
    kind: Option<String>,
    defined: Option<String>,
    site: Option<Site>,
}

#[derive(Deserialize)]
struct Unit {
    expansions: BTreeMap<String, Record>,
    definitions: Vec<Candidate>,
}

/// A generated function's sites.
#[derive(Deserialize)]
pub(super) struct Function {
    root: Fallible<String>,
    owner: Definition,
    implementation: Definition,
    signature: Site,
    body: Site,
    nodes: BTreeMap<String, NodeSites>,
    expansions: BTreeMap<String, Record>,
}

/// A HIR node's sites: its own span, and for a binding without a subpattern its
/// identifier's, for an item statement its item's; or a node the capture cannot place.
#[derive(Deserialize)]
struct NodeSites {
    #[serde(default)]
    unsupported: bool,
    span: Option<Site>,
    ident: Option<Site>,
    item: Option<Site>,
}

/// Installs the unit's generated-source capture on its first generated function's
/// facts, or its first function's, when the profile reads it.
pub fn capture(
    documents: &Documents,
    profile: crate::extraction_profile::ExtractionProfile,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    if !profile.supports(crate::result_maps::CAPABILITY)
        && !crate::diagnostics::switch(DIAGNOSTIC_ENV)
    {
        return Ok(());
    }
    let functions = documents.generated_functions(facts)?;
    let mut roots = declaration_roots(facts)?;
    roots.extend(
        functions
            .iter()
            .filter_map(|(_, function)| function.root.get().ok()),
    );
    if functions.is_empty() && roots.is_empty() {
        return Ok(());
    }
    let unit: Unit = documents.observation(UNIT_EXTENSION, "generated sources")?;
    let capture = unit_capture(&unit, &functions, &roots, facts);
    let index = functions.first().map_or(0, |(index, _)| *index);

    facts[index]
        .build
        .frontend_options
        .insert_data(Arc::new(capture));
    Ok(())
}

/// The unit's capture: the selected expansions' emitted definitions and each generated
/// function's observation, with every expansion the walk visits.
fn unit_capture(
    unit: &Unit,
    functions: &[(usize, Function)],
    roots: &BTreeSet<String>,
    facts: &[HirFnFacts],
) -> UnitCapture {
    let mut records: BTreeMap<&str, &Record> = BTreeMap::new();
    for expansions in std::iter::once(&unit.expansions)
        .chain(functions.iter().map(|(_, function)| &function.expansions))
    {
        records.extend(expansions.iter().map(|(id, record)| (id.as_str(), record)));
    }
    let mut provenance = Provenance {
        records,
        expansions: BTreeMap::new(),
        visiting: BTreeSet::new(),
    };
    let (emitted, mut outcomes) = provenance.roster(&unit.definitions, roots);
    outcomes.extend(functions.iter().map(|(index, function)| {
        let observed = function.root.get().and_then(|root| {
            provenance.expansion(&root)?;
            provenance.observe(function, root, &facts[*index])
        });
        match observed {
            Ok(observation) => Outcome::Observed {
                observation: Box::new(observation),
            },
            Err(reason) => Outcome::Unsupported {
                owner: function.owner.clone(),
                reason,
            },
        }
    }));

    UnitCapture {
        identity: UNIT_IDENTITY.into(),
        complete: outcomes
            .iter()
            .all(|outcome| matches!(outcome, Outcome::Observed { .. })),
        scope: "selected-local-bang-expansion-closure".into(),
        outcomes,
        expansions: provenance.expansions.into_values().collect(),
        emitted,
    }
}

/// The expansions the unit's generated declarations come from.
fn declaration_roots(facts: &[HirFnFacts]) -> Result<BTreeSet<String>, String> {
    let mut roots = BTreeSet::new();
    for facts in facts {
        let options = &facts.build.frontend_options;
        if options.contains_key(crate::declaration_origins::KEY) {
            let origins = options.data::<crate::declaration_origins::Origins>()?;
            roots.extend(
                origins
                    .generated
                    .iter()
                    .map(|origin| origin.defining_expansion.clone()),
            );
        }
    }

    Ok(roots)
}

/// The provenance walk: each expansion it visits, recorded once its ancestry is.
struct Provenance<'a> {
    records: BTreeMap<&'a str, &'a Record>,
    expansions: BTreeMap<String, Expansion>,
    visiting: BTreeSet<String>,
}

const MAX_ROSTER: usize = 65536;
const MAX_NODES: usize = 32768;
const MAX_DEPTH: usize = 256;

impl Provenance<'_> {
    fn origin(&mut self, site: &Site) -> Result<Origin, String> {
        match site {
            Site::Dummy { .. } => Err("Generated provenance contains a dummy raw span".into()),
            Site::Failed { error } => Err(error.clone()),
            Site::Located { raw, marks } => {
                for mark in marks {
                    self.expansion(&mark.expansion)?;
                }
                Ok(Origin {
                    raw: raw.clone(),
                    marks: marks.clone(),
                })
            }
        }
    }

    // rstyle-allow(mutation_per_target): the walk marks an expansion visiting, then records it, as the frontend's does
    fn expansion(&mut self, id: &str) -> Result<(), String> {
        if self.expansions.contains_key(id) || self.visiting.contains(id) {
            return Ok(());
        }
        if self.visiting.len() >= 128 || self.expansions.len() >= 8192 {
            return Err("Generated expansion ancestry exceeds diagnostic budget".into());
        }
        self.visiting.insert(id.to_owned());
        let record = self.record(id);
        self.visiting.remove(id);
        let record = record?;

        self.expansions.insert(id.to_owned(), record);
        Ok(())
    }

    fn record(&mut self, id: &str) -> Result<Expansion, String> {
        let data = *self
            .records
            .get(id)
            .ok_or_else(|| format!("The unit records no expansion {id}"))?;
        if let Some(parent) = &data.parent {
            self.expansion(parent)?;
        }
        let invocation = self.origin(&data.call_site)?;
        let definition = data.definition.as_ref().map(Fallible::get).transpose()?;

        Ok(Expansion {
            expansion: id.to_owned(),
            parent: data.parent.clone(),
            kind: data.kind.get()?,
            name: data.name.clone(),
            macro_definition: data.macro_definition.clone(),
            definition,
            invocation,
            edition: data.edition.clone(),
            local_inner_macros: data.local_inner_macros,
        })
    }

    /// Every local definition in the selected expansions' closure, and each definition
    /// whose root could not be found.
    fn roster(
        &mut self,
        candidates: &[Candidate],
        selected: &BTreeSet<String>,
    ) -> (Vec<Emitted>, Vec<Outcome>) {
        if selected.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let (mut records, mut errors) = (Vec::new(), Vec::new());
        for candidate in candidates {
            let unsupported = |reason| Outcome::Unsupported {
                owner: candidate.definition.clone(),
                reason,
            };
            match candidate.root.get() {
                Err(reason) => errors.push(unsupported(reason)),
                Ok(root) if selected.contains(&root) => match self.emitted(candidate, &root) {
                    Ok(record) => records.push(record),
                    Err(reason) => errors.push(unsupported(reason)),
                },
                Ok(_) => {}
            }
            if records.len() + errors.len() >= MAX_ROSTER {
                errors.push(unsupported(
                    "Generated emitted-definition roster exceeds diagnostic budget".into(),
                ));
                break;
            }
        }
        records.sort_by(|left: &Emitted, right| {
            (&left.definition.path, &left.definition.definition_identity).cmp(&(
                &right.definition.path,
                &right.definition.definition_identity,
            ))
        });

        (records, errors)
    }

    fn emitted(&mut self, candidate: &Candidate, root: &str) -> Result<Emitted, String> {
        let missing = || format!("The unit records no site of {}", candidate.definition.path);
        self.expansion(root)?;
        // Keep the defining expansion even when a descendant's span has root context.
        if let Some(defined) = &candidate.defined {
            self.expansion(defined)?;
        }
        let site = candidate.site.as_ref().ok_or_else(missing)?;

        Ok(Emitted {
            definition: candidate.definition.clone(),
            parent: candidate.parent.clone().ok_or_else(missing)?,
            kind: candidate.kind.clone().ok_or_else(missing)?,
            origin: self.origin(site)?,
        })
    }

    fn observe(
        &mut self,
        function: &Function,
        root: String,
        facts: &HirFnFacts,
    ) -> Result<Observation, String> {
        Ok(Observation {
            identity: IDENTITY.into(),
            owner: function.owner.clone(),
            implementation: function.implementation.clone(),
            defining_expansion: root,
            signature: self.origin(&function.signature)?,
            body: self.origin(&function.body)?,
            nodes: self.nodes(function, facts)?,
        })
    }

    /// Each node of the function's parameters, then of its body, at its tree path.
    fn nodes(
        &mut self,
        function: &Function,
        facts: &HirFnFacts,
    ) -> Result<Vec<NodeOrigin>, String> {
        let mut walk = Walk {
            provenance: self,
            function,
            records: Vec::new(),
        };
        for (index, parameter) in facts.signature.params.iter().enumerate() {
            let index =
                u32::try_from(index).map_err(|_| "Generated parameter index exceeds u32")?;
            walk.walk(&parameter.pat, &mut vec![0, index])?;
        }
        walk.walk(&facts.body, &mut vec![1])?;

        Ok(walk.records)
    }
}

struct Walk<'p, 'a, 'f> {
    provenance: &'p mut Provenance<'a>,
    function: &'f Function,
    records: Vec<NodeOrigin>,
}

impl Walk<'_, '_, '_> {
    fn walk(&mut self, node: &HirNode, path: &mut Vec<u32>) -> Result<(), String> {
        if self.records.len() >= MAX_NODES || path.len() > MAX_DEPTH {
            return Err("Generated facts role inventory exceeds diagnostic budget".into());
        }
        self.record(node, path)?;
        for (index, child) in node.children.iter().enumerate() {
            path.push(u32::try_from(index).map_err(|_| "Generated child index exceeds u32")?);
            self.walk(child, path)?;
            path.pop();
        }

        Ok(())
    }

    fn record(&mut self, node: &HirNode, path: &[u32]) -> Result<(), String> {
        if node.hir.krate != 0 {
            return Err("Generated node belongs to another compiler crate".into());
        }
        let key = format!("{}.{}", node.hir.owner, node.hir.local);
        let unplaced = || {
            format!(
                "Generated facts role {} has no exact raw HIR span capture",
                node.kind
            )
        };
        let sites = self.function.nodes.get(&key).ok_or_else(unplaced)?;
        let site = match sites {
            NodeSites {
                unsupported: true, ..
            } => None,
            NodeSites {
                ident: Some(ident), ..
            } if node.kind == "pat_binding" => Some(ident),
            NodeSites {
                item: Some(item), ..
            } if node.kind == "stmt_item" => Some(item),
            NodeSites { span, .. } => span.as_ref(),
        };
        let site = site.ok_or_else(unplaced)?;
        let origin = self.provenance.origin(site)?;

        self.records.push(NodeOrigin {
            path: path.to_vec(),
            hir: node.hir.clone(),
            kind: node.kind.clone(),
            origin,
        });
        Ok(())
    }
}
