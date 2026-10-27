//! Exact compiler/tool metadata never replaces traversal of the annotated item.
use super::*;

pub(super) fn accepts(attribute: &syn::Meta) -> bool {
    census::module_lints::expectation(attribute)
        || matches!(attribute, syn::Meta::Path(path)
            if crate::source_evidence::plain_relative_path(path)
                == Some(vec!["rustfmt".into(), "skip".into()]))
        || diagnostic_note(attribute)
}

fn diagnostic_note(attribute: &syn::Meta) -> bool {
    let syn::Meta::List(list) = attribute else {
        return false;
    };
    if crate::source_evidence::plain_relative_path(&list.path)
        != Some(vec!["diagnostic".into(), "on_unimplemented".into()])
    {
        return false;
    }
    let Ok(note) = list.parse_args::<syn::MetaNameValue>() else {
        return false;
    };
    let syn::Expr::Lit(syn::ExprLit {
        attrs,
        lit: syn::Lit::Str(value),
    }) = note.value
    else {
        return false;
    };

    note.path.is_ident("note")
        && attrs.is_empty()
        && !value.value().is_empty()
        && value.value().len() <= 512
}
