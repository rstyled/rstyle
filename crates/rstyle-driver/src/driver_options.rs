//! Explicit driver selection; the remaining arguments belong only to rustc.

use rstyle_capture::{CONTEXT_FLAG, UnitContext};

pub struct Invocation {
    pub arguments: Vec<String>,
    pub unit_context: Option<UnitContext>,
}

pub fn parse(mut arguments: Vec<String>) -> Result<Invocation, String> {
    let prefix = format!("{CONTEXT_FLAG}=");
    let unit_context = arguments
        .first()
        .and_then(|arg| arg.strip_prefix(&prefix))
        .map(UnitContext::parse)
        .transpose()?;
    if unit_context.is_some() {
        arguments.remove(0);
    }
    if arguments.is_empty() {
        return Err("Driver requires a rustc path and its invocation".into());
    }
    if arguments
        .iter()
        .any(|arg| arg == CONTEXT_FLAG || arg.starts_with(&prefix))
    {
        return Err(format!(
            "{CONTEXT_FLAG} must occur once before the rustc path"
        ));
    }

    Ok(Invocation {
        arguments,
        unit_context,
    })
}
