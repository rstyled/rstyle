//! Explicit dependencies and response files cannot substitute for pinned sysroot roots.
pub(crate) fn verify(
    facts: &dyn crate::unit_store::EvidenceFacts,
    roots: &[&str],
    label: &str,
) -> Result<(), String> {
    let invocation = facts
        .build
        .frontend_options
        .get("invocation")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("Missing {label} compiler invocation"))?;
    let mut extern_value = false;

    for argument in invocation {
        let argument = argument
            .as_str()
            .ok_or_else(|| format!("Invalid {label} compiler argument"))?;
        if argument.starts_with('@') {
            return Err(format!("{label} roots require expanded compiler arguments"));
        }
        let value = if extern_value {
            Some(argument)
        } else {
            argument.strip_prefix("--extern=")
        };
        if let Some(value) = value {
            let name = value
                .split('=')
                .next()
                .unwrap_or(value)
                .rsplit(':')
                .next()
                .unwrap_or(value);
            if roots.contains(&name) {
                return Err(format!(
                    "{label} library root is overridden by an extern dependency"
                ));
            }
        }
        extern_value = argument == "--extern";
    }

    if extern_value {
        return Err(format!("Incomplete {label} extern argument"));
    }
    Ok(())
}
