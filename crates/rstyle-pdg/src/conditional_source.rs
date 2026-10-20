//! Evaluate source-removal predicates against the actual compiler cfg set.

use std::collections::BTreeSet;
use syn::ext::IdentExt;
use syn::{Meta, Token, punctuated::Punctuated};

pub struct Configuration<'a> {
    values: BTreeSet<&'a str>,
}

impl<'a> Configuration<'a> {
    pub fn new(values: &'a [String]) -> Self {
        Self {
            values: values.iter().map(String::as_str).collect(),
        }
    }

    pub fn active(&self, attributes: &[syn::Attribute]) -> Result<bool, String> {
        let mut unknown = None;
        for attribute in attributes {
            match self.attribute(&attribute.meta) {
                Ok(false) => return Ok(false),
                Err(error) => unknown = Some(error),
                Ok(true) => {}
            }
        }
        unknown.map_or(Ok(true), Err)
    }

    /// Return active attribute metadata without hiding non-cfg attributes.
    /// Item presence must still be checked with `active`; this is not a proof
    /// that an attribute's expansion or effects are supported.
    pub fn effective_attributes(&self, attributes: &[syn::Attribute]) -> Result<Vec<Meta>, String> {
        let mut effective = Vec::new();
        for attribute in attributes {
            self.expand_attribute(&attribute.meta, &mut effective)?;
        }

        Ok(effective)
    }

    fn expand_attribute(&self, meta: &Meta, effective: &mut Vec<Meta>) -> Result<(), String> {
        if !meta.path().is_ident("cfg_attr") {
            effective.push(meta.clone());
            return Ok(());
        }
        let Meta::List(list) = meta else {
            return Err("Malformed cfg_attr attribute".into());
        };
        let arguments = list
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .map_err(|error| error.to_string())?;
        let mut arguments = arguments.iter();
        let condition = arguments.next().ok_or("Missing cfg_attr condition")?;
        if self.predicate(condition)? {
            for attribute in arguments {
                self.expand_attribute(attribute, effective)?;
            }
        }

        Ok(())
    }

