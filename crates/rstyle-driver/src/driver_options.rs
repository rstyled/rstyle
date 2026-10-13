//! Explicit driver selection; the remaining arguments belong only to rustc.

use rstyle_pdg::extraction_profile::{DRIVER_SELECTION_FLAG, ExtractionProfile};

pub struct Invocation {
    pub arguments: Vec<String>,
    pub profile: Option<ExtractionProfile>,
}

pub fn parse(mut arguments: Vec<String>) -> Result<Invocation, String> {
    let prefix = format!("{DRIVER_SELECTION_FLAG}=");
    let profile = arguments
        .first()
        .and_then(|arg| arg.strip_prefix(&prefix))
        .map(ExtractionProfile::parse)
        .transpose()?;
    if profile.is_some() {
        arguments.remove(0);
    }
    if arguments.is_empty() {
        return Err("Driver requires a rustc path and its invocation".into());
    }
    if arguments
        .iter()
        .any(|arg| arg == DRIVER_SELECTION_FLAG || arg.starts_with(&prefix))
    {
        return Err(format!(
            "{DRIVER_SELECTION_FLAG}=<profile> must occur once before the rustc path"
        ));
    }

    Ok(Invocation { arguments, profile })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    #[test]
    fn selection_is_typed_and_removed_from_rustc_arguments() {
        let original = args(&["rustc", "input.rs", "--crate-type", "lib"]);
        let mut selected = vec![format!("{DRIVER_SELECTION_FLAG}=hir-helpers-v1")];
        selected.extend(original.clone());
        let invocation = parse(selected).unwrap();
        assert_eq!(invocation.profile, Some(ExtractionProfile::HelpersV1));
        assert_eq!(invocation.arguments, original);
        assert_eq!(parse(original).unwrap().profile, None);
    }

    #[test]
    fn malformed_duplicate_or_misplaced_selection_fails_before_compilation() {
        for invalid in [
            vec![],
            args(&["--rstyle-extraction-profile=unknown", "rustc"]),
            args(&["--rstyle-extraction-profile=hir-helpers-v1"]),
            args(&[
                "--rstyle-extraction-profile=hir-helpers-v1",
                "--rstyle-extraction-profile=r0",
                "rustc",
            ]),
            args(&["rustc", "--rstyle-extraction-profile=hir-helpers-v1"]),
            args(&["--rstyle-extraction-profile", "hir-helpers-v1", "rustc"]),
        ] {
            assert!(parse(invalid).is_err());
        }
    }
}
