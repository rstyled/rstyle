//! Where each observed local declaration is written: its file, as the compiler opened
//! it, the declaration's span in it, and a standard `Copy` derive it carries.

use super::{Capture, Failure, unsupported};
use crate::concrete_types::{InterfaceObservations, Nominal};
use crate::declaration_origins::{self as records, BuiltinCopy, Origin, Origins};
use crate::extraction_profile::ExtractionProfile;
use crate::frontend::{ByteSpan, HirFnFacts};
use crate::{SourceSnapshot, SourceText};
use rstyle_core::cpg::model::{Definition, FileSpan};

impl<'a> Capture<'_, '_, 'a> {
    /// A declaration's nominal identity, with where it is written recorded.
    pub(super) fn origin_nominal(
        &mut self,
        definition: &'a Definition,
        source: &SourceText,
    ) -> Result<Nominal, Failure> {
        let nominal = self.nominal(&definition.name, source)?;
        self.origin(definition, &nominal)?;
        Ok(nominal)
    }

    fn origin(&mut self, definition: &Definition, nominal: &Nominal) -> Result<(), Failure> {
        let reader = self.context.reader;
        let span = definition
            .span
            .as_ref()
            .ok_or_else(|| unsupported("Missing original declaration range"))?;
        let source_file = reader
            .session_file(&span.file)
            .ok_or_else(|| unsupported("Missing local declaration file"))?;
        let content = reader
            .file(&span.file)
            .ok()
            .and_then(|file| file.content.as_deref())
            .ok_or_else(|| unsupported("Missing original declaration file buffer"))?;
        let snapshot = SourceSnapshot::from(content);
        let origin = Origin {
            nominal: nominal.clone(),
            source_file: source_file.to_owned(),
            file_digest: snapshot.checksum().to_owned(),
            file_bytes: snapshot,
            declaration_span: ByteSpan {
                start: span.start as u32,
                end: span.end as u32,
            },
            builtin_copy: self.builtin_copy(definition),
        };
        self.origins.insert(nominal.definition_path.clone(), origin);

        Ok(())
    }

    /// The standard `Copy` derive a declaration carries, where the data profile reads it.
    fn builtin_copy(&self, definition: &Definition) -> Option<BuiltinCopy> {
        if self.context.reader.profile != ExtractionProfile::DataV3 {
            return None;
        }
        let copy = definition.extensions.get("rust:builtin-copy")?;
        let record = |key: &str| {
            let definition = self
                .context
                .reader
                .tables
                .definition(copy.get(key)?.as_str()?)
                .ok()?;
            self.context.library_record(definition)
        };
        let span: FileSpan = serde_json::from_value(copy.get("derive_span")?.clone()).ok()?;

        Some(BuiltinCopy {
            trait_definition: record("trait")?,
            derive_definition: record("derive")?,
            derive_span: ByteSpan {
                start: span.start as u32,
                end: span.end as u32,
            },
        })
    }

    /// The declaration origins the observations need, kept to their context.
    pub(super) fn install_origins(
        &mut self,
        observations: &InterfaceObservations,
        facts: &mut HirFnFacts,
    ) -> Result<(), String> {
        let mut origins = Origins {
            identity: records::IDENTITY.into(),
            declarations: std::mem::take(&mut self.origins).into_values().collect(),
            generated: std::mem::take(&mut self.generated_origins)
                .into_values()
                .collect(),
        };
        let context = records::context(facts, observations, &origins)?;
        origins.retain_context(&context)?;
        records::verify(&context, &origins)?;
        facts.build.frontend_options.insert_data(origins);

        Ok(())
    }
}
