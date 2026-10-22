//! Exact concrete observations, including declaration bytes, map to pure tokens.
use crate::concrete_types::TypeTree;
use std::cell::RefCell;

const MAX_BYTES: usize = 256 * 1024;
const MAX_ENTRIES: usize = 128;

#[derive(Default)]
struct Tokens {
    values: Vec<(TypeTree, String)>,
    bytes: usize,
}

thread_local! {
    static TOKENS: RefCell<Tokens> = RefCell::default();
}

pub(super) fn get(
    tree: &TypeTree,
    convert: impl FnOnce() -> Result<String, String>,
) -> Result<String, String> {
    // Scalar validation is cheaper than searching the composite cache.
    if matches!(tree, TypeTree::Scalar(_)) {
        return convert();
    }
    TOKENS.with(|tokens| {
        if let Some((_, text)) = tokens
            .borrow()
            .values
            .iter()
            .rev()
            .find(|(key, _)| key == tree)
        {
            return Ok(text.clone());
        }
        let text = convert()?;
        tokens.borrow_mut().remember(tree, &text);
        Ok(text)
    })
}

impl Tokens {
    fn remember(&mut self, tree: &TypeTree, text: &str) {
        // Include full concrete source evidence in the retention budget. The
        // exact typed value, rather than its wire size or digest, is the key.
        let Ok(size) = crate::evidence_budget::serialized_len(tree) else {
            return;
        };
        let bytes = size.saturating_add(text.len());
        if bytes > MAX_BYTES {
            return;
        }
        if self.bytes + bytes > MAX_BYTES || self.values.len() == MAX_ENTRIES {
            *self = Self::default();
        }

        // Detach source ranges so a small declaration cannot keep an entire
        // crate source snapshot alive in this process-local cache.
        let Ok(encoded) = serde_json::to_vec(tree) else {
            return;
        };
        let Ok(key) = serde_json::from_slice(&encoded) else {
            return;
        };

        self.bytes += bytes;
        self.values.push((key, text.to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types;

    #[test]
    fn changed_nested_evidence_is_revalidated_and_returned_text_is_independent() {
        let original = TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(TypeTree::Scalar(
            "u8".into(),
        )))));
        let expected = data_types::token(&original).unwrap();
        let mut returned = get(&original, || panic!("already checked")).unwrap();
        returned.clear();
        assert_eq!(data_types::token(&original).unwrap(), expected);
        let changed = TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(TypeTree::Scalar(
            "invalid".into(),
        )))));
        assert!(data_types::token(&changed).is_err());
        assert_eq!(data_types::token(&original).unwrap(), expected);
    }

    fn nominal() -> TypeTree {
        use crate::concrete_types::{Field, Nominal};
        let source: crate::SourceText = "struct Value { byte: u8 }".into();
        TypeTree::Struct {
            nominal: Nominal {
                source_identity: Some("fixture-source".into()),
                package_name: "fixture".into(),
                package_version: "1.0.0".into(),
                manifest_digest: format!("sha256:{}", "0".repeat(64)),
                definition_path: "fixture::Value".into(),
                declaration_digest: source.identity().into(),
            },
            source,
            fields: vec![Field {
                position: 0,
                name: "byte".into(),
                declaration: "u8".into(),
                ty: TypeTree::Scalar("u8".into()),
            }],
        }
    }

    #[test]
    fn matching_nominal_names_do_not_hide_changed_source_or_field_evidence() {
        let original = nominal();
        let expected = data_types::token(&original).unwrap();
        let mut changed_source = original.clone();
        let TypeTree::Struct { source, .. } = &mut changed_source else {
            unreachable!()
        };
        *source = "struct Value { byte: u16 }".into();
        assert!(data_types::token(&changed_source).is_err());
        let mut changed_field = original.clone();
        let TypeTree::Struct { fields, .. } = &mut changed_field else {
            unreachable!()
        };
        fields[0].declaration = "u16".into();
        assert!(data_types::token(&changed_field).is_err());
        assert_eq!(data_types::token(&original).unwrap(), expected);
    }

    #[test]
    fn retained_declarations_do_not_pin_larger_source_snapshots() {
        let mut tree = nominal();
        let TypeTree::Struct { source, .. } = &mut tree else {
            unreachable!()
        };
        let declaration = source.to_string();
        let snapshot: crate::SourceSnapshot =
            format!("{declaration}{}", " ".repeat(MAX_BYTES)).into();
        *source = crate::SourceText::from_snapshot(
            snapshot,
            crate::ByteSpan {
                start: 0,
                end: declaration.len() as u32,
            },
        )
        .unwrap();
        let mut tokens = Tokens::default();
        tokens.remember(&tree, "token");
        let TypeTree::Struct { source, .. } = &tokens.values[0].0 else {
            unreachable!()
        };
        assert_eq!(source.as_str(), declaration);
        assert_eq!(source.snapshot_range().unwrap().0.len(), declaration.len());
    }

    #[test]
    fn retention_budget_includes_concrete_evidence() {
        let mut tokens = Tokens::default();
        for length in 0..MAX_ENTRIES + 10 {
            let tree = TypeTree::Array {
                element: Box::new(TypeTree::Scalar("u8".into())),
                length: length as u64,
            };
            tokens.remember(&tree, "token");
            assert!(tokens.values.len() <= MAX_ENTRIES);
            assert!(tokens.bytes <= MAX_BYTES);
        }
        let count = tokens.values.len();
        tokens.remember(&TypeTree::Scalar("x".repeat(MAX_BYTES)), "token");
        assert_eq!(tokens.values.len(), count);
    }
}
