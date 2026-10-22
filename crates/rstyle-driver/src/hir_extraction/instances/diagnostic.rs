//! Explicit owner-local feasibility capture; never imported as admission evidence.
use super::super::{Options, function_source, hir::Context, source};
use super::{
    adjustments, comparisons, definition_record, projection::Projection, provenance, types,
};
use rstyle_pdg::{HirNode, SignatureFacts};
use rustc_hir::{self as hir, def::DefKind, def_id::LocalDefId};
use rustc_middle::ty::{self, TyCtxt};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

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
    arguments: Vec<String>,
}

#[derive(Serialize)]
struct Inventory {
    identity: &'static str,
    scope: &'static str,
    obligations_verified: bool,
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
    Projected(Box<Body>),
    Unavailable { reason: String },
    Unsupported { reason: String },
    Unresolved { reason: String },
    Error { reason: String },
}

#[derive(Debug)]
enum Failure {
    Unsupported(String),
    Unresolved(String),
    Error(String),
}

impl From<super::projection::Error> for Failure {
    fn from(error: super::projection::Error) -> Self {
        use super::projection::Error;
        match error {
            Error::Unresolved | Error::NoInstance => Self::Unresolved(error.to_string()),
            Error::Normalization | Error::Resolution => Self::Error(error.to_string()),
            _ => Self::Unsupported(error.to_string()),
        }
    }
}

impl From<&str> for Failure {
    fn from(reason: &str) -> Self {
        Self::Unsupported(reason.into())
    }
}

impl From<String> for Failure {
    fn from(reason: String) -> Self {
        Self::Unsupported(reason)
    }
}

#[derive(Serialize)]
struct Body {
    definition: super::Definition,
    provenance: provenance::Observation,
    arguments: types::Arguments,
    source: rstyle_pdg::SourceFacts,
    fn_span: rstyle_pdg::ByteSpan,
    signature: SignatureFacts,
    bindings: Vec<rstyle_pdg::BindingFact>,
    body: HirNode,
    comparisons: Vec<comparisons::Observation>,
    adjustments: Vec<adjustments::Observation>,
}

