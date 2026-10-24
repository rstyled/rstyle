//! Prove exact core Default dispatch before ordinary callee graph verification.
use super::{model, source};
use crate::{HirNode, concrete_types::Nominal, unit_store::EvidenceFacts};

pub(crate) fn verify(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
    resolution: &serde_json::Value,
) -> Result<(), String> {
    let selection = resolution.get("default_selection");
    if selection.is_none() && !callee.build.frontend_options.contains_key(super::KEY) {
        return Ok(());
    }
    if !crate::extraction_profile::facts_support(caller, super::CAPABILITY) {
        return Err("Default call requires its registered concrete capability".into());
    }

    let observation = super::observation(callee)?
        .ok_or("Default selection lacks captured original owning member")?;
    let selection: model::Selection = serde_json::from_value(
        selection
            .cloned()
            .ok_or("Default owning member lacks exact call selection")?,
    )
    .map_err(|error| error.to_string())?;
    let tree = super::result(callee)?;
    let (nominal, _) = tree
        .nominal_parts()
        .ok_or("Default call lacks exact nominal result")?;
    let checker = Checker {
        node,
        caller,
        callee,
        resolution,
        nominal,
    };

    checker.selection(&selection, &observation)?;
    checker.signature(&tree)?;
    checker.source()
}

struct Checker<'a> {
    node: &'a HirNode,
    caller: &'a dyn EvidenceFacts,
    callee: &'a dyn EvidenceFacts,
    resolution: &'a serde_json::Value,
    nominal: &'a Nominal,
}

impl Checker<'_> {
    fn selection(
        &self,
        selection: &model::Selection,
        owner: &model::Observation,
    ) -> Result<(), String> {
        super::authority(
            &selection.requested,
            self.caller,
            "rust.concrete-default.method",
        )?;
        super::authority(
            &selection.trait_definition,
            self.caller,
            "rust.concrete-default.trait",
        )?;

        self.original_dispatch(selection)?;
        self.selected_types(selection, owner)?;
        if selection.identity != "rstyle.rust.concrete-default-call-observation/1"
            || selection.admission_authority
            || selection.hir != self.node.hir
            || selection.span != self.node.span
            || selection.requested != owner.trait_method
            || selection.trait_definition != owner.trait_definition
            || selection.selected != owner.method
            || owner.selected != owner.method
            || selection.implementation != owner.implementation
            || self.resolution["path"].as_str() != Some(selection.selected.path.as_str())
            || self.resolution["crate"].as_str() != Some(selection.selected.crate_name.as_str())
            || self.resolution["definition_identity"].as_str()
                != Some(selection.selected.definition_identity.as_str())
        {
            return Err(
                "Default call selection contradicts exact original owning implementation".into(),
            );
        }
        Ok(())
    }

    fn original_dispatch(&self, selection: &model::Selection) -> Result<(), String> {
        let inventory = self
            .caller
            .build
            .frontend_options
            .get("rstyle.rust.call-instances")
            .ok_or("Default call lacks its independent original call inventory")?;
        let original = original_call(inventory, &self.node.hir, self.node.span)?;
        if original.requested != selection.requested
            || original.selected != selection.selected
            || !original.selected_arguments.is_empty()
            || model::nominal(&original.requested_arguments, self.nominal)?
                != model::nominal(&selection.requested_arguments, self.nominal)?
        {
            return Err(
                "Default marker contradicts independently captured original dispatch".into(),
            );
        }
        Ok(())
    }

    fn selected_types(
        &self,
        selection: &model::Selection,
        owner: &model::Observation,
    ) -> Result<(), String> {
        source::definition(self.callee, &selection.selected)?;
        source::local(self.callee, &selection.implementation)?;
        let nominal = model::nominal(&owner.trait_arguments, self.nominal)?;
        source::local(self.callee, &nominal)?;

        if !selection.selected_arguments.is_empty()
            || !owner.selected_arguments.is_empty()
            || owner.associated_error.is_some()
            || owner.method_header.is_const
            || owner.method_header.is_async
            || model::nominal(&selection.requested_arguments, self.nominal)? != nominal
            || selection.requested_signature.verify(self.nominal)? != nominal
            || selection.selected_signature.verify(self.nominal)? != nominal
            || owner.requested_signature.verify(self.nominal)? != nominal
            || owner.selected_signature.verify(self.nominal)? != nominal
        {
            return Err(
                "Default selected signatures differ from their original concrete owner".into(),
            );
        }
        Ok(())
    }

    fn signature(&self, tree: &crate::concrete_types::TypeTree) -> Result<(), String> {
        let token = crate::data_types::token(tree)?;
        let node = self.node;
        if node.kind != "expr_call"
            || node.expansion.is_some()
            || !node.children.is_empty()
            || !self.callee.signature.params.is_empty()
            || self.callee.signature.has_self
            || node.res.as_ref().is_none_or(|resolution| {
                resolution.kind != "resolved_direct_callee" || resolution.target.is_some()
            })
            || node.dispatch.as_ref().is_none_or(|dispatch| {
                dispatch.kind != "resolved_direct"
                    || !dispatch.operand_tys.is_empty()
                    || dispatch.result_ty != token
                    || dispatch.op != self.callee.name
            })
            || crate::data_aggregates::token(node)? != token
            || !self.callee.nominal_resolution()?.checked_owned_copy(tree)
        {
            return Err(
                "Default call lacks its original zero-argument Copy product signature".into(),
            );
        }
        Ok(())
    }

    fn source(&self) -> Result<(), String> {
        let syntax = crate::source_evidence::original_expression(self.node, self.caller)?;
        let syn::Expr::Call(call) = crate::data_aggregates::ungroup(&syntax) else {
            return Err("Default requires an original direct invocation".into());
        };
        let syn::Expr::Path(path) = crate::data_aggregates::ungroup(&call.func) else {
            return Err("Default requires an original concrete owner path".into());
        };
        if !call.args.is_empty()
            || path.qself.is_some()
            || path
                .path
                .segments
                .iter()
                .any(|segment| !segment.arguments.is_none())
            || path
                .path
                .segments
                .last()
                .is_none_or(|segment| segment.ident != "default")
        {
            return Err(
                "Default source invocation has unsupported arguments or dispatch syntax".into(),
            );
        }

        let mut owner = path.path.clone();
        owner.segments.pop();
        owner.segments.pop_punct();
        if owner.segments.is_empty() {
            return Err("Default invocation lacks an explicit concrete owner".into());
        }
        self.caller.nominal_resolution()?.default_call_owner(
            self.caller.facts(),
            &syn::TypePath {
                qself: None,
                path: owner,
            },
            self.nominal,
        )
    }
}

