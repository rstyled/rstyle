//! Pure canonical syntax reuse. This grants no source or admission authority.
use super::TypeToken;
use std::{cell::RefCell, collections::BTreeMap};

const MAX_BYTES: usize = 256 * 1024;
const MAX_ENTRIES: usize = 128;

#[derive(Default)]
struct Tokens {
    values: BTreeMap<String, TypeToken>,
    bytes: usize,
}

thread_local! {
    static TOKENS: RefCell<Tokens> = RefCell::default();
}

pub(super) fn get(
    text: &str,
    parse: impl FnOnce() -> Result<TypeToken, String>,
) -> Result<TypeToken, String> {
    TOKENS.with(|tokens| {
        if let Some(token) = tokens.borrow().values.get(text) {
            return Ok(token.clone());
        }
        let token = parse()?;
        tokens.borrow_mut().remember(text, &token);
        Ok(token)
    })
}

impl Tokens {
    fn remember(&mut self, text: &str, token: &TypeToken) {
        // Bound both total canonical input bytes and entry count. Oversized
        // inputs still receive the normal parser's validation, without retention.
        if text.len() > MAX_BYTES {
            return;
        }
        if self.bytes + text.len() > MAX_BYTES || self.values.len() == MAX_ENTRIES {
            *self = Self::default();
        }
        self.bytes += text.len();
        self.values.insert(text.to_owned(), token.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{data_types, extraction_profile::ExtractionProfile};

    #[test]
    fn reuse_requires_exact_canonical_text_and_keeps_profile_checks() {
        let text = r#"{"kind":"option","value":{"kind":"scalar","value":"u8"}}"#;
        let expected = TypeToken::Option(Box::new(TypeToken::Scalar("u8".into())));
        assert_eq!(data_types::parse(text).unwrap(), expected);
        assert_eq!(get(text, || panic!("already parsed")).unwrap(), expected);
        for changed in [
            format!(" {text}"),
            text.replace("u8", "bogus"),
            text.replace("value", "extra"),
        ] {
            assert!(data_types::parse(&changed).is_err());
        }
        assert!(data_types::parse_for(text, ExtractionProfile::DataV3).is_ok());
        assert!(data_types::parse_for(text, ExtractionProfile::R0).is_err());
        assert_eq!(data_types::parse(text).unwrap(), expected);
    }

    #[test]
    fn retention_is_bounded_and_failure_does_not_publish_a_token() {
        let token = TypeToken::Scalar("u8".into());
        let mut tokens = Tokens::default();
        for index in 0..MAX_ENTRIES + 10 {
            tokens.remember(&index.to_string(), &token);
            assert!(tokens.values.len() <= MAX_ENTRIES);
            assert!(tokens.bytes <= MAX_BYTES);
        }
        tokens.remember(&"x".repeat(MAX_BYTES + 1), &token);
        assert!(tokens.bytes <= MAX_BYTES);
        tokens.remember(&"x".repeat(MAX_BYTES), &token);
        assert_eq!(tokens.values.len(), 1);
        assert_eq!(tokens.bytes, MAX_BYTES);
        assert!(get("invalid", || Err("invalid syntax".into())).is_err());
        assert!(get("invalid", || Err("checked again".into())).is_err());
    }
}
