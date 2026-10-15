//! Storage sharing requires exact bytes; it grants no declaration authority.
use super::*;
use crate::{
    ByteSpan, SourceText,
    concrete_types::{Field, Nominal, Observation, TypeTree},
    declaration_origins::Origins,
    source_table::Ranges,
};

impl FrontendOptions {
    pub(crate) fn share_declaration_ranges(&mut self, ranges: &mut Ranges) -> Result<(), String> {
        let Some((key, entry)) = self.0.remove_entry(crate::concrete_types::KEY) else {
            return Ok(());
        };
        let entry = share_entry(entry, self.0.get(crate::declaration_origins::KEY), ranges)?;
        self.0.insert(key, entry);
        Ok(())
    }
}

fn share_entry(
    entry: Entry,
    origins: Option<&Entry>,
    ranges: &mut Ranges,
) -> Result<Entry, String> {
    match (entry, origins) {
        (Entry::Interface(record), Some(Entry::Origins(origins))) => {
            let value = Bindings {
                origins: &origins.value,
                ranges,
                entries: 0,
            }
            .interface(record.value)?;
            Ok(Entry::Interface(Record {
                value,
                nulls: record.nulls,
                json: record.json,
            }))
        }
        (entry, _) => Ok(entry),
    }
}

struct Bindings<'a> {
    origins: &'a Origins,
    ranges: &'a mut Ranges,
    entries: usize,
}

struct Declaration {
    nominal: Nominal,
    source: SourceText,
    fields: Vec<Field>,
}

