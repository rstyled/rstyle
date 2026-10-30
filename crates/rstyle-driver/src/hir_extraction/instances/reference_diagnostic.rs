//! Owning reference-instance observations, never an admission or lifetime proof.
mod association;
use super::super::{Options, function_source, hir::Context, source};
use super::diagnostic_support::ArgumentsProjection;
use super::{
    borrowed_operations, definition_record, diagnostic_support, reference_owners,
    types::{self, endpoint},
};
use rstyle_pdg::{HirFnFacts, HirNode};
use rustc_hir::{self as hir, def::DefKind, def_id::LocalDefId};
use rustc_middle::ty::{self, TyCtxt};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Requests {
    identity: String,
    crate_name: String,
    requests: Vec<Request>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Request {
    function: String,
    haystack: String,
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
    Observed(Box<Body>),
    Unavailable { reason: String },
    Unsupported { reason: String },
}

#[derive(Serialize)]
struct Body {
    definition: super::Definition,
    provenance: reference_owners::Observation,
    arguments: types::Arguments,
    original: HirFnFacts,
    projected_result: types::Arguments,
    nodes: Vec<Node>,
    conversions: Vec<borrowed_operations::Observation>,
    calls: Vec<Call>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outer_trait_association: Option<association::Observation>,
}

#[derive(Serialize)]
struct Node {
    hir: rstyle_pdg::HirRef,
    #[serde(flatten)]
    ty: NodeType,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum NodeType {
    Value {
        ty: types::Arguments,
    },
    FunctionItem {
        definition: super::Definition,
        arguments: types::Arguments,
    },
}

#[derive(Serialize)]
struct Call {
    hir: rstyle_pdg::HirRef,
    requested: super::Definition,
    requested_arguments: types::Arguments,
    selected: super::Definition,
    selected_arguments: types::Arguments,
}

pub(crate) fn capture(
    tcx: TyCtxt<'_>,
    scanned: &[source::Function],
    options: &Options<'_>,
) -> Result<(), String> {
    let Some(requests): Option<Requests> =
        diagnostic_support::optional_requests("RSTYLE_DIAGNOSTIC_REFERENCE_INSTANCE_REQUESTS")?
    else {
        return Ok(());
    };
    validate(&requests, tcx, options)?;

    let owners = diagnostic_support::owners(tcx, &[DefKind::Fn, DefKind::AssocFn]);
    let entries = requests.requests.into_iter().map(|request| {
        let outcome = match owners.get(&request.function).copied().flatten() {
            Some(id) => observe(Owner { tcx, id }, &request, scanned, options),
            None => Outcome::Unavailable {
                reason: "Original owning function is absent or ambiguous".into(),
            },
        };
        Entry { request, outcome }
    });

    diagnostic_support::persist(
        &Inventory {
            identity: "rstyle.diagnostic.reference-instance-bodies/1",
            scope: "original owning-body observations only; no lifetime, cleanup, origin, artifact or admission authority",
            admission_authority: false,
            requests: entries.collect(),
        },
        "reference-instances",
        "Reference diagnostics require an explicit snapshot directory",
    )
}

fn validate(requests: &Requests, tcx: TyCtxt<'_>, options: &Options<'_>) -> Result<(), String> {
    if options.selected() != rstyle_pdg::extraction_profile::ExtractionProfile::DataV3
        || requests.identity != "rstyle.diagnostic.reference-instance-requests/1"
        || requests.requests.len() > 64
        || requests.crate_name != tcx.crate_name(hir::def_id::LOCAL_CRATE).as_str()
    {
        return Err("Unregistered reference-instance diagnostic context or requests".into());
    }
    Ok(())
}

struct Owner<'tcx> {
    tcx: TyCtxt<'tcx>,
    id: LocalDefId,
}

fn observe(
    owner: Owner<'_>,
    request: &Request,
    scanned: &[source::Function],
    options: &Options<'_>,
) -> Outcome {
    let Some((facts, context)) = function_source(owner.tcx, owner.id, scanned, options) else {
        return Outcome::Unavailable {
            reason: "Original owner source/signature is unavailable".into(),
        };
    };
    match body(owner.id, request, context, facts) {
        Ok(body) => Outcome::Observed(Box::new(body)),
        Err(reason) => Outcome::Unsupported { reason },
    }
}

fn body<'tcx>(
    owner: LocalDefId,
    request: &Request,
    context: Context<'tcx>,
    facts: HirFnFacts,
) -> Result<Body, String> {
    let tcx = context.tcx;
    let projection = reference_owners::Projection::new(tcx, owner, &request.haystack)?;
    let provenance = reference_owners::capture(&context, owner, &projection)?;
    let outer_trait_association = association::capture(tcx, owner, &projection)?;
    let projected_result = endpoint(tcx, projection.ty(tcx, context.result)?)?;

    let mut scan = Scan {
        context: &context,
        projection: &projection,
        nodes: Vec::new(),
        conversions: Vec::new(),
        calls: Vec::new(),
    };

    for parameter in &facts.signature.params {
        scan.node(&parameter.pat)?;
    }
    scan.node(&facts.body)?;

    Ok(Body {
        definition: definition_record(tcx, owner.to_def_id()),
        provenance,
        arguments: types::arguments(tcx, projection.arguments()),
        projected_result,
        nodes: scan.nodes,
        conversions: scan.conversions,
        calls: scan.calls,
        original: facts,
        outer_trait_association,
    })
}

