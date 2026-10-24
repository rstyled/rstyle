//! The bounded expansion uses constructor identifiers in its own checked namespace.
//! Independent declaration and namespace-closure receipts supply value ownership.
use super::*;
use crate::concrete_types::Nominal;

pub(super) fn verify(member: &CheckedMember, request: &TransportRequest) -> Result<(), String> {
    let success = request.copy_requests()[1]
        .nominal_identity()
        .ok_or("Transport success callback lacks its nominal constructor owner")?;
    let error = request.cleanup_requests()[1]
        .nominal_identity()
        .ok_or("Transport error callback lacks its nominal constructor owner")?;
    check(member.namespace(), member.function(), [success, error])
}

fn check(
    namespace: &[String],
    function: &syn::ImplItemFn,
    owners: [&Nominal; 2],
) -> Result<(), String> {
    if function.sig.inputs.len() != 1 {
        return Err("Transport callback scope requires exactly one scalar parameter".into());
    }
    let Some(syn::FnArg::Typed(parameter)) = function.sig.inputs.first() else {
        return Err("Transport callback scope requires a scalar parameter".into());
    };
    let syn::Pat::Ident(parameter) = parameter.pat.as_ref() else {
        return Err("Transport callback scope requires a plain parameter binding".into());
    };
    check_leaf_argument(function, &parameter.ident)?;
    for (path, nominal) in paths(function)?.into_iter().zip(owners) {
        let names = crate::source_evidence::plain_relative_path(&path.path)
            .ok_or("Transport callback requires its plain local constructor identifier")?;
        let [name] = names.as_slice() else {
            return Err("Transport callbacks exclude qualified paths and imported aliases".into());
        };
        let expected = namespace
            .iter()
            .cloned()
            .chain([name.clone()])
            .collect::<Vec<_>>()
            .join("::");
        if path.qself.is_some()
            || expected != nominal.definition_path
            || parameter.ident == name.as_str()
        {
            return Err(
                "Transport callback identifier differs from its exact declaration scope".into(),
            );
        }
    }

    Ok(())
}

fn check_leaf_argument(function: &syn::ImplItemFn, parameter: &syn::Ident) -> Result<(), String> {
    let [syn::Stmt::Expr(syn::Expr::MethodCall(outer), None)] = function.block.stmts.as_slice()
    else {
        return Err("Transport leaf lacks its whole source expression".into());
    };
    let syn::Expr::MethodCall(inner) = outer.receiver.as_ref() else {
        return Err("Transport leaf lacks its source map".into());
    };
    let syn::Expr::Call(leaf) = inner.receiver.as_ref() else {
        return Err("Transport leaf requires a direct source call".into());
    };
    let Some(syn::Expr::Path(argument)) = leaf.args.first() else {
        return Err("Transport leaf requires exactly one source parameter argument".into());
    };
    if leaf.args.len() != 1 || argument.qself.is_some() || !argument.path.is_ident(parameter) {
        return Err("Transport leaf source argument differs from its exact parameter".into());
    }

    Ok(())
}

fn paths(function: &syn::ImplItemFn) -> Result<[&syn::ExprPath; 2], String> {
    let [syn::Stmt::Expr(syn::Expr::MethodCall(outer), None)] = function.block.stmts.as_slice()
    else {
        return Err("Transport callback source has no exact whole tail expression".into());
    };
    let syn::Expr::MethodCall(inner) = outer.receiver.as_ref() else {
        return Err("Transport callback source has no exact inner map".into());
    };
    let (Some(syn::Expr::Path(success)), Some(syn::Expr::Path(error))) =
        (inner.args.first(), outer.args.first())
    else {
        return Err("Transport callbacks must be direct constructor paths".into());
    };
    if inner.method != "map"
        || outer.method != "map_err"
        || inner.args.len() != 1
        || outer.args.len() != 1
    {
        return Err("Transport callback source changes method or callback arity".into());
    }

    Ok([success, error])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nominal(name: &str) -> Nominal {
        Nominal {
            source_identity: None,
            package_name: "sample".into(),
            package_version: "1".into(),
            manifest_digest: "manifest".into(),
            definition_path: format!("model::{name}"),
            declaration_digest: "declaration".into(),
        }
    }

    fn verify_source(source: &str) -> Result<(), String> {
        let function = syn::parse_str(source).unwrap();
        check(
            &["model".into()],
            &function,
            [&nominal("Wrapper"), &nominal("Failure")],
        )
    }

    #[test]
    fn both_callback_identifiers_bind_to_their_exact_nominal_namespace() {
        assert!(
            verify_source(
                "fn new(value: usize) {
            Base::new(value).map(Wrapper).map_err(Failure) }"
            )
            .is_ok()
        );
    }

    #[test]
    fn neighboring_alias_qualified_and_parameter_shadow_paths_are_rejected() {
        for source in [
            "fn new(value: usize) { Base::new(value).map(Other).map_err(Failure) }",
            "fn new(value: usize) { Base::new(value).map(Wrapper).map_err(Other) }",
            "fn new(value: usize) { Base::new(value).map(self::Wrapper).map_err(Failure) }",
            "fn new(value: usize) { Base::new(value).map(Wrapper).map_err(alias::Failure) }",
            "fn new(Wrapper: usize) { Base::new(Wrapper).map(Wrapper).map_err(Failure) }",
            "fn new(Failure: usize) { Base::new(Failure).map(Wrapper).map_err(Failure) }",
        ] {
            assert!(verify_source(source).is_err(), "{source}");
        }
    }

    #[test]
    fn leaf_source_argument_must_name_the_exact_plain_parameter() {
        for argument in [
            "OTHER",
            "self::value",
            "value::<usize>",
            "work(value)",
            "value, value",
        ] {
            let source = format!(
                "fn new(value: usize) {{ Base::new({argument}).map(Wrapper).map_err(Failure) }}"
            );
            assert!(verify_source(&source).is_err(), "{source}");
        }
    }

    #[test]
    fn checked_generated_roles_cannot_replace_source_parameter_binding() {
        let mut fixture = crate::generated_sources::checked::tests::fixture::Fixture::new();
        fixture.change_source("SmallIndex::new(value)", "SmallIndex::new(OTHER)");
        let member = fixture.verify().unwrap();
        let mut success = nominal("PatternID");
        let mut error = nominal("PatternIDError");
        success.definition_path = "util::primitives::PatternID".into();
        error.definition_path = "util::primitives::PatternIDError".into();
        assert!(check(member.namespace(), member.function(), [&success, &error]).is_err());
    }
}
