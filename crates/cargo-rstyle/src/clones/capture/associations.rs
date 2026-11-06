//! Run-local compiled-artifact associations, separate from semantic call authority.
use super::Unit;
use rstyle_core::cli_args::flag_value;
use std::path::{Path, PathBuf};

/// Borrow captured extern spellings without reading compiler artifacts.
pub(in crate::clones) fn extern_arguments(
    invocation: &[String],
) -> impl Iterator<Item = (&str, &str)> {
    invocation
        .iter()
        .enumerate()
        .filter_map(|(index, argument)| {
            argument
                .strip_prefix("--extern=")
                .or_else(|| {
                    (argument == "--extern")
                        .then(|| invocation.get(index + 1).map(String::as_str))
                        .flatten()
                })?
                .split_once('=')
        })
}

pub(in crate::clones) fn extern_path(
    invocation: &[String],
    alias: &str,
) -> Result<PathBuf, String> {
    let mut matches = Vec::new();
    for (name, path) in extern_arguments(invocation) {
        if name == alias {
            if path.is_empty() {
                return Err("Empty selected --extern path".into());
            }
            matches.push(PathBuf::from(path));
        }
    }
    match matches.as_slice() {
        [path] => Ok(path.clone()),
        _ => Err("Missing or ambiguous caller --extern alias".into()),
    }
}

/// Expected metadata output from the captured compiler invocation; performs no I/O.
pub(in crate::clones) fn emission_path(unit: &Unit) -> Result<PathBuf, String> {
    let output = flag_value(&unit.invocation, "--out-dir")
        .ok_or("Missing callee compiler output directory")?;
    let name =
        flag_value(&unit.invocation, "--crate-name").ok_or("Missing callee compiler crate name")?;
    let suffix = extra_filename(&unit.invocation)?;

    if name != unit.target.name {
        return Err("Emitted rmeta disagrees with exact callee ledger output".into());
    }
    Ok(Path::new(output).join(format!("lib{name}{suffix}.rmeta")))
}

fn extra_filename(invocation: &[String]) -> Result<&str, String> {
    let matches: Vec<_> = invocation
        .iter()
        .enumerate()
        .filter_map(|(index, argument)| {
            argument.strip_prefix("-Cextra-filename=").or_else(|| {
                (argument == "-C")
                    .then(|| invocation.get(index + 1)?.strip_prefix("extra-filename="))
                    .flatten()
            })
        })
        .collect();
    match matches.as_slice() {
        [suffix] => Ok(suffix),
        _ => Err("Missing or ambiguous callee extra-filename".into()),
    }
}