pub(crate) fn capture(
    tcx: TyCtxt<'_>,
    scanned: &[source::Function],
    options: &Options<'_>,
) -> Result<(), String> {
    let Some(path) = std::env::var_os("RSTYLE_DIAGNOSTIC_INSTANCE_REQUESTS") else {
        return Ok(());
    };
    if options.selected() != rstyle_pdg::extraction_profile::ExtractionProfile::DataV3 {
        return Err("Scalar instance diagnostics require hir-data-v3".into());
    }

    let input: Requests =
        serde_json::from_reader(std::fs::File::open(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    if input.identity != "rstyle.diagnostic.scalar-instance-requests/1" || input.requests.len() > 64
    {
        return Err("Unregistered or oversized scalar diagnostic requests".into());
    }
    if input.crate_name != tcx.crate_name(hir::def_id::LOCAL_CRATE).as_str() {
        return Err("Scalar diagnostic requests name a different owning crate".into());
    }

    let owners = owners(tcx);
    let inventory = Inventory {
        identity: "rstyle.diagnostic.scalar-instance-bodies/1",
        scope: "owner-local projection only; no caller/artifact binding, obligation proof or admission authority",
        obligations_verified: false,
        requests: input
            .requests
            .into_iter()
            .map(|request| {
                let owner = Owner {
                    tcx,
                    id: owners.get(&request.function).copied().flatten(),
                };
                let outcome = project(owner, scanned, options, &request);
                Entry { request, outcome }
            })
            .collect(),
    };
    persist(&inventory)
}

fn owners(tcx: TyCtxt<'_>) -> BTreeMap<String, Option<LocalDefId>> {
    let mut owners = BTreeMap::new();
    for id in tcx
        .hir_body_owners()
        .filter(|id| tcx.def_kind(*id) == DefKind::Fn)
    {
        owners
            .entry(tcx.def_path_str(id.to_def_id()))
            .and_modify(|existing| *existing = None)
            .or_insert(Some(id));
    }
    owners
}

struct Owner<'tcx> {
    tcx: TyCtxt<'tcx>,
    id: Option<LocalDefId>,
}

fn project(
    owner: Owner<'_>,
    scanned: &[source::Function],
    options: &Options<'_>,
    request: &Request,
) -> Outcome {
    let Some(id) = owner.id else {
        return Outcome::Unavailable {
            reason: "Requested original free-function owner is unavailable or ambiguous".into(),
        };
    };
    let Some((facts, context)) = function_source(owner.tcx, id, scanned, options) else {
        return Outcome::Unavailable {
            reason: "Requested original source evidence is unavailable".into(),
        };
    };
    match body(id, request, context, facts) {
        Ok(body) => Outcome::Projected(Box::new(body)),
        Err(Failure::Unsupported(reason)) => Outcome::Unsupported { reason },
        Err(Failure::Unresolved(reason)) => Outcome::Unresolved { reason },
        Err(Failure::Error(reason)) => Outcome::Error { reason },
    }
}

fn body<'tcx>(
    owner: LocalDefId,
    request: &Request,
    context: Context<'tcx>,
    facts: rstyle_pdg::HirFnFacts,
) -> Result<Body, Failure> {
    let tcx = context.tcx;
    if request.arguments.len() != 1 {
        return Err("Diagnostic projection requires exactly one type argument".into());
    }

    let arguments = request
        .arguments
        .iter()
        .map(|argument| match argument.as_str() {
            "i32" => Ok(tcx.types.i32.into()),
            "u32" => Ok(tcx.types.u32.into()),
            _ => Err(Failure::from(
                "Diagnostic instance argument is outside i32/u32",
            )),
        })
        .collect::<Result<Vec<ty::GenericArg<'tcx>>, Failure>>()?;
    let arguments = tcx.mk_args(&arguments);
    let projection = Projection::new(tcx, owner, arguments).map_err(Failure::from)?;

    let provenance = provenance::capture(&context, owner, &projection, &facts.body)?;
    let signature = project_signature(&context, &projection, facts.signature)?;
    let bindings = project_bindings(&context, &projection, facts.bindings)?;
    let projected = project_node(&context, &projection, facts.body)?;
    Ok(Body {
        definition: definition_record(tcx, owner.to_def_id()),
        provenance,
        arguments: types::arguments(tcx, arguments),
        source: facts.source,
        fn_span: facts.fn_span,
        signature,
        bindings,
        body: projected.node,
        comparisons: projected.comparisons,
        adjustments: projected.adjustments,
    })
}

fn project_bindings<'tcx>(
    context: &Context<'tcx>,
    projection: &Projection<'tcx>,
    bindings: Vec<rstyle_pdg::BindingFact>,
) -> Result<Vec<rstyle_pdg::BindingFact>, Failure> {
    bindings
        .into_iter()
        .map(|mut binding| {
            binding.ty = projected_type(context, projection, &binding.hir, &binding.ty.spelling)?;
            Ok(binding)
        })
        .collect()
}

fn project_signature<'tcx>(
    context: &Context<'tcx>,
    projection: &Projection<'tcx>,
    mut signature: SignatureFacts,
) -> Result<SignatureFacts, Failure> {
    signature.params = signature
        .params
        .into_iter()
        .map(|mut parameter| {
            parameter.ty = projected_type(
                context,
                projection,
                &parameter.pat.hir,
                &parameter.ty.spelling,
            )?;
            parameter.pat = project_node(context, projection, parameter.pat)?.node;
            Ok(parameter)
        })
        .collect::<Result<_, Failure>>()?;
    signature.return_ty = super::super::hir::resolved(
        projection
            .ty(context.tcx, context.result)
            .map_err(Failure::from)?,
        signature.return_ty.spelling,
    );
    Ok(signature)
}

struct Projected {
    node: HirNode,
    comparisons: Vec<comparisons::Observation>,
    adjustments: Vec<adjustments::Observation>,
}

fn project_node<'tcx>(
    context: &Context<'tcx>,
    projection: &Projection<'tcx>,
    node: HirNode,
) -> Result<Projected, Failure> {
    let mut view = View {
        context,
        projection,
        comparisons: Vec::new(),
        adjustments: Vec::new(),
    };
    let node = view.node(node)?;
    Ok(Projected {
        node,
        comparisons: view.comparisons,
        adjustments: view.adjustments,
    })
}

