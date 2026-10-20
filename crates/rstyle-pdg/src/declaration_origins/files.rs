//! Original-file checks owned by one declaration inventory/binding operation.
use super::*;

#[derive(Default)]
pub(crate) struct Sources {
    files: BTreeMap<String, CheckedFile>,
}

struct CheckedFile {
    bytes: crate::SourceSnapshot,
    digest: String,
    declarations: Vec<(ByteSpan, Vec<syn::Attribute>)>,
}

impl Sources {
    pub(crate) fn verify(
        &mut self,
        interface: &InterfaceObservations,
        origins: &Origins,
    ) -> Result<(), String> {
        verify_inventory(interface, origins, |origin, declaration| {
            self.check(origin, declaration)
        })
    }

    pub(crate) fn verify_copy(
        &self,
        facts: &crate::HirFnFacts,
        origins: &Origins,
    ) -> Result<(), String> {
        for origin in &origins.declarations {
            let Some(observed) = &origin.builtin_copy else {
                continue;
            };
            let file = self
                .files
                .get(&origin.source_file)
                .ok_or("Copy observation lacks checked declaration source")?;
            let (_, attributes) = file
                .declarations
                .iter()
                .find(|(span, _)| *span == origin.declaration_span)
                .ok_or("Copy observation lacks exact struct ownership")?;
            super::copy::verify(facts, origin, observed, attributes)?;
        }
        Ok(())
    }

    fn check(&mut self, origin: &Origin, declaration: &str) -> Result<(), String> {
        if let Some(file) = self.files.get(&origin.source_file) {
            return file.check(origin, declaration);
        }

        let file = CheckedFile::new(origin)?;
        file.check(origin, declaration)?;
        self.files.insert(origin.source_file.clone(), file);
        Ok(())
    }
}

impl CheckedFile {
    fn new(origin: &Origin) -> Result<Self, String> {
        if origin.source_file.is_empty() || origin.file_digest != origin.file_bytes.identity() {
            return Err("Declaration origin file identity or original byte range disagrees".into());
        }
        let source = origin
            .file_bytes
            .strip_prefix('\u{feff}')
            .unwrap_or(&origin.file_bytes);
        let prefix = origin.file_bytes.len() - source.len();
        let file = syn::parse_file(source).map_err(|error| error.to_string())?;
        let mut visitor = DeclarationSpans {
            source,
            prefix: prefix as u32,
            spans: Vec::new(),
        };
        syn::visit::Visit::visit_file(&mut visitor, &file);

        Ok(Self {
            bytes: origin.file_bytes.clone(),
            digest: origin.file_digest.clone(),
            declarations: visitor.spans,
        })
    }

    fn check(&self, origin: &Origin, declaration: &str) -> Result<(), String> {
        let span = origin.declaration_span;
        if self.bytes != origin.file_bytes
            || self.digest != origin.file_digest
            || self.bytes.get(span.start as usize..span.end as usize) != Some(declaration)
        {
            return Err("Declaration origin file identity or original byte range disagrees".into());
        }
        if self
            .declarations
            .iter()
            .filter(|(candidate, _)| *candidate == span)
            .count()
            != 1
        {
            return Err("Declaration origin does not identify an exact source struct item".into());
        }
        Ok(())
    }
}

struct DeclarationSpans<'a> {
    source: &'a str,
    prefix: u32,
    spans: Vec<(ByteSpan, Vec<syn::Attribute>)>,
}

impl<'ast> syn::visit::Visit<'ast> for DeclarationSpans<'_> {
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        use syn::spanned::Spanned;
        let span = item.span();
        let start = match item.vis {
            syn::Visibility::Inherited => item.struct_token.span,
            _ => item.vis.span(),
        };
        self.record(span, start, &item.attrs);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        use syn::spanned::Spanned;
        let start = match item.vis {
            syn::Visibility::Inherited => item.enum_token.span,
            _ => item.vis.span(),
        };
        self.record(item.span(), start, &item.attrs);
    }
}

