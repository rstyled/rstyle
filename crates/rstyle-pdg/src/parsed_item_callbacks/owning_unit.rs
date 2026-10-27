//! Same owning build/source evidence join, without certification or foreign fallback.
use super::*;
use crate::{HirFnFacts, nominal_boundary::census::Census};
use std::sync::Arc;

pub(super) fn verify(
    input: &HirFnFacts,
    original: &HirFnFacts,
    call: &Value,
    context: &Context<'_>,
) -> Result<(), String> {
    let identity = original
        .build
        .frontend_options
        .get("definition_identity")
        .and_then(Value::as_str);
    let member_name = context.member.path.rsplit("::").next();
    if identity != Some(context.member.definition_identity.as_str())
        || member_name != original.name.rsplit("::").next()
        || !same_build(&input.build, &original.build)
    {
        return Err("Callback original member definition or owning build disagrees".into());
    }

    let consumer = input.build.frontend_options.data::<Arc<Census>>()?;
    let producer = original.build.frontend_options.data::<Arc<Census>>()?;
    if (!Arc::ptr_eq(&consumer, &producer) && consumer != producer)
        || consumer.identity != crate::nominal_boundary::census::IDENTITY
        || consumer.context.crate_name != context.member.crate_name
    {
        return Err(
            "Callback original member requires its exact owning unit census/package".into(),
        );
    }

    selected_context(input, &consumer)?;
    selected_context(original, &consumer)?;
    let inventory = input
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Missing separately scoped callback inventory")?;
    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
        || !scoped_call(inventory, call)
    {
        return Err(
            "Callback call is absent from its complete separately scoped owning inventory".into(),
        );
    }
    Ok(())
}

fn selected_context(facts: &HirFnFacts, census: &Census) -> Result<(), String> {
    let value = facts
        .build
        .frontend_options
        .get(crate::nominal_boundary::CONTEXT_KEY)
        .ok_or("Missing callback selected unit context")?;
    let selected: crate::nominal_boundary::UnitContext =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    if selected != census.context {
        return Err("Callback selected package/source context disagrees with its census".into());
    }
    Ok(())
}

fn scoped_call(inventory: &Value, call: &Value) -> bool {
    let Some(callbacks) = inventory["callbacks"].as_array() else {
        return false;
    };
    let Some(hir) = call.get("hir").filter(|value| value.is_object()) else {
        return false;
    };
    let mut occurrences = callbacks
        .iter()
        .filter(|callback| {
            callback["scope"]
                == "immediate-core-option-and-then-callback-owning-body; nested owners excluded"
        })
        .filter_map(|callback| callback["calls"].as_array())
        .flatten()
        .filter(|item| item.get("hir") == Some(hir));

    occurrences.next().is_some_and(|item| item == call) && occurrences.next().is_none()
}

fn same_build(left: &crate::frontend::BuildFacts, right: &crate::frontend::BuildFacts) -> bool {
    [
        left.producer_revision == right.producer_revision,
        left.rustc_version == right.rustc_version,
        left.rustc_commit == right.rustc_commit,
        left.edition == right.edition,
        left.target_triple == right.target_triple,
        left.pointer_width == right.pointer_width,
        left.data_layout == right.data_layout,
        left.cfgs == right.cfgs,
        left.features == right.features,
        left.overflow_checks == right.overflow_checks,
        left.debug_assertions == right.debug_assertions,
        left.panic_strategy == right.panic_strategy,
        left.opt_level == right.opt_level,
        left.frontend_options.get("invocation") == right.frontend_options.get("invocation"),
    ]
    .into_iter()
    .all(|equal| equal)
}