struct View<'a, 'tcx> {
    context: &'a Context<'tcx>,
    projection: &'a Projection<'tcx>,
    comparisons: Vec<comparisons::Observation>,
    adjustments: Vec<adjustments::Observation>,
}

impl<'tcx> View<'_, 'tcx> {
    fn node(&mut self, mut node: HirNode) -> Result<HirNode, Failure> {
        let id = hir_id(&node.hir)?;
        if id.owner.def_id != self.context.typeck.hir_owner.def_id {
            return Err("Projected body contains a different HIR owner".into());
        }
        if let Some(original) = &node.ty {
            node.ty = Some(if node.kind == "expr_ret" {
                super::super::hir::resolved(
                    self.projection
                        .ty(self.context.tcx, self.context.result)
                        .map_err(Failure::from)?,
                    original.spelling.clone(),
                )
            } else {
                projected_type(self.context, self.projection, &node.hir, &original.spelling)?
            });
        }

        if let hir::Node::Expr(expression) = self.context.tcx.hir_node(id) {
            self.observe(expression)?;
            node.dispatch = node
                .dispatch
                .map(|dispatch| self.dispatch(expression, dispatch))
                .transpose()?;
        }

        node.children = node
            .children
            .into_iter()
            .map(|child| self.node(child))
            .collect::<Result<_, _>>()?;
        Ok(node)
    }

    fn observe(&mut self, expression: &hir::Expr<'tcx>) -> Result<(), Failure> {
        if let Some(observation) = comparisons::observe(self.context, expression, self.projection)
            .map_err(Failure::Error)?
        {
            self.comparisons.push(observation);
        }
        let observation = adjustments::operand(self.context, expression, self.projection)?;
        if !observation.is_empty() {
            self.adjustments.push(observation);
        }
        Ok(())
    }

    fn dispatch(
        &self,
        expression: &hir::Expr<'tcx>,
        dispatch: rstyle_pdg::Dispatch,
    ) -> Result<rstyle_pdg::Dispatch, Failure> {
        let (_, operands) = self
            .context
            .dispatch_operands(expression)
            .ok_or("Projected dispatch lacks original operands")?;
        let operand_tys = operands
            .iter()
            .map(|operand| {
                self.projection
                    .node_ty(self.context.tcx, self.context.typeck, operand.hir_id)
                    .map(|ty| ty.to_string())
                    .map_err(Failure::from)
            })
            .collect::<Result<_, _>>()?;
        let result_ty = self
            .projection
            .node_ty(self.context.tcx, self.context.typeck, expression.hir_id)
            .map_err(Failure::from)?
            .to_string();
        Ok(rstyle_pdg::Dispatch {
            operand_tys,
            result_ty,
            ..dispatch
        })
    }
}

fn projected_type<'tcx>(
    context: &Context<'tcx>,
    projection: &Projection<'tcx>,
    reference: &rstyle_pdg::HirRef,
    spelling: &str,
) -> Result<rstyle_pdg::ResolvedTy, Failure> {
    projection
        .node_ty(context.tcx, context.typeck, hir_id(reference)?)
        .map(|ty| super::super::hir::resolved(ty, spelling.into()))
        .map_err(Failure::from)
}

fn hir_id(reference: &rstyle_pdg::HirRef) -> Result<hir::HirId, Failure> {
    if reference.krate != 0 {
        return Err("Projected HIR reference is not owner-local".into());
    }
    Ok(hir::HirId {
        owner: hir::OwnerId {
            def_id: LocalDefId {
                local_def_index: hir::def_id::DefIndex::from_u32(reference.owner),
            },
        },
        local_id: hir::ItemLocalId::from_u32(reference.local),
    })
}

fn persist(inventory: &Inventory) -> Result<(), String> {
    let directory = PathBuf::from(
        std::env::var_os("RSTYLE_DIAGNOSTIC_FACTS_DIR")
            .ok_or("Scalar instance diagnostic requires a snapshot directory")?,
    );
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join(format!("{}-scalar-instances.json", std::process::id()));

    super::super::diagnostics::persist_file(&path, inventory).map_err(|error| error.to_string())
}
