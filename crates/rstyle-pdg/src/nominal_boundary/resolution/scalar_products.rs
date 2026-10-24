//! Exact scalar product instances retain their own source receipt, never nominal permissions.
use super::*;
use selected::no_drop::scalar_product_instances as proof;

impl Resolution {
    pub(super) fn capture_scalar_product(&mut self, facts: &HirFnFacts) -> Result<(), String> {
        let instances = self
            .trees
            .values()
            .filter_map(|tree| match tree {
                TypeTree::ScalarProductInstance(instance) => Some(instance.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let instance = match instances.as_slice() {
            [] => return Ok(()),
            [instance] => *instance,
            _ => {
                return Err(
                    "Multiple scalar product instances require separate source receipts".into(),
                );
            }
        };
        if !supports(facts, crate::slice_patterns::CAPABILITY)? {
            return Err("Scalar product source requires its registered capture capability".into());
        }

        let origins = facts
            .build
            .frontend_options
            .data::<declaration_origins::Origins>()?;
        self.scalar_product_source = Some(proof::capture_checked(facts, self, &origins, instance)?);
        Ok(())
    }

    pub(crate) fn checked_scalar_product(
        &self,
        facts: &HirFnFacts,
        tree: &TypeTree,
    ) -> Result<bool, String> {
        let TypeTree::ScalarProductInstance(instance) = tree else {
            return Ok(false);
        };
        let receipt = self
            .scalar_product_source
            .as_ref()
            .ok_or("Scalar product lacks its exact source/cleanup receipt")?;
        receipt.verify_owner(facts, &self.environment, instance)?;
        Ok(true)
    }

    pub(super) fn bind_scalar_product(&mut self, facts: &HirFnFacts) -> Result<bool, String> {
        let Some(receipt) = &self.scalar_product_source else {
            return Ok(false);
        };
        self.namespace = receipt
            .checked_namespace(facts, &self.environment)?
            .to_vec();
        Ok(true)
    }
}

pub(super) fn resolve(
    path: &syn::TypePath,
    scope: &Scoped<'_>,
) -> Result<Option<TypeTree>, String> {
    let Some(tree) = exact_tree(path, scope)? else {
        return Ok(None);
    };
    let TypeTree::ScalarProductInstance(instance) = &tree else {
        unreachable!()
    };
    verify_arguments(path, instance.argument)?;
    Ok(Some(tree))
}

fn exact_tree(path: &syn::TypePath, scope: &Scoped<'_>) -> Result<Option<TypeTree>, String> {
    let Some(receipt) = &scope.resolution.scalar_product_source else {
        return Ok(None);
    };
    let mut name = path.clone();
    let last = name
        .path
        .segments
        .last_mut()
        .ok_or("Empty scalar product type path")?;
    last.arguments = syn::PathArguments::None;
    let key = scope.name(&name)?;

    let Some(TypeTree::ScalarProductInstance(instance)) = scope.resolution.trees.get(&key) else {
        return Ok(None);
    };
    if instance.as_ref() != receipt.instance() {
        return Err("Scalar product spelling differs from the checked exact instance".into());
    }
    if scope.namespace.as_slice() != receipt.namespace() {
        return Err("Scalar product source requires its exact checked method namespace".into());
    }
    Ok(Some(TypeTree::ScalarProductInstance(instance.clone())))
}

fn verify_arguments(
    path: &syn::TypePath,
    expected: crate::scalar_product_instances::Argument,
) -> Result<(), String> {
    let Some(last) = path.path.segments.last() else {
        return Err("Missing scalar product type name".into());
    };
    match &last.arguments {
        syn::PathArguments::None => {
            Err("Scalar product source type requires its concrete argument".into())
        }
        syn::PathArguments::AngleBracketed(arguments) if arguments.colon2_token.is_none() => {
            let arguments = arguments.args.iter().collect::<Vec<_>>();
            let scalar = match arguments.as_slice() {
                [syn::GenericArgument::Type(scalar)] => scalar,
                [
                    syn::GenericArgument::Lifetime(_),
                    syn::GenericArgument::Type(scalar),
                ] => scalar,
                _ => {
                    return Err(
                        "Scalar product requires one exact scalar and optional lifetime".into(),
                    );
                }
            };
            if !matches!(scalar, syn::Type::Path(value) if value.qself.is_none() && value.path.is_ident(expected.as_str()))
            {
                return Err(
                    "Scalar product type argument differs from its checked substitution".into(),
                );
            }
            Ok(())
        }
        _ => Err("Unsupported scalar product generic arguments".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scalar_product_instances::Argument;

    #[test]
    fn concrete_width_and_lifetime_arity_are_checked_independently() {
        for source in ["ParsedItem<u8>", "ParsedItem<'_, u8>", "ParsedItem<'a, u8>"] {
            let path = syn::parse_str(source).unwrap();
            assert!(verify_arguments(&path, Argument::U8).is_ok(), "{source}");
        }
        let wide = syn::parse_str("ParsedItem<'_, u16>").unwrap();
        assert!(verify_arguments(&wide, Argument::U16).is_ok());
        assert!(verify_arguments(&wide, Argument::U8).is_err());
        for source in [
            "ParsedItem",
            "ParsedItem<'a>",
            "ParsedItem<u8, u8>",
            "ParsedItem<'a, 'b, u8>",
            "ParsedItem<&u8>",
            "ParsedItem<Byte>",
            "ParsedItem<core::primitive::u8>",
            "ParsedItem<u32>",
            "ParsedItem<{8}>",
            "ParsedItem<Output = u8>",
        ] {
            let path = syn::parse_str(source).unwrap();
            assert!(verify_arguments(&path, Argument::U8).is_err(), "{source}");
        }
    }
}
