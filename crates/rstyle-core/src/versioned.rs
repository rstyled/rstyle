//! Data files compiled into the engine: TOML with a `schema` naming their format.

use serde::de::DeserializeOwned;

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
