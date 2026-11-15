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

/// A data file that names its schema and the frontend namespace it's for.
pub trait Namespaced {
    fn schema(&self) -> &str;
    fn namespace(&self) -> &str;
}

/// Parses a namespace's file, as `ByNamespace` keys it: its namespace, and the file.
pub fn namespaced<T: DeserializeOwned + Namespaced>(
    text: &str,
    schema: (&str, &str),
) -> Result<(String, T), String> {
    let file: T = parse(text, schema, T::schema)?;
    Ok((file.namespace().to_owned(), file))
}

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

    /// A namespace's data, or `T::default()` for a namespace without a file. `parse` reads a
    /// file into its namespace and what it holds.
    pub fn of(
        &'static self,
        namespace: &str,
        (texts, parse): (&[&str], impl Fn(&str) -> Result<(String, T), String>),
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

impl<T: Default + DeserializeOwned + Namespaced> ByNamespace<T> {
    /// A namespace's data among files that each name their schema and their namespace, read
    /// as `schema`: the schema they must name, and what to call a file that names another.
    pub fn named(
        &'static self,
        namespace: &str,
        (texts, schema): (&[&str], (&str, &str)),
    ) -> &'static T {
        self.of(namespace, (texts, |text| namespaced(text, schema)))
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
