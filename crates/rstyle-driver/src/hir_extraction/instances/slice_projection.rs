//! Proof inputs for selected local shared-slice instances; never admission authority.
use super::{definition_record, diagnostic_support, projection, types};
use crate::hir_extraction::{Options, function_source, hir::Context, source};
use rstyle_pdg::{HirFnFacts, HirNode};
use rustc_hir::{self as hir, def::DefKind, def_id::LocalDefId};
use rustc_middle::ty::{self, GenericArgsRef, Ty, TyCtxt};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

mod methods;

const KEY: &str = "rstyle.rust.shared-slice-instance-projections";

/// Only actual, complete same-unit call receipts request this bounded lens.
pub(super) fn capture(
    tcx: TyCtxt<'_>,
    scanned: &[source::Function],
    options: &Options<'_>,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    let receipts = receipts(facts);
    if receipts.is_empty() {
        return Ok(());
    }

    let capture = Capture {
        tcx,
        scanned,
        options,
        receipts,
        owners: owner_roster(tcx),
    };
    let observations = facts
        .iter()
        .enumerate()
        .filter_map(|(index, original)| capture.observation(original).map(|value| (index, value)))
        .map(|(index, value)| {
            serde_json::to_value(value)
                .map(|value| (index, value))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, String>>()?;

    for (index, value) in observations {
        install(&mut facts[index], value);
    }

    Ok(())
}

struct Capture<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    scanned: &'a [source::Function],
    options: &'a Options<'a>,
    receipts: BTreeMap<String, Vec<Value>>,
    owners: BTreeMap<String, LocalDefId>,
}

#[derive(Serialize)]
struct Envelope<'a> {
    identity: &'static str,
    scope: &'static str,
    obligations_verified: bool,
    call_receipts: &'a [Value],
    outcome: Outcome,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Outcome {
    Observed { projection: Box<Observation> },
    Unsupported { reason: String },
}

impl Capture<'_, '_> {
    fn observation(&self, original: &HirFnFacts) -> Option<Envelope<'_>> {
        let identity = original
            .build
            .frontend_options
            .get("definition_identity")?
            .as_str()?;
        let calls = self.receipts.get(identity)?;
        let owner = *self.owners.get(identity)?;
        let observation = match function_source(self.tcx, owner, self.scanned, self.options) {
            Some((_, context)) => observe(&context, owner, original),
            None => Err("Selected instance original source context is unavailable".into()),
        };
        let outcome = match observation {
            Ok(projection) => Outcome::Observed {
                projection: Box::new(projection),
            },
            Err(reason) => Outcome::Unsupported { reason },
        };

        Some(Envelope {
            identity: "rstyle.rust.shared-slice-instance-projections/1",
            scope: "selected local u8 shared-slice original-owner type overlay; no obligation, source verification or admission authority",
            obligations_verified: false,
            call_receipts: calls,
            outcome,
        })
    }
}

fn owner_roster(tcx: TyCtxt<'_>) -> BTreeMap<String, LocalDefId> {
    crate::hir_extraction::function_owners(tcx)
        .filter(|id| candidate(tcx, *id))
        .map(|id| {
            (
                definition_record(tcx, id.to_def_id()).definition_identity,
                id,
            )
        })
        .collect()
}

fn install(original: &mut HirFnFacts, value: Value) {
    original.build.frontend_options.insert(KEY.into(), value);
}

fn receipts(facts: &[HirFnFacts]) -> BTreeMap<String, Vec<Value>> {
    facts
        .iter()
        .flat_map(receipt_pairs)
        .fold(BTreeMap::new(), group_receipt)
}

fn group_receipt(
    mut grouped: BTreeMap<String, Vec<Value>>,
    pair: (String, Value),
) -> BTreeMap<String, Vec<Value>> {
    grouped.entry(pair.0).or_default().push(pair.1);
    grouped
}

fn receipt_pairs(caller: &HirFnFacts) -> Vec<(String, Value)> {
    let options = &caller.build.frontend_options;
    let Some(inventory) = options.get("rstyle.rust.call-instances") else {
        return Vec::new();
    };
    if inventory.get("identity").and_then(Value::as_str)
        != Some("rstyle.rust.call-instance-observations/1")
        || inventory.get("complete") != Some(&Value::Bool(true))
    {
        return Vec::new();
    }
    let Some(calls) = inventory.get("calls").and_then(Value::as_array) else {
        return Vec::new();
    };

    calls
        .iter()
        .filter_map(|call| {
            let identity = receipt_identity(call)?;
            Some((
                identity.into(),
                json!({
                    "caller":options.get("definition_identity"),
                    "caller_source_digest":caller.source.digest,
                    "caller_fn_span":caller.fn_span,
                    "call":call,
                }),
            ))
        })
        .collect()
}

