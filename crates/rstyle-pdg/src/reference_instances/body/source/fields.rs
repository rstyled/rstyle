//! Complete ordered constructor fields preserve the once-converted saved value.
use super::{Flow, Result, Role, nodes};
use crate::HirNode;

impl<'a> Flow<'a> {
    pub(super) fn product(
        &mut self,
        syntax: &syn::ExprStruct,
        node: &'a HirNode,
    ) -> Result<&'a HirNode> {
        self.structure(syntax, node, Role::Input)?;
        self.path(&syntax.fields[0].expr, &node.children[0], Role::Bytes)?;
        self.product_fields(syntax, node)
    }

    fn product_fields(
        &mut self,
        syntax: &syn::ExprStruct,
        node: &'a HirNode,
    ) -> Result<&'a HirNode> {
        let syn::Expr::Struct(span) = &syntax.fields[1].expr else {
            return Err("Constructor result span is not its complete original Span value".into());
        };

        let length = self.span(span, &node.children[1])?;
        self.variant(&syntax.fields[2].expr, &node.children[2])?;
        self.literal(&syntax.fields[3].expr, &node.children[3], false)?;
        Ok(length)
    }

    fn span(&mut self, syntax: &syn::ExprStruct, node: &'a HirNode) -> Result<&'a HirNode> {
        self.structure(syntax, node, Role::Span)?;
        self.literal(&syntax.fields[0].expr, &node.children[0], true)?;
        self.length(&syntax.fields[1].expr, &node.children[1])
    }

    fn length(&mut self, syntax: &syn::Expr, call: &'a HirNode) -> Result<&'a HirNode> {
        let syn::Expr::MethodCall(length) = syntax else {
            return Err("Constructor span endpoint is not the saved view length".into());
        };

        self.node(call, length, nodes::Shape::method(Role::Usize))?;
        if length.method != "len"
            || !length.args.is_empty()
            || length.turbofish.is_some()
            || !length.attrs.is_empty()
        {
            return Err("Constructor endpoint changes its original length invocation".into());
        }
        self.path(&length.receiver, &call.children[0], Role::Bytes)?;
        Ok(call)
    }

    fn structure(&mut self, syntax: &syn::ExprStruct, node: &'a HirNode, role: Role) -> Result<()> {
        let (name, fields): (_, &[&str]) = match role {
            Role::Input => ("Input", &["haystack", "span", "anchored", "earliest"]),
            Role::Span => ("Span", &["start", "end"]),
            _ => return Err("Unknown constructor source product role".into()),
        };
        structure(syntax, name, fields)?;

        self.node(
            node,
            syntax,
            nodes::Shape::value("expr_struct", fields.len(), role),
        )?;
        self.nominal_paths.push((
            role,
            syn::TypePath {
                qself: None,
                path: syntax.path.clone(),
            },
        ));
        Ok(())
    }

    fn variant(&mut self, syntax: &syn::Expr, node: &'a HirNode) -> Result<()> {
        let syn::Expr::Path(path) = syntax else {
            return Err("Constructor anchored field is not its original unit variant".into());
        };
        self.node(node, path, nodes::Shape::variant())?;
        if !path.attrs.is_empty()
            || path.qself.is_some()
            || crate::source_evidence::plain_relative_path(&path.path).as_deref()
                != Some(&["Anchored".into(), "No".into()])
        {
            return Err("Constructor anchored field must preserve Anchored::No".into());
        }

        let mut anchored_path = path.path.clone();
        anchored_path.segments.pop();
        anchored_path.segments.pop_punct();
        self.nominal_paths.push((
            Role::Anchored,
            syn::TypePath {
                qself: None,
                path: anchored_path,
            },
        ));
        let resolution = node
            .res
            .as_ref()
            .ok_or("Missing original unit variant resolution")?;
        let definition: crate::data_library::Callee =
            serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
        let compiled = self
            .facts
            .build
            .frontend_options
            .get("rstyle.rust.compiled-crate")
            .ok_or("Missing canonical constructor crate")?;
        if definition.path != "util::search::Anchored::No"
            || definition.implementation.is_some()
            || definition.crate_hash != "local-unencoded"
            || definition.definition_hash.is_empty()
            || compiled
                .get("crate_name")
                .and_then(serde_json::Value::as_str)
                != Some(definition.crate_name.as_str())
        {
            return Err(
                "Constructor unit variant differs from its original source-backed nominal owner"
                    .into(),
            );
        }
        Ok(())
    }

    fn literal(&mut self, syntax: &syn::Expr, node: &'a HirNode, zero: bool) -> Result<()> {
        let syn::Expr::Lit(literal) = syntax else {
            return Err("Constructor start/earliest field is not an original literal".into());
        };
        let expected = if zero {
            nodes::Shape::literal(Role::Usize, "0")
        } else {
            nodes::Shape::literal(Role::Bool, "false")
        };

        self.node(node, literal, expected)?;
        let matches = if zero {
            matches!(&literal.lit, syn::Lit::Int(value) if value.to_string() == "0")
        } else {
            matches!(&literal.lit, syn::Lit::Bool(value) if !value.value)
        };
        if !literal.attrs.is_empty() || !matches {
            return Err("Constructor fields must preserve start zero and earliest false".into());
        }
        Ok(())
    }
}

fn structure(syntax: &syn::ExprStruct, name: &str, fields: &[&str]) -> Result<()> {
    if !syntax.attrs.is_empty()
        || syntax.qself.is_some()
        || !syntax.path.is_ident(name)
        || syntax.rest.is_some()
        || syntax.dot2_token.is_some()
        || syntax.fields.len() != fields.len()
        || !syntax.fields.iter().zip(fields).all(|(field, name)| {
            field.attrs.is_empty()
                && matches!(&field.member, syn::Member::Named(ident) if ident == name)
        })
    {
        return Err(
            "Constructor must preserve every original named field in evaluation order".into(),
        );
    }
    Ok(())
}