/// Decode only the selected site's record; other calls may contain unrelated
/// structural generic kinds outside this bounded concrete ADT contract.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OriginalCall {
    hir: crate::HirRef,
    span: crate::ByteSpan,
    expanded: bool,
    status: String,
    requested: model::Definition,
    requested_arguments: Vec<model::Argument>,
    selected: model::Definition,
    selected_arguments: Vec<model::Argument>,
    signature: serde_json::Value,
    #[serde(default)]
    compiled_dependency: Option<serde_json::Value>,
}

pub(super) fn original_call(
    inventory: &serde_json::Value,
    hir: &crate::HirRef,
    span: crate::ByteSpan,
) -> Result<OriginalCall, String> {
    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
    {
        return Err("Default requires the complete independent original call inventory".into());
    }
    let calls = inventory["calls"]
        .as_array()
        .ok_or("Default original call roster missing")?;
    let mut matching = calls.iter().filter(|call| {
        serde_json::from_value::<crate::HirRef>(call["hir"].clone())
            .ok()
            .as_ref()
            == Some(hir)
            || serde_json::from_value::<crate::ByteSpan>(call["span"].clone()).ok() == Some(span)
    });
    let selected = matching
        .next()
        .ok_or("Default original call observation missing")?;
    if matching.next().is_some() {
        return Err("Default original call observation duplicated or ambiguously anchored".into());
    }

    let call: OriginalCall =
        serde_json::from_value(selected.clone()).map_err(|error| error.to_string())?;
    if call.hir != *hir
        || call.span != span
        || call.expanded
        || call.status != "resolved"
        || call.compiled_dependency.is_some()
        || call.signature["display_only"] != true
        || call.signature["inputs"]
            .as_array()
            .is_none_or(|inputs| !inputs.is_empty())
        || call.signature["output"].as_str().is_none_or(str::is_empty)
    {
        return Err(
            "Default original call is not its exact resolved nonexpanded local selection".into(),
        );
    }
    Ok(call)
}