fn receipt_identity(call: &Value) -> Option<&str> {
    let requested = call.get("requested")?;
    if call.get("status").and_then(Value::as_str) != Some("resolved")
        || call.get("expanded") != Some(&Value::Bool(false))
        || requested.get("local") != Some(&Value::Bool(true))
        || call.get("selected") != Some(requested)
        || !u8_arguments(call.get("requested_arguments"))
        || !u8_arguments(call.get("selected_arguments"))
    {
        return None;
    }

    requested.get("definition_identity").and_then(Value::as_str)
}

fn u8_arguments(value: Option<&Value>) -> bool {
    let Some(arguments) = value.and_then(Value::as_array) else {
        return false;
    };
    matches!(arguments.as_slice(), [argument]
        if argument.get("kind").and_then(Value::as_str) == Some("type")
        && argument.get("identity_status").and_then(Value::as_str) == Some("known")
        && argument.get("identity") == Some(&json!({"kind":"builtin","name":"u8"})))
}

fn candidate(tcx: TyCtxt<'_>, owner: LocalDefId) -> bool {
    let generics = tcx.generics_of(owner);
    if tcx.def_kind(owner) != DefKind::Fn
        || generics.parent.is_some()
        || generics.own_params.len() != 1
        || !matches!(
            generics.own_params[0].kind,
            ty::GenericParamDefKind::Type {
                has_default: false,
                synthetic: false
            }
        )
    {
        return false;
    }

    let signature = tcx.fn_sig(owner).instantiate_identity().skip_binder();
    let [slice, length, what] = signature.inputs() else {
        return false;
    };
    let ty::Ref(_, slice, hir::Mutability::Not) = slice.kind() else {
        return false;
    };
    let ty::Slice(element) = slice.kind() else {
        return false;
    };

    if !matches!(element.kind(), ty::Param(parameter) if parameter.index == 0)
        || !matches!(length.kind(), ty::Uint(ty::UintTy::Usize))
        || !matches!(what.kind(), ty::Ref(region, inner, hir::Mutability::Not)
            if matches!(region.kind(), ty::ReStatic) && matches!(inner.kind(), ty::Str))
    {
        return false;
    }

    let ty::Adt(result, args) = signature.output().kind() else {
        return false;
    };
    if !tcx.is_diagnostic_item(rustc_span::sym::Result, result.did())
        || args.len() != 2
        || !matches!(args.type_at(0).kind(), ty::Tuple(elements) if elements.is_empty())
    {
        return false;
    }

    matches!(args.type_at(1).kind(), ty::Adt(error, arguments)
        if error.is_struct() && error.did().is_local() && arguments.is_empty())
}

#[derive(Serialize)]
struct Observation {
    definition: super::Definition,
    type_parameter: super::Definition,
    owner: rstyle_pdg::HirRef,
    source_digest: String,
    fn_span: rstyle_pdg::ByteSpan,
    original_source_scan_complete: bool,
    arguments: types::Arguments,
    parameters: Vec<Typed>,
    result: TypeObservation,
    bindings: Vec<Typed>,
    nodes: Vec<Typed>,
    methods: Vec<methods::Method>,
}

#[derive(Serialize)]
struct Typed {
    hir: rstyle_pdg::HirRef,
    concrete: TypeObservation,
    adjustments: Vec<TypeObservation>,
    dispatch_operands: Vec<TypeObservation>,
}

/// A function item's definition and complete arguments identify its type without
/// claiming that its higher-ranked callable signature has been independently proved.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TypeObservation {
    Structural {
        arguments: types::Arguments,
    },
    FunctionItem {
        definition: super::Definition,
        arguments: types::Arguments,
    },
}

struct Lens<'tcx> {
    owner: LocalDefId,
    arguments: GenericArgsRef<'tcx>,
}

impl<'tcx> Lens<'tcx> {
    fn endpoint(&self, tcx: TyCtxt<'tcx>, original: Ty<'tcx>) -> Result<TypeObservation, String> {
        let concrete = projection::normalize(tcx, self.arguments, original)
            .map_err(|error| error.to_string())?;
        if let ty::FnDef(definition, arguments) = concrete.kind() {
            let arguments = projection::complete_arguments(
                tcx,
                projection::normalize(tcx, self.arguments, arguments.skip_binder()),
            )?
            .observation;
            return Ok(TypeObservation::FunctionItem {
                definition: definition_record(tcx, *definition),
                arguments,
            });
        }
        Ok(TypeObservation::Structural {
            arguments: types::endpoint(tcx, concrete)?,
        })
    }

