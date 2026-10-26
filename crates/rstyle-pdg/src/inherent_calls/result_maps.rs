//! Exact generated static leaf spelling; this supplies no helper/body authority.
use crate::{HirNode, concrete_types::Nominal, unit_store::EvidenceFacts};

pub(super) fn verify(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    expected: Option<&Nominal>,
) -> Result<Option<bool>, String> {
    if node.kind != "expr_call"
        || !facts
            .body
            .any(&|node| crate::result_maps::candidate(node, facts))
    {
        return Ok(None);
    }
    let resolution = facts.nominal_resolution()?;
    let source = resolution
        .transport_source()
        .ok_or("Missing checked generated leaf source")?;
    source.verify(facts.facts())?;
    let role = source
        .member()
        .roles()
        .iter()
        .find(|role| role.path() == [1, 0, 0, 0])
        .ok_or("Missing exact generated leaf role")?;
    if role.hir() != &node.hir || role.kind() != node.kind {
        return Err("Generated helper call is not its exact checked source leaf".into());
    }
    let call = leaf(source.member().function())?;
    let nominal = source.request().copy_requests()[0]
        .nominal_identity()
        .ok_or("Generated leaf success lacks its exact static nominal owner")?;
    if expected.is_some_and(|expected| expected != nominal) {
        return Ok(Some(false));
    }
    let selected = crate::helper_calls::direct_resolution(node)
        .ok_or("Generated leaf lacks direct selected definition")?;
    let path = selected["path"]
        .as_str()
        .ok_or("Generated leaf lacks selected path")?;
    let owner = owner(call, path, nominal)?;
    resolution.matches_nominal(&owner, nominal)?;

    Ok(Some(true))
}

pub(crate) fn leaf(function: &syn::ImplItemFn) -> Result<&syn::ExprCall, String> {
    let [syn::Stmt::Expr(syn::Expr::MethodCall(outer), None)] = function.block.stmts.as_slice()
    else {
        return Err("Generated helper requires one whole transport expression".into());
    };
    let syn::Expr::MethodCall(inner) = outer.receiver.as_ref() else {
        return Err("Generated helper lacks its exact inner map".into());
    };
    let syn::Expr::Call(call) = inner.receiver.as_ref() else {
        return Err("Generated helper receiver is not a direct leaf call".into());
    };
    if outer.method != "map_err"
        || inner.method != "map"
        || outer.turbofish.is_some()
        || inner.turbofish.is_some()
        || !matches!(outer.args.first(), Some(syn::Expr::Path(_)))
        || outer.args.len() != 1
        || !matches!(inner.args.first(), Some(syn::Expr::Path(_)))
        || inner.args.len() != 1
        || call.args.len() != 1
        || !matches!(call.args.first(), Some(syn::Expr::Path(_)))
    {
        return Err("Generated helper source contains extra or unrelated work".into());
    }

    Ok(call)
}

fn owner(call: &syn::ExprCall, selected: &str, nominal: &Nominal) -> Result<syn::TypePath, String> {
    let source = call_owner(call)?;
    if selected != format!("{}::{}", nominal.definition_path, source.method) {
        return Err(
            "Generated helper selected definition differs from its exact source owner/method"
                .into(),
        );
    }
    Ok(source.owner)
}

pub(crate) struct SourceCall {
    pub owner: syn::TypePath,
    pub method: String,
}

pub(crate) fn call_owner(call: &syn::ExprCall) -> Result<SourceCall, String> {
    let syn::Expr::Path(path) = call.func.as_ref() else {
        return Err("Generated helper callee is not a concrete static path".into());
    };
    let Some(names) = crate::source_evidence::plain_relative_path(&path.path) else {
        return Err("Generated helper callee has generic or qualified syntax".into());
    };
    let method = names
        .last()
        .ok_or("Generated helper callee has no method")?;
    if path.qself.is_some() || names.len() < 2 {
        return Err(
            "Generated helper selected definition differs from its exact source owner/method"
                .into(),
        );
    }
    let mut owner = path.path.clone();
    owner.segments.pop();
    owner.segments.pop_punct();

    Ok(SourceCall {
        owner: syn::TypePath {
            qself: None,
            path: owner,
        },
        method: method.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nominal() -> Nominal {
        Nominal {
            source_identity: None,
            package_name: "sample".into(),
            package_version: "1".into(),
            manifest_digest: "manifest".into(),
            definition_path: "model::Base".into(),
            declaration_digest: "declaration".into(),
        }
    }

    #[test]
    fn generated_static_leaf_keeps_complete_owner_and_method() {
        let function: syn::ImplItemFn =
            syn::parse_str("fn new(value: usize) { Base::new(value).map(Wrapper).map_err(Error) }")
                .unwrap();
        let call = leaf(&function).unwrap();
        assert!(owner(call, "model::Base::new", &nominal()).is_ok());
        assert!(owner(call, "other::Base::new", &nominal()).is_err());
        assert!(owner(call, "model::Base::other", &nominal()).is_err());
    }

    #[test]
    fn generated_static_leaf_rejects_surplus_and_computed_work() {
        for expression in [
            "Base::new(value, value).map(Wrapper).map_err(Error)",
            "Base::new(work(value)).map(Wrapper).map_err(Error)",
            "Base::new(value).map(work()).map_err(Error)",
            "Base::new(value).map(Wrapper).map_err(Error, Other)",
            "Base::new(value).other(Wrapper).map_err(Error)",
        ] {
            let function =
                syn::parse_str(&format!("fn new(value: usize) {{ {expression} }}")).unwrap();
            assert!(leaf(&function).is_err(), "{expression}");
        }
    }
}
