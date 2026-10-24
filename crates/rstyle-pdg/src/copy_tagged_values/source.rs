use super::*;

impl Context<'_> {
    pub(super) fn typed(&self, node: &HirNode, tree: &TypeTree) -> Result<(), String> {
        let expected = crate::data_types::TypeToken::from_tree(tree).text()?;
        if node.expansion.is_some() || crate::data_aggregates::token(node)? != expected {
            return Err("Copy conversion HIR type or source provenance disagrees".into());
        }
        Ok(())
    }

    pub(super) fn correspondence(
        &self,
        span: proc_macro2::Span,
        child: &HirNode,
    ) -> Result<(), String> {
        crate::source_evidence::verify_child_span(self.node, child, span, self.facts)
    }

    pub(super) fn formal(&self, node: &HirNode) -> Result<(), String> {
        let formal = self
            .facts
            .signature
            .params
            .first()
            .ok_or("Missing conversion formal")?;
        if node.kind != "expr_path"
            || !node.children.is_empty()
            || !node.res.as_ref().is_some_and(|res| {
                res.kind == "local" && res.target.as_ref() == Some(&formal.pat.hir)
            })
        {
            return Err("Copy conversion must evaluate its original formal exactly once".into());
        }
        Ok(())
    }

    pub(super) fn variant(&self, node: &HirNode, kind: &str) -> Result<String, String> {
        let TypeTree::MixedEnum {
            nominal, variants, ..
        } = self.endpoints.tagged
        else {
            return Err("Missing checked mixed tagged declaration".into());
        };
        let tag = &variants
            .get(1)
            .ok_or("Missing checked payload variant")?
            .name;
        let resolution = node
            .res
            .as_ref()
            .ok_or("Missing conversion variant identity")?;
        let identity: crate::data_library::Callee =
            serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
        let selected = self
            .facts
            .build
            .frontend_options
            .get("rstyle.nominal_selected_unit")
            .and_then(|unit| unit.get("crate_name"))
            .and_then(serde_json::Value::as_str);
        if resolution.kind != kind
            || resolution.target.is_some()
            || identity.path != format!("{}::{tag}", nominal.definition_path)
            || Some(identity.crate_name.as_str()) != selected
            || identity.crate_hash != "local-unencoded"
            || identity.definition_hash.is_empty()
            || identity.implementation.is_some()
        {
            return Err("Copy conversion variant loses its exact compiler identity".into());
        }

        Ok(tag.clone())
    }
}

pub(super) fn dispatch(node: &HirNode) -> Result<(), String> {
    crate::data_enums::verify_constructor_dispatch(
        node,
        crate::data_aggregates::token(&node.children[0])?,
    )
}
