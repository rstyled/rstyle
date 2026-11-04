//! Tuple structs a local macro generates, observed through the declaration and field
//! type the macro writes.

use super::{Capture, Failure, unsupported};
use crate::SourceText;
use crate::concrete_types::{MAX_SOURCE_BYTES, TypeTree};
use rstyle_core::cpg::model::{Definition, FileSpan, TypeField};
use serde_json::Value;

impl<'a> Capture<'_, '_, 'a> {
    /// A unary tuple struct a local macro generates, with its declaration and field type
    /// as the macro writes them.
    pub(super) fn generated_structure(
        &mut self,
        (definition, fields): (&'a Definition, &'a [TypeField]),
        generated: &Value,
        depth: usize,
    ) -> Result<TypeTree, Failure> {
        let (template, field) = self.generated_raw(generated)?;
        let record = generated_definition(definition)?;
        self.visiting.push(definition.id.as_str());
        let captured = self.fields(fields, vec![("0".into(), field.as_str().to_owned())], depth);
        self.visiting.pop();
        let nominal = self.nominal(&definition.name, &template)?;
        let mut captured = captured?;
        let [only] = captured.as_mut_slice() else {
            return Err(unsupported("Generated tuple field inventory disagrees"));
        };
        only.declaration = field;
        Ok(TypeTree::GeneratedStruct {
            nominal,
            template,
            definition: Box::new(record),
            fields: captured,
        })
    }

    /// The declaration and its one field type, after the frontend's admission checks.
    fn generated_raw(&self, generated: &Value) -> Result<(SourceText, SourceText), Failure> {
        if !self.supports(crate::result_maps::CAPABILITY) {
            return Err(unsupported(
                "Generated declaration requires its registered Result-map capability",
            ));
        }
        let field_types = generated
            .get("field_types")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        if generated.get("shape").and_then(Value::as_str) != Some("tuple") {
            return Err(unsupported(
                "Generated declaration requires a unary tuple struct",
            ));
        }
        if generated.get("generic").is_some() || field_types.len() != 1 {
            return Err(unsupported(
                "Generated tuple declaration has generics or another field inventory",
            ));
        }
        let template = self.raw(generated.get("template"))?;
        let field = self.raw(field_types.first())?;
        crate::declaration_origins::generated_field_type(&field).map_err(Failure::Unsupported)?;
        if generated.get("root").is_none() {
            return Err(unsupported(
                "Generated declaration has no unique original local bang root",
            ));
        }
        Ok((template, field))
    }

    /// Raw macro-written bytes at a recorded span, within the source budget.
    fn raw(&self, span: Option<&Value>) -> Result<SourceText, Failure> {
        let missing = || unsupported("Missing exact raw generated declaration bytes");
        let span: FileSpan =
            serde_json::from_value(span.ok_or_else(missing)?.clone()).map_err(|_| missing())?;
        let source = self.context.reader.source(&span).map_err(|_| missing())?;
        if source.len() > MAX_SOURCE_BYTES {
            return Err(Failure::Budget(
                "Generated declaration exceeds source budget".into(),
            ));
        }
        Ok(source)
    }
}

fn generated_definition(
    definition: &Definition,
) -> Result<crate::generated_sources::capture::Definition, Failure> {
    let identity = |key: &str| {
        definition
            .extensions
            .get("rust:compiler-identity")
            .and_then(|identity| identity.get(key)?.as_str())
            .map(str::to_owned)
            .ok_or_else(|| unsupported("Missing generated definition identity"))
    };
    Ok(crate::generated_sources::capture::Definition {
        path: definition.name.clone(),
        definition_identity: identity("def_path_hash")?,
        crate_name: identity("crate_name")?,
        stable_crate_id: identity("stable_crate_id")?,
        local: true,
    })
}