impl Bindings<'_> {
    fn interface(
        &mut self,
        interface: InterfaceObservations,
    ) -> Result<InterfaceObservations, String> {
        let parameters = interface
            .parameters
            .into_iter()
            .map(|value| self.observation(value))
            .collect::<Result<_, _>>()?;
        let result = self.observation(interface.result)?;
        Ok(InterfaceObservations {
            identity: interface.identity,
            parameters,
            result,
        })
    }

    fn observation(&mut self, observation: Observation) -> Result<Observation, String> {
        match observation {
            Observation::Captured { ty } => Ok(Observation::Captured {
                ty: self.tree(ty, 0)?,
            }),
            other => Ok(other),
        }
    }

    fn tree(&mut self, tree: TypeTree, depth: usize) -> Result<TypeTree, String> {
        self.entries += 1;
        if self.entries > crate::concrete_types::MAX_ENTRIES
            || depth > crate::concrete_types::MAX_DEPTH
        {
            return Err("Concrete type observation exceeds depth/entry budget".into());
        }
        self.dispatch(tree, depth)
    }

    fn dispatch(&mut self, tree: TypeTree, depth: usize) -> Result<TypeTree, String> {
        match tree {
            TypeTree::Struct {
                nominal,
                source,
                fields,
            } => self.structure(
                Declaration {
                    nominal,
                    source,
                    fields,
                },
                depth,
            ),
            TypeTree::Tuple(elements) => self.tuple(elements, depth),
            other => self.element(other, depth),
        }
    }

    fn tuple(&mut self, elements: Vec<TypeTree>, depth: usize) -> Result<TypeTree, String> {
        elements
            .into_iter()
            .map(|value| self.tree(value, depth + 1))
            .collect::<Result<_, _>>()
            .map(TypeTree::Tuple)
    }

    fn element(&mut self, tree: TypeTree, depth: usize) -> Result<TypeTree, String> {
        match tree {
            TypeTree::Array { element, length } => Ok(TypeTree::Array {
                element: self.child(element, depth)?,
                length,
            }),
            TypeTree::Shared(element) => Ok(TypeTree::Shared(self.child(element, depth)?)),
            TypeTree::Slice(element) => Ok(TypeTree::Slice(self.child(element, depth)?)),
            other => Ok(other),
        }
    }

    fn child(&mut self, mut element: Box<TypeTree>, depth: usize) -> Result<Box<TypeTree>, String> {
        let original = std::mem::replace(element.as_mut(), TypeTree::Tuple(Vec::new()));
        *element = self.tree(original, depth + 1)?;
        Ok(element)
    }

    fn structure(&mut self, declaration: Declaration, depth: usize) -> Result<TypeTree, String> {
        let source = self.declaration_source(&declaration.nominal, declaration.source)?;
        let fields = declaration
            .fields
            .into_iter()
            .map(|field| self.field(field, &source, depth))
            .collect::<Result<_, _>>()?;
        Ok(TypeTree::Struct {
            nominal: declaration.nominal,
            source,
            fields,
        })
    }

    fn declaration_source(
        &mut self,
        nominal: &Nominal,
        source: SourceText,
    ) -> Result<SourceText, String> {
        let Some(origin) = self
            .origins
            .declarations
            .iter()
            .find(|origin| &origin.nominal == nominal)
        else {
            return Ok(source);
        };
        let span = origin.declaration_span;
        if origin
            .file_bytes
            .get(span.start as usize..span.end as usize)
            != Some(source.as_str())
        {
            return Ok(source);
        }

        self.ranges.bind(&source, &origin.file_bytes, span)
    }

    fn field(&mut self, field: Field, source: &SourceText, depth: usize) -> Result<Field, String> {
        Ok(Field {
            declaration: self.field_source(field.declaration, source)?,
            ty: self.tree(field.ty, depth + 1)?,
            position: field.position,
            name: field.name,
        })
    }

    fn field_source(
        &mut self,
        field: SourceText,
        source: &SourceText,
    ) -> Result<SourceText, String> {
        let Some(offset) = source.find(field.as_str()) else {
            return Ok(field);
        };
        let (snapshot, span) = source.snapshot_range()?;
        let start = span
            .start
            .checked_add(u32::try_from(offset).map_err(|_| "Field source offset overflow")?)
            .ok_or("Field source offset overflow")?;
        let end = start
            .checked_add(u32::try_from(field.len()).map_err(|_| "Field source length overflow")?)
            .ok_or("Field source length overflow")?;

        self.ranges.bind(&field, snapshot, ByteSpan { start, end })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn options() -> FrontendOptions {
        let source = "struct Data { value: i32 }";
        let file = format!("\u{feff}// π\r\n{source}\r\n");
        let start = file.find(source).unwrap() as u32;
        let nominal = json!({"package_name":"sample","package_version":"1",
            "manifest_digest":crate::concrete_types::digest(b"manifest"),
            "definition_path":"sample::Data",
            "declaration_digest":crate::concrete_types::digest(source.as_bytes())});
        let tree = json!({"kind":"Struct","evidence":{"nominal":nominal,
            "source":source,"fields":[{"position":0,"name":"value",
                "declaration":"i32","ty":{"kind":"Scalar","evidence":"i32"}}]}});
        serde_json::from_value(json!({
            crate::concrete_types::KEY:{"identity":crate::concrete_types::IDENTITY,
                "parameters":[{"status":"Captured","ty":tree},{"status":"Captured","ty":tree}],
                "result":{"status":"Captured","ty":tree}},
            crate::declaration_origins::KEY:{"identity":crate::declaration_origins::IDENTITY,
                "declarations":[{"nominal":nominal,"source_file":"/sample/src/lib.rs",
                    "file_bytes":file,"file_digest":crate::concrete_types::digest(file.as_bytes()),
                    "declaration_span":{"start":start,"end":start+source.len() as u32}}]}
        }))
        .unwrap()
    }

    fn structure(observation: &Observation) -> (&SourceText, &[Field]) {
        let Observation::Captured {
            ty: TypeTree::Struct { source, fields, .. },
        } = observation
        else {
            panic!("expected struct");
        };
        (source, fields)
    }

    #[test]
    fn declaration_and_field_ranges_share_original_owner_and_actual_identity() {
        let mut options = options();
        let before = serde_json::to_vec(&options).unwrap();
        let mut ranges = Ranges::default();
        options.share_declaration_ranges(&mut ranges).unwrap();
        assert_eq!(serde_json::to_vec(&options).unwrap(), before);
        let interface = options.data::<InterfaceObservations>().unwrap();
        interface.validate().unwrap();
        let origins = options.data::<Origins>().unwrap();
        crate::declaration_origins::verify(&interface, &origins).unwrap();
        let (first, fields) = structure(&interface.parameters[0]);
        let snapshot = first.snapshot_range().unwrap().0;
        assert!(crate::SourceSnapshot::ptr_eq(
            snapshot,
            &origins.declarations[0].file_bytes
        ));
        assert!(crate::SourceSnapshot::ptr_eq(
            snapshot,
            fields[0].declaration.snapshot_range().unwrap().0
        ));
        for observation in interface.parameters.iter().chain([&interface.result]) {
            let (source, other_fields) = structure(observation);
            assert!(std::ptr::eq(first.identity(), source.identity()));
            assert!(std::ptr::eq(
                fields[0].declaration.identity(),
                other_fields[0].declaration.identity()
            ));
        }
    }

    #[test]
    fn malformed_declarations_remain_untrusted_and_byte_exact() {
        for property in ["source", "field", "digest", "range"] {
            let mut options = options();
            let mut wire = serde_json::to_value(&options).unwrap();
            let interface = &mut wire[crate::concrete_types::KEY];
            match property {
                "source" => {
                    interface["parameters"][0]["ty"]["evidence"]["source"] = json!("struct Other;")
                }
                "field" => {
                    interface["parameters"][0]["ty"]["evidence"]["fields"][0]["declaration"] =
                        json!("wrong")
                }
                "digest" => {
                    interface["parameters"][0]["ty"]["evidence"]["nominal"]["declaration_digest"] =
                        json!("sha256:wrong")
                }
                "range" => {
                    wire[crate::declaration_origins::KEY]["declarations"][0]["declaration_span"]["end"] =
                        json!(u32::MAX)
                }
                _ => unreachable!(),
            }
            options = serde_json::from_value(wire.clone()).unwrap();
            options
                .share_declaration_ranges(&mut Ranges::default())
                .unwrap();
            assert_eq!(serde_json::to_value(&options).unwrap(), wire);
            assert!(
                crate::declaration_origins::verify(
                    &options.data::<InterfaceObservations>().unwrap(),
                    &options.data::<Origins>().unwrap()
                )
                .is_err()
            );
        }
    }
}
