//! Opt-in associated product observations; no admission or callable authority.
mod conversions;
mod projection;

use super::super::{Options, function_source, hir::Context, source};
use super::diagnostic_support::ArgumentsProjection;
use super::{diagnostic_support, types};
use rstyle_pdg::{HirFnFacts, HirNode};
use rustc_hir::{self as hir, def::DefKind, def_id::LocalDefId};
use rustc_middle::ty::{self, TyCtxt};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub(super) struct CompilerDefinition {
    #[serde(flatten)]
    definition: super::Definition,
    #[serde(skip_serializing_if = "Option::is_none")]
    crate_hash: Option<String>,
}

fn compiler_definition(tcx: TyCtxt<'_>, id: hir::def_id::DefId) -> CompilerDefinition {
    CompilerDefinition {
        definition: super::definition_record(tcx, id),
        crate_hash: (!id.is_local()).then(|| tcx.crate_hash(id.krate).to_string()),
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Request {
    function: String,
    arguments: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Requests {
    identity: String,
    crate_name: String,
    requests: Vec<Request>,
}

#[derive(Serialize)]
struct Inventory {
    identity: &'static str,
    scope: &'static str,
    admission_authority: bool,
    requests: Vec<Entry>,
}

#[derive(Serialize)]
struct Entry {
    request: Request,
    #[serde(flatten)]
    outcome: Outcome,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Outcome {
    Observed(Box<OriginalOwner>),
    Unavailable { reason: String },
    Unsupported { reason: String },
}

#[derive(Serialize)]
struct OriginalOwner {
    original: HirFnFacts,
    instances: Vec<InstanceEntry>,
}

#[derive(Serialize)]
struct InstanceEntry {
    argument: String,
    #[serde(flatten)]
    outcome: InstanceOutcome,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum InstanceOutcome {
    Observed(Box<InstanceObservation>),
    Unsupported { reason: String },
    Unresolved { reason: String },
    Error { reason: String },
}

pub(super) enum Failure {
    Unsupported(String),
    Unresolved(String),
    Error(String),
}

impl From<String> for Failure {
    fn from(reason: String) -> Self {
        Self::Unsupported(reason)
    }
}
impl From<&str> for Failure {
    fn from(reason: &str) -> Self {
        Self::Unsupported(reason.into())
    }
}

#[derive(Serialize)]
struct InstanceObservation {
    arguments: types::Arguments,
    owners: Vec<projection::Owner>,
    original_signature: OriginalSignature,
    projected_inputs: Vec<types::Arguments>,
    projected_result: types::Arguments,
    nodes: Vec<Node>,
    conversions: Vec<conversions::Observation>,
    selected_local_from: Option<conversions::LocalBody>,
}

#[derive(Serialize)]
struct OriginalSignature {
    inputs: Vec<projection::OriginalType>,
    output: projection::OriginalType,
}

#[derive(Serialize)]
struct Node {
    hir: rstyle_pdg::HirRef,
    span: rstyle_pdg::ByteSpan,
    original_kind: String,
    #[serde(flatten)]
    ty: NodeType,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum NodeType {
    Value {
        original: projection::OriginalType,
        projected: types::Arguments,
    },
    FunctionItem {
        definition: CompilerDefinition,
        arguments: types::Arguments,
    },
}

pub(crate) fn capture(
    tcx: TyCtxt<'_>,
    scanned: &[source::Function],
    options: &Options<'_>,
) -> Result<(), String> {
    let Some(requests): Option<Requests> =
        diagnostic_support::optional_requests("RSTYLE_DIAGNOSTIC_PRODUCT_INSTANCE_REQUESTS")?
    else {
        return Ok(());
    };
    validate(&requests, tcx, options)?;
    let owners = diagnostic_support::owners(tcx, &[DefKind::AssocFn]);
    let entries = requests
        .requests
        .into_iter()
        .map(|request| {
            let outcome = match owners.get(&request.function).copied().flatten() {
                Some(owner) => observe_owner(tcx, owner, &request, Inputs { scanned, options }),
                None => Outcome::Unavailable {
                    reason: "Original associated owner is unavailable or ambiguous".into(),
                },
            };
            Entry { request, outcome }
        })
        .collect();

    diagnostic_support::persist(
        &Inventory {
            identity: "rstyle.diagnostic.product-instance-bodies/1",
            scope: "original associated Match constructor, Span/Range substitutions and selected Into/From observations only",
            admission_authority: false,
            requests: entries,
        },
        "product-instances",
        "Product diagnostics require an explicit snapshot directory",
    )
}

fn validate(requests: &Requests, tcx: TyCtxt<'_>, options: &Options<'_>) -> Result<(), String> {
    if options.selected() != rstyle_pdg::extraction_profile::ExtractionProfile::DataV3
        || requests.identity != "rstyle.diagnostic.product-instance-requests/1"
        || requests.crate_name != tcx.crate_name(hir::def_id::LOCAL_CRATE).as_str()
        || requests.requests.is_empty()
        || requests.requests.len() > 8
    {
        return Err("Unregistered product diagnostic context or oversized requests".into());
    }
    for request in &requests.requests {
        if request.arguments.is_empty()
            || request.arguments.len() > 2
            || request
                .arguments
                .iter()
                .any(|argument| !matches!(argument.as_str(), "Span" | "Range<usize>"))
            || request.arguments.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(
                "Product requests require distinct bounded Span/Range<usize> substitutions".into(),
            );
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Inputs<'a, 'b> {
    scanned: &'a [source::Function],
    options: &'a Options<'b>,
}

#[derive(Clone, Copy)]
struct Original<'a, 'tcx> {
    context: &'a Context<'tcx>,
    owner: LocalDefId,
    facts: &'a HirFnFacts,
}

fn observe_owner(
    tcx: TyCtxt<'_>,
    owner: LocalDefId,
    request: &Request,
    inputs: Inputs<'_, '_>,
) -> Outcome {
    let Some((facts, context)) = function_source(tcx, owner, inputs.scanned, inputs.options) else {
        return Outcome::Unavailable {
            reason: "Original product source/signature is unavailable".into(),
        };
    };
    if tcx.item_name(owner.to_def_id()).as_str() != "new" {
        return Outcome::Unsupported {
            reason: "Bounded product diagnostic requires the original new constructor".into(),
        };
    }
    let instances = request
        .arguments
        .iter()
        .map(|argument| {
            let outcome = match observe_instance(
                Original {
                    context: &context,
                    owner,
                    facts: &facts,
                },
                argument,
                inputs,
            ) {
                Ok(observation) => InstanceOutcome::Observed(Box::new(observation)),
                Err(Failure::Unsupported(reason)) => InstanceOutcome::Unsupported { reason },
                Err(Failure::Unresolved(reason)) => InstanceOutcome::Unresolved { reason },
                Err(Failure::Error(reason)) => InstanceOutcome::Error { reason },
            };
            InstanceEntry {
                argument: argument.clone(),
                outcome,
            }
        })
        .collect();
    Outcome::Observed(Box::new(OriginalOwner {
        original: facts,
        instances,
    }))
}

fn observe_instance(
    original: Original<'_, '_>,
    argument: &str,
    inputs: Inputs<'_, '_>,
) -> Result<InstanceObservation, Failure> {
    let Original { context, owner, .. } = original;
    let tcx = context.tcx;
    let lens = projection::Projection::new(context, owner, argument)?;
    let parent = tcx
        .generics_of(owner)
        .parent
        .ok_or("Missing product original parent")?
        .expect_local();
    let signature = tcx.fn_sig(owner).instantiate_identity().skip_binder();
    let scan = Scan::capture(original, &lens)?;

    let selected_local_from = scan
        .local_from
        .map(|id| conversions::local_owner(tcx, id, inputs.scanned, inputs.options))
        .transpose()?;
    Ok(InstanceObservation {
        arguments: types::arguments(tcx, lens.arguments()),
        owners: vec![
            projection::owner(context, parent, &lens)?,
            projection::owner(context, owner, &lens)?,
        ],
        original_signature: OriginalSignature {
            inputs: signature
                .inputs()
                .iter()
                .map(|value| projection::original_type(tcx, *value, 0))
                .collect::<Result<_, _>>()?,
            output: projection::original_type(tcx, signature.output(), 0)?,
        },
        projected_inputs: signature
            .inputs()
            .iter()
            .map(|value| types::endpoint(tcx, lens.ty(tcx, *value)?))
            .collect::<Result<_, String>>()?,
        projected_result: types::endpoint(tcx, lens.ty(tcx, context.result)?)?,
        nodes: scan.nodes,
        conversions: scan.conversions,
        selected_local_from,
    })
}

struct Scan<'a, 'tcx> {
    context: &'a Context<'tcx>,
    lens: &'a projection::Projection<'tcx>,
    nodes: Vec<Node>,
    conversions: Vec<conversions::Observation>,
    local_from: Option<LocalDefId>,
}

impl<'tcx> Scan<'_, 'tcx> {
    fn capture<'a>(
        original: Original<'a, 'tcx>,
        lens: &'a projection::Projection<'tcx>,
    ) -> Result<Scan<'a, 'tcx>, Failure> {
        let mut scan = Scan {
            context: original.context,
            lens,
            nodes: Vec::new(),
            conversions: Vec::new(),
            local_from: None,
        };

        for parameter in &original.facts.signature.params {
            scan.node(&parameter.pat)?;
        }
        scan.node(&original.facts.body)?;
        if scan.conversions.len() != 1 {
            return Err("Product body must retain exactly one selected Into/From operation".into());
        }
        Ok(scan)
    }

    fn node(&mut self, node: &HirNode) -> Result<(), Failure> {
        let id =
            diagnostic_support::hir_id(&node.hir, "Product HIR node is not original owner-local")?;
        if id.owner.def_id != self.context.typeck.hir_owner.def_id {
            return Err("Product node roster contains a different HIR owner".into());
        }
        if node.ty.is_some() {
            self.nodes.push(Node {
                hir: node.hir.clone(),
                span: node.span,
                original_kind: node.kind.clone(),
                ty: self.node_type(id)?,
            });
        }
        self.operation(id)?;

        for child in &node.children {
            self.node(child)?;
        }
        Ok(())
    }

    fn operation(&mut self, id: hir::HirId) -> Result<(), Failure> {
        if let hir::Node::Expr(expression) = self.context.tcx.hir_node(id)
            && let Some(captured) = conversions::observe(self.context, expression, self.lens)?
        {
            self.local_from = captured.local_from;
            self.conversions.push(captured.observation);
        }
        Ok(())
    }

    fn node_type(&self, id: hir::HirId) -> Result<NodeType, Failure> {
        let tcx = self.context.tcx;
        let original = self
            .context
            .typeck
            .node_type_opt(id)
            .ok_or("Missing original product node type")?;
        if let ty::FnDef(definition, arguments) = original.kind() {
            return Ok(NodeType::FunctionItem {
                definition: compiler_definition(tcx, *definition),
                arguments: types::arguments(tcx, self.lens.args(tcx, arguments.skip_binder())?),
            });
        }
        Ok(NodeType::Value {
            original: projection::original_type(tcx, original, 0)?,
            projected: types::endpoint(tcx, self.lens.node_ty(self.context, id)?)?,
        })
    }
}