    fn attribute(&self, meta: &Meta) -> Result<bool, String> {
        if meta.path().is_ident("cfg") {
            let Meta::List(list) = meta else {
                return Err("Malformed cfg attribute".into());
            };
            return self.predicate(
                &list
                    .parse_args::<Meta>()
                    .map_err(|error| error.to_string())?,
            );
        }
        if meta.path().is_ident("cfg_attr") {
            let Meta::List(list) = meta else {
                return Err("Malformed cfg_attr attribute".into());
            };
            let arguments = list
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .map_err(|error| error.to_string())?;
            let mut arguments = arguments.iter();
            let condition = arguments.next().ok_or("Missing cfg_attr condition")?;
            if !self.predicate(condition)? {
                return Ok(true);
            }
            for attribute in arguments {
                if !self.attribute(attribute)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub fn predicate(&self, meta: &Meta) -> Result<bool, String> {
        match meta {
            Meta::Path(path) if path.segments.len() == 1 => Ok(self
                .values
                .contains(path.segments[0].ident.unraw().to_string().as_str())),
            Meta::NameValue(value) if value.path.segments.len() == 1 => {
                let syn::Expr::Lit(literal) = &value.value else {
                    return Err("Nonliteral cfg value".into());
                };
                let syn::Lit::Str(value_text) = &literal.lit else {
                    return Err("Nonstring cfg value".into());
                };
                Ok(self.values.contains(
                    format!(
                        "{}=\"{}\"",
                        value.path.segments[0].ident.unraw(),
                        value_text.value()
                    )
                    .as_str(),
                ))
            }
            Meta::List(list) => self.list(list),
            _ => Err("Unsupported cfg predicate".into()),
        }
    }

    fn list(&self, list: &syn::MetaList) -> Result<bool, String> {
        let arguments = list
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .map_err(|error| error.to_string())?;
        let values = arguments
            .iter()
            .map(|meta| self.predicate(meta))
            .collect::<Result<Vec<_>, _>>()?;

        if list.path.is_ident("all") {
            return Ok(values.into_iter().all(|value| value));
        }
        if list.path.is_ident("any") {
            return Ok(values.into_iter().any(|value| value));
        }
        if list.path.is_ident("not") && values.len() == 1 {
            return Ok(!values[0]);
        }
        Err("Unsupported cfg predicate operator or arity".into())
    }
}

pub fn expression_attributes(expression: &syn::Expr) -> &[syn::Attribute] {
    match expression {
        syn::Expr::Array(value) => &value.attrs,
        syn::Expr::Assign(value) => &value.attrs,
        syn::Expr::Async(value) => &value.attrs,
        syn::Expr::Await(value) => &value.attrs,
        syn::Expr::Binary(value) => &value.attrs,
        syn::Expr::Block(value) => &value.attrs,
        syn::Expr::Break(value) => &value.attrs,
        syn::Expr::Call(value) => &value.attrs,
        syn::Expr::Cast(value) => &value.attrs,
        syn::Expr::Closure(value) => &value.attrs,
        syn::Expr::Const(value) => &value.attrs,
        syn::Expr::Continue(value) => &value.attrs,
        syn::Expr::Field(value) => &value.attrs,
        syn::Expr::ForLoop(value) => &value.attrs,
        syn::Expr::Group(value) => &value.attrs,
        syn::Expr::If(value) => &value.attrs,
        syn::Expr::Index(value) => &value.attrs,
        syn::Expr::Infer(value) => &value.attrs,
        syn::Expr::Let(value) => &value.attrs,
        syn::Expr::Lit(value) => &value.attrs,
        syn::Expr::Loop(value) => &value.attrs,
        syn::Expr::Macro(value) => &value.attrs,
        syn::Expr::Match(value) => &value.attrs,
        syn::Expr::MethodCall(value) => &value.attrs,
        syn::Expr::Paren(value) => &value.attrs,
        syn::Expr::Path(value) => &value.attrs,
        syn::Expr::Range(value) => &value.attrs,
        syn::Expr::RawAddr(value) => &value.attrs,
        syn::Expr::Reference(value) => &value.attrs,
        syn::Expr::Repeat(value) => &value.attrs,
        syn::Expr::Return(value) => &value.attrs,
        syn::Expr::Struct(value) => &value.attrs,
        syn::Expr::Try(value) => &value.attrs,
        syn::Expr::TryBlock(value) => &value.attrs,
        syn::Expr::Tuple(value) => &value.attrs,
        syn::Expr::Unary(value) => &value.attrs,
        syn::Expr::Unsafe(value) => &value.attrs,
        syn::Expr::While(value) => &value.attrs,
        syn::Expr::Yield(value) => &value.attrs,
        _ => &[],
    }
}

pub fn item_attributes(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(value) => &value.attrs,
        syn::Item::Enum(value) => &value.attrs,
        syn::Item::ExternCrate(value) => &value.attrs,
        syn::Item::Fn(value) => &value.attrs,
        syn::Item::ForeignMod(value) => &value.attrs,
        syn::Item::Impl(value) => &value.attrs,
        syn::Item::Macro(value) => &value.attrs,
        syn::Item::Mod(value) => &value.attrs,
        syn::Item::Static(value) => &value.attrs,
        syn::Item::Struct(value) => &value.attrs,
        syn::Item::Trait(value) => &value.attrs,
        syn::Item::TraitAlias(value) => &value.attrs,
        syn::Item::Type(value) => &value.attrs,
        syn::Item::Union(value) => &value.attrs,
        syn::Item::Use(value) => &value.attrs,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditional_metadata_preserves_active_derive_and_other_effects() {
        let item: syn::ItemStruct = syn::parse_quote! {
            #[derive(Copy)]
            #[cfg_attr(feature = "custom", derive(custom::Dangerous), path = "other.rs")]
            #[cfg_attr(not(feature = "custom"), cfg_attr(unix, derive(Clone)))]
            struct Data;
        };
        let values = vec!["unix".into()];
        let configuration = Configuration::new(&values);
        let effective = configuration.effective_attributes(&item.attrs).unwrap();
        assert_eq!(effective.len(), 2);
        assert!(effective.iter().all(|meta| meta.path().is_ident("derive")));
        let Meta::List(last) = &effective[1] else {
            panic!("derive list")
        };
        assert_eq!(last.tokens.to_string(), "Clone");

        let values = vec!["unix".into(), "feature=\"custom\"".into()];
        let effective = Configuration::new(&values)
            .effective_attributes(&item.attrs)
            .unwrap();
        assert_eq!(effective.len(), 3);
        let Meta::List(custom) = &effective[1] else {
            panic!("derive list")
        };
        assert_eq!(custom.tokens.to_string(), "custom :: Dangerous");
        assert!(effective[2].path().is_ident("path"));
    }

    #[test]
    fn effective_metadata_does_not_certify_presence_or_unknown_predicates() {
        let item: syn::ItemStruct = syn::parse_quote! {
            #[cfg(any())]
            #[cfg_attr(all(), derive(custom::Dangerous))]
            struct Data;
        };
        let configuration = Configuration::new(&[]);
        assert!(!configuration.active(&item.attrs).unwrap());
        let effective = configuration.effective_attributes(&item.attrs).unwrap();
        assert_eq!(effective.len(), 2);
        assert!(effective[0].path().is_ident("cfg"));
        assert!(effective[1].path().is_ident("derive"));

        let malformed: syn::ItemStruct = syn::parse_quote! {
            #[cfg_attr(unknown_operator(), derive(Copy))]
            struct Data;
        };
        assert!(
            configuration
                .effective_attributes(&malformed.attrs)
                .is_err()
        );
    }
}
