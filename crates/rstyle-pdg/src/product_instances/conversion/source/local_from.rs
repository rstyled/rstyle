//! Join diagnostic selection to the actual enriched original body and lexical owner.
use super::*;
use crate::{BindingFact, ByteSpan, SignatureFacts, SourceFacts};
use serde::Deserialize;
use syn::spanned::Spanned;

mod enrichment;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Original {
    definition: Definition,
    source: SourceFacts,
    fn_span: ByteSpan,
    signature: SignatureFacts,
    bindings: Vec<BindingFact>,
    body: HirNode,
    source_diagnostics: Vec<crate::HirDiagnostic>,
    source_scan_complete: bool,
}

pub(in super::super) fn verify(
    caller: &HirFnFacts,
    local: &HirFnFacts,
    diagnostic: &Value,
    selection: &super::super::model::Selection,
) -> Result<(), String> {
    let original: Original =
        serde_json::from_value(diagnostic.clone()).map_err(|error| error.to_string())?;

    context(caller, local)?;
    super::super::authority::local_definition(local, &selection.selected)?;
    if original.definition != selection.selected
        || local.name != selection.selected.path
        || local
            .build
            .frontend_options
            .get("definition_identity")
            .and_then(Value::as_str)
            != Some(selection.selected.definition_identity.as_str())
        || original.source != local.source
        || original.fn_span != local.fn_span
        || original.source_diagnostics != local.source_diagnostics
        || original.source_scan_complete != local.source_scan_complete
    {
        return Err(
            "Selected local From differs from its enriched original definition/source".into(),
        );
    }

    enrichment::verify(&original, local)?;
    lexical(local, &selection.selected_impl)
}

fn context(caller: &HirFnFacts, local: &HirFnFacts) -> Result<(), String> {
    if !crate::helper_references::same_unit(caller, local)?
        || caller.build.pointer_width != local.build.pointer_width
        || caller.build.overflow_checks != local.build.overflow_checks
        || caller.build.debug_assertions != local.build.debug_assertions
        || caller.build.panic_strategy != local.build.panic_strategy
        || caller
            .build
            .frontend_options
            .get("rstyle.rust.compiled-crate")
            != local
                .build
                .frontend_options
                .get("rstyle.rust.compiled-crate")
    {
        return Err(
            "Local From and original constructor use different compiled build contexts".into(),
        );
    }
    Ok(())
}

fn lexical(local: &HirFnFacts, implementation: &Definition) -> Result<(), String> {
    super::super::authority::local_definition(local, implementation)?;
    if local.name != format!("{}::from", implementation.path) {
        return Err("Selected local From method contradicts its implementation path".into());
    }

    let census = local
        .build
        .frontend_options
        .data::<std::sync::Arc<crate::nominal_boundary::census::Census>>()?;
    crate::nominal_boundary::census::check_budget(&census)?;
    let file = local
        .build
        .frontend_options
        .get("source_file")
        .and_then(Value::as_str)
        .ok_or("Selected local From lacks its source file")?;
    let files: Vec<_> = census
        .files
        .iter()
        .filter(|source| source.path == std::path::Path::new(file))
        .collect();
    let [file] = files.as_slice() else {
        return Err("Selected local From source file is absent or ambiguous".into());
    };
    if file
        .bytes
        .get(local.fn_span.start as usize..local.fn_span.end as usize)
        != Some(local.source.bytes.as_str())
    {
        return Err("Local From source is not the exact retained source-file interval".into());
    }

    let source = syn::parse_file(&file.bytes).map_err(|error| error.to_string())?;
    let mut owners = Vec::new();
    enclosing(&source.items, &file.bytes, local.fn_span, &mut owners);

    let [owner] = owners.as_slice() else {
        return Err("Local From lacks one exact original lexical implementation owner".into());
    };
    owner_shape(owner)
}

fn enclosing<'a>(
    items: &'a [syn::Item],
    source: &str,
    target: ByteSpan,
    owners: &mut Vec<&'a syn::ItemImpl>,
) {
    for item in items {
        match item {
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    enclosing(items, source, target, owners);
                }
            }
            syn::Item::Impl(owner) => {
                for member in &owner.items {
                    if let syn::ImplItem::Fn(method) = member {
                        let range = ByteSpan {
                            start: crate::source_evidence::original_span(source, method.sig.span())
                                .start,
                            end: crate::source_evidence::original_span(source, method.block.span())
                                .end,
                        };
                        if range == target {
                            owners.push(owner);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Exact method identity plus lexical ownership authorizes the join. The unjoined
/// selected_impl fingerprint remains diagnostic; it cannot authorize another body.
fn owner_shape(owner: &syn::ItemImpl) -> Result<(), String> {
    if owner.trait_.is_none()
        || owner.unsafety.is_some()
        || owner.defaultness.is_some()
        || !owner.generics.params.is_empty()
        || owner.generics.where_clause.is_some()
        || owner.items.len() != 1
    {
        return Err("Local Range From requires one safe concrete trait implementation".into());
    }
    Ok(())
}