    fn typed(
        &self,
        context: &Context<'tcx>,
        reference: &rstyle_pdg::HirRef,
    ) -> Result<Typed, String> {
        let id = diagnostic_support::hir_id(reference, "Shared-slice projection HIR is not local")?;
        let concrete = projection::project_node(self.owner, context.typeck, id, |ty| {
            projection::normalize(context.tcx, self.arguments, ty)
                .map_err(|error| error.to_string())
        })?;

        let mut adjustments = Vec::new();
        let mut dispatch_operands = Vec::new();
        if let hir::Node::Expr(expression) = context.tcx.hir_node(id) {
            for adjustment in context.typeck.expr_adjustments(expression) {
                adjustments.push(self.endpoint(context.tcx, adjustment.target)?);
            }
            if let Some((_, operands)) = context.dispatch_operands(expression) {
                for operand in operands {
                    dispatch_operands
                        .push(self.endpoint(context.tcx, context.typeck.expr_ty(operand))?);
                }
            }
        }

        Ok(Typed {
            hir: reference.clone(),
            concrete: self.endpoint(context.tcx, concrete)?,
            adjustments,
            dispatch_operands,
        })
    }
}

struct Traversal<'a, 'tcx> {
    context: &'a Context<'tcx>,
    lens: &'a Lens<'tcx>,
    records: Vec<Typed>,
    budget: usize,
}

impl<'a, 'tcx> Traversal<'a, 'tcx> {
    fn capture(
        context: &'a Context<'tcx>,
        lens: &'a Lens<'tcx>,
        node: &HirNode,
    ) -> Result<Vec<Typed>, String> {
        let mut traversal = Self {
            context,
            lens,
            records: Vec::new(),
            budget: 4096,
        };
        traversal.visit(node, 0)?;
        Ok(traversal.records)
    }

    fn visit(&mut self, node: &HirNode, depth: usize) -> Result<(), String> {
        if self.budget == 0 || depth >= 128 {
            return Err("Shared-slice projection exceeds original HIR node budget".into());
        }

        self.budget -= 1;
        if node.ty.is_some() {
            self.records.push(self.lens.typed(self.context, &node.hir)?);
        }

        for child in &node.children {
            self.visit(child, depth + 1)?;
        }

        Ok(())
    }
}

fn observe<'tcx>(
    context: &Context<'tcx>,
    owner: LocalDefId,
    original: &HirFnFacts,
) -> Result<Observation, String> {
    rstyle_pdg::slice_length_instances::source::verify(&original.source.bytes)?;
    if original.bindings.len() > 256 {
        return Err("Shared-slice projection exceeds the original binding budget".into());
    }

    let arguments = context.tcx.mk_args(&[context.tcx.types.u8.into()]);
    let checked = projection::complete_arguments(
        context.tcx,
        projection::normalize(context.tcx, arguments, arguments),
    )?;
    let lens = Lens {
        owner,
        arguments: checked.arguments,
    };

    let parameters = original
        .signature
        .params
        .iter()
        .map(|parameter| lens.typed(context, &parameter.pat.hir))
        .collect::<Result<_, _>>()?;
    let bindings = original
        .bindings
        .iter()
        .map(|binding| lens.typed(context, &binding.hir))
        .collect::<Result<_, _>>()?;
    let nodes = Traversal::capture(context, &lens, &original.body)?;

    Ok(Observation {
        definition: definition_record(context.tcx, owner.to_def_id()),
        type_parameter: definition_record(
            context.tcx,
            context.tcx.generics_of(owner).own_params[0].def_id,
        ),
        owner: crate::hir_extraction::hir::reference(context.tcx.local_def_id_to_hir_id(owner)),
        source_digest: original.source.digest.clone(),
        fn_span: original.fn_span,
        // A deferred generic call can make the ordinary scanner reject this
        // owner. Type observations retain that rejection; they never discharge it.
        original_source_scan_complete: original.source_scan_complete,
        arguments: checked.observation,
        parameters,
        result: lens.endpoint(context.tcx, context.result)?,
        bindings,
        nodes,
        methods: methods::capture(context, &lens, original)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concrete_request_uses_structural_identity_instead_of_display() {
        let mut arguments = json!([{
            "kind":"type", "identity_status":"known",
            "identity":{"kind":"builtin","name":"u8"},
            "value":"display is not authority", "display_only":true,
        }]);
        assert!(u8_arguments(Some(&arguments)));
        arguments[0]["value"] = json!("u8");
        arguments[0]["identity"]["name"] = json!("u16");
        assert!(!u8_arguments(Some(&arguments)));
        arguments[0]["identity"]["name"] = json!("u8");
        arguments[0]["identity_status"] = json!("unsupported");
        assert!(!u8_arguments(Some(&arguments)));
        assert!(!u8_arguments(Some(&json!([]))));
        assert!(!u8_arguments(Some(&json!([arguments[0], arguments[0]]))));
    }
}