struct Scan<'a, 'tcx> {
    context: &'a Context<'tcx>,
    projection: &'a reference_owners::Projection<'tcx>,
    nodes: Vec<Node>,
    conversions: Vec<borrowed_operations::Observation>,
    calls: Vec<Call>,
}

impl<'tcx> Scan<'_, 'tcx> {
    fn node(&mut self, node: &HirNode) -> Result<(), String> {
        let id = diagnostic_support::hir_id(
            &node.hir,
            "Reference observation HIR must belong to the original crate",
        )?;
        if id.owner.def_id != self.context.typeck.hir_owner.def_id {
            return Err("Reference observation encountered a different HIR owner".into());
        }

        if node.ty.is_some() {
            self.nodes.push(Node {
                hir: node.hir.clone(),
                ty: self.node_type(id)?,
            });
        }
        if let hir::Node::Expr(expression) = self.context.tcx.hir_node(id) {
            self.expression(expression)?;
        }

        for child in &node.children {
            self.node(child)?;
        }
        Ok(())
    }

    fn node_type(&self, id: hir::HirId) -> Result<NodeType, String> {
        let tcx = self.context.tcx;
        let original = self
            .context
            .typeck
            .node_type_opt(id)
            .ok_or("Reference observation is missing its original node type")?;
        if let ty::FnDef(definition, arguments) = original.kind() {
            return Ok(NodeType::FunctionItem {
                definition: definition_record(tcx, *definition),
                arguments: types::arguments(
                    tcx,
                    self.projection.args(tcx, arguments.skip_binder())?,
                ),
            });
        }
        Ok(NodeType::Value {
            ty: endpoint(tcx, self.projection.node_ty(tcx, self.context.typeck, id)?)?,
        })
    }

    fn expression(&mut self, expression: &hir::Expr<'tcx>) -> Result<(), String> {
        if let Some(observation) =
            borrowed_operations::observe(self.context, expression, self.projection)?
        {
            self.conversions.push(observation);
        }

        let hir::ExprKind::Call(callee, _) = expression.kind else {
            return Ok(());
        };
        let ty::FnDef(definition, arguments) = self.context.typeck.expr_ty(callee).kind() else {
            return Err("Reference diagnostic direct call is not a function item".into());
        };

        let original_arguments = arguments.skip_binder();
        let selected = self
            .projection
            .select(self.context.tcx, *definition, original_arguments)?;

        self.calls.push(Call {
            hir: super::super::hir::reference(expression.hir_id),
            requested: definition_record(self.context.tcx, *definition),
            requested_arguments: types::arguments(
                self.context.tcx,
                self.projection.args(self.context.tcx, original_arguments)?,
            ),
            selected: definition_record(self.context.tcx, selected.def_id()),
            selected_arguments: types::arguments(self.context.tcx, selected.args),
        });
        Ok(())
    }
}
