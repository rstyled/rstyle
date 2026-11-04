//! Tuple structs a local macro generates, observed through the declaration and field
//! type the macro writes.

use super::{Capture, Failure, unsupported};
use crate::SourceText;
use crate::concrete_types::{MAX_SOURCE_BYTES, TypeTree};
use rstyle_core::cpg::model::{Definition, FileSpan, TypeField};
use serde_json::Value;

impl<'a> Capture<'_, '_, 'a> {
    // rstyle-allow(mutation_per_target): recursive observation shares one budgeted observer
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
        self.generated_origin(generated, (&nominal, &record))?;
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

impl<'a> Capture<'_, '_, 'a> {
    /// Where a generated declaration's template is written, with its hygiene marks
    /// and the original local invocation that defines it.
    fn generated_origin(
        &mut self,
        generated: &Value,
        (nominal, definition): (
            &crate::concrete_types::Nominal,
            &crate::generated_sources::capture::Definition,
        ),
    ) -> Result<(), Failure> {
        use crate::generated_sources::capture::{Mark, Origin};
        let missing = || unsupported("Missing exact raw generated declaration bytes");
        let span: FileSpan =
            serde_json::from_value(generated.get("template").ok_or_else(missing)?.clone())
                .map_err(|_| missing())?;
        let (path, snapshot) = self.raw_file(&span.file)?;
        let marks: Vec<Mark> =
            serde_json::from_value(generated.get("marks").cloned().unwrap_or_default())
                .map_err(|_| unsupported("Generated declaration hygiene is unavailable"))?;
        let root = generated
            .get("root")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                unsupported("Generated declaration has no unique original local bang root")
            })?;
        let origin = crate::declaration_origins::GeneratedOrigin {
            nominal: nominal.clone(),
            definition: definition.clone(),
            origin: Origin {
                raw: crate::nominal_boundary::census::expansions::FileSpan {
                    path,
                    digest: snapshot.checksum().to_owned(),
                    span: crate::frontend::ByteSpan {
                        start: span.start as u32,
                        end: span.end as u32,
                    },
                },
                marks,
            },
            defining_expansion: root.to_owned(),
            file_bytes: snapshot,
        };
        self.generated_origins
            .insert(nominal.definition_path.clone(), origin);

        Ok(())
    }
}

impl Capture<'_, '_, '_> {
    /// A file's absolute path and its bytes, as the compiler read them.
    fn raw_file(&self, file: &str) -> Result<(std::path::PathBuf, crate::SourceSnapshot), Failure> {
        let reader = self.context.reader;
        let path = reader
            .session_file(file)
            .ok_or_else(|| unsupported("Module expansion source path is unavailable"))?;
        let content = reader
            .file(file)
            .ok()
            .and_then(|file| file.content.as_deref())
            .ok_or_else(|| unsupported("Module expansion source buffer is unavailable"))?;
        Ok((path.into(), crate::SourceSnapshot::from(content)))
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