impl DeclarationSpans<'_> {
    fn record(
        &mut self,
        span: proc_macro2::Span,
        start: proc_macro2::Span,
        attributes: &[syn::Attribute],
    ) {
        self.spans.push((
            ByteSpan {
                start: crate::source_evidence::byte_offset(self.source, start.start()) as u32
                    + self.prefix,
                end: crate::source_evidence::byte_offset(self.source, span.end()) as u32
                    + self.prefix,
            },
            attributes
                .iter()
                .filter(|attribute| {
                    ["derive", "cfg", "cfg_attr"]
                        .iter()
                        .any(|name| attribute.path().is_ident(name))
                })
                .cloned()
                .collect(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence() -> (InterfaceObservations, Origins) {
        let bytes: crate::SourceSnapshot =
            "\u{feff}// π\r\n/// document A\r\n#[derive(Debug)]\r\npub struct A;\r\npub struct B;\r\n/* pub struct C; */\r\n".into();
        let declarations = ["A", "B"].map(|name| {
            let source = format!("pub struct {name};");
            let start = bytes.find(&source).unwrap() as u32;
            Origin {
                builtin_copy: None,
                nominal: Nominal {
                    source_identity: None,
                    package_name: "sample".into(),
                    package_version: "1".into(),
                    manifest_digest: digest(b"manifest"),
                    definition_path: format!("sample::{name}"),
                    declaration_digest: digest(source.as_bytes()),
                },
                source_file: "/sample/src/lib.rs".into(),
                file_bytes: bytes.clone(),
                file_digest: digest(bytes.as_bytes()),
                declaration_span: ByteSpan {
                    start,
                    end: start + source.len() as u32,
                },
            }
        });
        let observations = declarations
            .iter()
            .map(|origin| Observation::Captured {
                ty: TypeTree::Struct {
                    nominal: origin.nominal.clone(),
                    source: origin.file_bytes[origin.declaration_span.start as usize
                        ..origin.declaration_span.end as usize]
                        .into(),
                    fields: Vec::new(),
                },
            })
            .collect::<Vec<_>>();
        (
            InterfaceObservations {
                identity: crate::concrete_types::IDENTITY.into(),
                parameters: observations[..1].to_vec(),
                result: observations[1].clone(),
            },
            Origins {
                identity: IDENTITY.into(),
                declarations: declarations.into(),
            },
        )
    }

    #[test]
    fn one_inventory_checks_each_exact_file_once_and_rejects_changed_snapshots() {
        let (interface, origins) = evidence();
        let mut sources = Sources::default();
        sources.verify(&interface, &origins).unwrap();
        assert_eq!(sources.files.len(), 1);
        let checked = &sources.files["/sample/src/lib.rs"];
        let spans = checked.declarations.as_ptr();
        assert!(crate::SourceSnapshot::ptr_eq(
            &checked.bytes,
            &origins.declarations[0].file_bytes
        ));
        for _ in 0..20 {
            sources.verify(&interface, &origins).unwrap();
            assert_eq!(
                sources.files["/sample/src/lib.rs"].declarations.as_ptr(),
                spans
            );
        }
        let mut changed = origins.clone();
        for origin in &mut changed.declarations {
            origin.file_bytes = format!("{}// edited\r\n", origin.file_bytes).into();
            origin.file_digest = origin.file_bytes.identity().to_owned();
        }
        assert!(sources.verify(&interface, &changed).is_err());
        verify(&interface, &changed).unwrap();
        let mut contradictory = origins.clone();
        contradictory.declarations[1] = changed.declarations[1].clone();
        assert!(verify(&interface, &contradictory).is_err());
        assert_eq!(
            sources.files["/sample/src/lib.rs"].declarations.as_ptr(),
            spans
        );
    }

    #[test]
    fn claims_ranges_and_commented_structs_cannot_reuse_checked_file_proofs() {
        let (interface, origins) = evidence();
        let mut sources = Sources::default();
        let mut bad = origins.clone();
        bad.declarations[0].declaration_span.start = 1;
        assert!(sources.verify(&interface, &bad).is_err());
        assert!(sources.files.is_empty());
        sources.verify(&interface, &origins).unwrap();
        for change in ["digest", "bytes", "range", "comment"] {
            let mut bad = origins.clone();
            let origin = &mut bad.declarations[0];
            match change {
                "digest" => origin.file_digest.push('x'),
                "bytes" => origin.file_bytes = format!("{} ", origin.file_bytes).into(),
                "range" => origin.declaration_span.end = u32::MAX,
                "comment" => {
                    let start = origin.file_bytes.find("pub struct C;").unwrap() as u32;
                    origin.declaration_span = ByteSpan {
                        start,
                        end: start + "pub struct C;".len() as u32,
                    };
                    assert!(sources.check(origin, "pub struct C;").is_err());
                }
                _ => unreachable!(),
            }
            assert!(sources.verify(&interface, &bad).is_err(), "{change}");
        }
        sources.verify(&interface, &origins).unwrap();
    }
}
