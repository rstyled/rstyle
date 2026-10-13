//! Explicit driver selection; the remaining arguments belong only to rustc.

use rstyle_pdg::extraction_profile::{DRIVER_SELECTION_FLAG, ExtractionProfile};

pub struct Invocation {
    pub arguments: Vec<String>,
    pub profile: Option<ExtractionProfile>,
    pub unit_context: Option<rstyle_pdg::nominal_boundary::UnitContext>,
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
    let unit_context = selected_unit(&mut arguments, profile)?;
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

    Ok(Invocation {
        arguments,
        profile,
        unit_context,
    })
}

fn selected_unit(
    arguments: &mut Vec<String>,
    profile: Option<ExtractionProfile>,
) -> Result<Option<rstyle_pdg::nominal_boundary::UnitContext>, String> {
    use rstyle_pdg::nominal_boundary::{CONTEXT_FLAG, UnitContext};
    let prefix = format!("{CONTEXT_FLAG}=");
    let context = arguments
        .first()
        .and_then(|arg| arg.strip_prefix(&prefix))
        .map(UnitContext::parse)
        .transpose()?;

    if context.is_some() {
        if profile != Some(ExtractionProfile::DataV1) {
            return Err("Selected-unit context requires hir-data-v1".into());
        }
        arguments.remove(0);
    }
    if arguments
        .iter()
        .any(|arg| arg == CONTEXT_FLAG || arg.starts_with(&prefix))
    {
        return Err("Selected-unit context must occur once before the rustc path".into());
    }
    Ok(context)
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
    fn selected_unit_is_private_typed_and_data_only() {
        use rstyle_pdg::{
            concrete_types::digest,
            nominal_binding::{PACKAGE_IDENTITY, PackageContext},
            nominal_boundary::{CONTEXT_FLAG, UnitContext},
        };
        let context = UnitContext {
            package: PackageContext {
                identity: PACKAGE_IDENTITY.into(),
                package_id: "sample".into(),
                name: "sample".into(),
                version: "1.0.0".into(),
                manifest_path: "/sample/Cargo.toml".into(),
                manifest_bytes: "".into(),
                manifest_digest: digest(b""),
            },
            root_source: "/sample/lib.rs".into(),
            crate_name: "sample".into(),
        };
        let flag = format!("{CONTEXT_FLAG}={}", context.transport().unwrap());
        let selection = format!("{DRIVER_SELECTION_FLAG}=hir-data-v1");
        let invocation = parse(vec![selection.clone(), flag.clone(), "rustc".into()]).unwrap();
        assert_eq!(invocation.unit_context, Some(context));
        assert_eq!(invocation.arguments, args(&["rustc"]));
        for invalid in [
            vec![flag.clone(), "rustc".into()],
            vec![
                format!("{DRIVER_SELECTION_FLAG}=hir-helpers-v1"),
                flag.clone(),
                "rustc".into(),
            ],
            vec![
                selection.clone(),
                flag.clone(),
                flag.clone(),
                "rustc".into(),
            ],
            vec![selection.clone(), "rustc".into(), flag],
            vec![selection, format!("{CONTEXT_FLAG}={{}}"), "rustc".into()],
        ] {
            assert!(parse(invalid).is_err());
        }
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
