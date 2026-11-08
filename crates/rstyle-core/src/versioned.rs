//! Data files compiled into the engine: TOML with a `schema` naming their format.

use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Parses `text` and checks that its schema, as `schema` reads it, is `expected`.
pub fn parse<T: DeserializeOwned>(
    text: &str,
    (expected, label): (&str, &str),
    schema: impl FnOnce(&T) -> &str,
) -> Result<T, String> {
    let file: T = toml::from_str(text).map_err(|error| error.to_string())?;
    let found = schema(&file);
    if found != expected {
        return Err(format!("Unsupported {label} schema {found}"));
    }
    Ok(file)
}

/// Reads a data file into its namespace and what it holds.
type ParseFile<T> = fn(&str) -> Result<(String, T), String>;

/// Data read once from files compiled into the engine, one per frontend namespace.
pub struct ByNamespace<T: 'static> {
    files: OnceLock<BTreeMap<String, T>>,
    none: OnceLock<T>,
}

impl<T: Default> ByNamespace<T> {
    pub const fn empty() -> Self {
        Self {
            files: OnceLock::new(),
            none: OnceLock::new(),
        }
    }

    /// A namespace's data, or `T::default()` for a namespace without a file.
    pub fn of(
        &'static self,
        namespace: &str,
        (texts, parse): (&[&str], ParseFile<T>),
    ) -> &'static T {
        let files = self.files.get_or_init(|| {
            texts
                .iter()
                .map(|text| parse(text).expect("embedded data files are valid"))
                .collect()
        });
        files
            .get(namespace)
            .unwrap_or_else(|| self.none.get_or_init(T::default))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(text: &str) -> Result<(String, String), String> {
        Ok((text.to_string(), format!("data of {text}")))
    }

    #[test]
    fn a_namespace_gets_its_file_and_any_other_the_default() {
        let data: &'static ByNamespace<String> = Box::leak(Box::new(ByNamespace::empty()));
        assert_eq!(data.of("go", (&["go"], named)), "data of go");
        assert_eq!(data.of("rust", (&["go"], named)), "");
    }
}
