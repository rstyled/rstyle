//! Package/source observations bound to one independently selected Cargo unit.
//! No data eligibility or effect certificate is issued by this module.

use crate::concrete_types::{Nominal, digest};
use crate::declaration_origins::Origins;
use crate::{HirFnFacts, concrete_types, declaration_origins};
mod builds;
pub use builds::{BuildContexts, BuildId};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

pub const PACKAGE_IDENTITY: &str = "rstyle.rust.cargo-package-capture/1";
pub const IDENTITY: &str = "rstyle.rust.nominal-unit-binding/2";
pub const MAX_MANIFEST_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageContext {
    pub identity: String,
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub manifest_path: PathBuf,
    pub manifest_bytes: String,
    pub manifest_digest: String,
}

impl PackageContext {
    pub fn validate(&self) -> Result<(), String> {
        if self.identity != PACKAGE_IDENTITY
            || self.package_id.is_empty()
            || self.name.is_empty()
            || self.version.is_empty()
            || self.manifest_bytes.len() > MAX_MANIFEST_BYTES
            || self.manifest_digest != digest(self.manifest_bytes.as_bytes())
            || !self.manifest_path.is_absolute()
            || self
                .manifest_path
                .file_name()
                .is_none_or(|name| name != "Cargo.toml")
        {
            return Err("Invalid, inconsistent or over-budget Cargo package context".into());
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundDeclaration {
    pub nominal: Nominal,
    pub source_file: PathBuf,
    pub file_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Binding {
    pub identity: String,
    pub package_id: String,
    pub manifest_digest: String,
    pub build_id: BuildId,
    pub declarations: Vec<BoundDeclaration>,
}

pub struct UnitBindings {
    pub bindings: Vec<Option<Binding>>,
    pub builds: BuildContexts,
}

impl UnitBindings {
    pub fn empty(count: usize) -> Self {
        Self {
            bindings: vec![None; count],
            builds: BuildContexts::default(),
        }
    }
}

/// The context must come from the independently frozen selected Cargo plan,
/// after observed target/unit and frontend invocation checks, not the producer.
pub fn bind_unit(facts: &[HirFnFacts], context: &PackageContext) -> Result<UnitBindings, String> {
    context.validate()?;
    let mut files = BTreeMap::new();
    let mut builds = BuildContexts::default();
    let bindings = facts
        .iter()
        .map(|function| bind(function, context, &mut files, &mut builds))
        .collect::<Result<Vec<_>, _>>()?;
    for function in facts {
        check_function_source(function, &files)?;
    }

    Ok(UnitBindings { bindings, builds })
}

fn bind(
    facts: &HirFnFacts,
    context: &PackageContext,
    files: &mut BTreeMap<PathBuf, std::sync::Arc<str>>,
    builds: &mut BuildContexts,
) -> Result<Option<Binding>, String> {
    let options = &facts.build.frontend_options;
    if !options.contains_key(concrete_types::KEY) {
        if options.contains_key(declaration_origins::KEY) {
            return Err("Orphan concrete declaration origins".into());
        }
        return Ok(None);
    }
    let interface = options.data::<crate::concrete_types::InterfaceObservations>()?;
    if !options.contains_key(declaration_origins::KEY) {
        return Err("Missing original declaration evidence".into());
    }
    let origins = options.data::<Origins>()?;
    declaration_origins::verify(&interface, &origins)?;
    let declarations = origins
        .declarations
        .iter()
        .map(|origin| bind_declaration(origin, context, files))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(Binding {
        identity: IDENTITY.into(),
        package_id: context.package_id.clone(),
        manifest_digest: context.manifest_digest.clone(),
        build_id: builds.intern(&facts.build)?,
        declarations,
    }))
}

fn bind_declaration(
    origin: &declaration_origins::Origin,
    context: &PackageContext,
    files: &mut BTreeMap<PathBuf, std::sync::Arc<str>>,
) -> Result<BoundDeclaration, String> {
    let nominal = &origin.nominal;
    if nominal
        .source_identity
        .as_ref()
        .is_some_and(|id| id != &context.package_id)
        || nominal.package_name != context.name
        || nominal.package_version != context.version
        || nominal.manifest_digest != context.manifest_digest
    {
        return Err("Nominal package evidence disagrees with selected Cargo package".into());
    }
    let path = absolute_path(Path::new(&origin.source_file))?;
    let root = absolute_path(
        context
            .manifest_path
            .parent()
            .ok_or("Missing package root")?,
    )?;
    if !path.starts_with(root) {
        return Err("Declaration source lies outside the selected package root".into());
    }
    if let Some(previous) = files.get(&path) {
        if previous != &origin.file_bytes {
            return Err("Contradictory compiler-file bytes in one selected unit".into());
        }
    } else {
        files.insert(path.clone(), origin.file_bytes.clone());
    }

    Ok(BoundDeclaration {
        nominal: nominal.clone(),
        source_file: path,
        file_digest: origin.file_digest.clone(),
    })
}

fn check_function_source(
    facts: &HirFnFacts,
    files: &BTreeMap<PathBuf, std::sync::Arc<str>>,
) -> Result<(), String> {
    if files.is_empty() {
        return Ok(());
    }
    let name = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing captured function source-file identity")?;
    let path = absolute_path(Path::new(name))?;
    let Some(source) = files.get(&path) else {
        return Ok(());
    };

    check_range(source, facts.fn_span, &facts.source)
}

fn check_range(
    source: &str,
    span: crate::ByteSpan,
    function: &crate::SourceFacts,
) -> Result<(), String> {
    if function.digest != function.bytes.identity()
        || source.get(span.start as usize..span.end as usize) != Some(function.bytes.as_str())
    {
        return Err(
            "Captured function range or digest disagrees with original declaration file".into(),
        );
    }

    Ok(())
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("Nominal binding requires an absolute source path".into());
    }
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir if !result.pop() => {
                return Err("Source path escapes its root".into());
            }
            Component::ParentDir | Component::CurDir => {}
            other => result.push(other.as_os_str()),
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(root: &str) -> PackageContext {
        let manifest = "[package]\nname = 'sample'\nversion = '1.2.3'\n";
        PackageContext {
            identity: PACKAGE_IDENTITY.into(),
            package_id: format!("path+file://{root}#sample@1.2.3"),
            name: "sample".into(),
            version: "1.2.3".into(),
            manifest_path: Path::new(root).join("Cargo.toml"),
            manifest_bytes: manifest.into(),
            manifest_digest: digest(manifest.as_bytes()),
        }
    }

    fn origin(context: &PackageContext) -> declaration_origins::Origin {
        let source = "pub struct Counter;";
        declaration_origins::Origin {
            nominal: Nominal {
                source_identity: None,
                package_name: context.name.clone(),
                package_version: context.version.clone(),
                manifest_digest: context.manifest_digest.clone(),
                definition_path: "sample::Counter".into(),
                declaration_digest: digest(source.as_bytes()),
            },
            source_file: context
                .manifest_path
                .parent()
                .unwrap()
                .join("src/types.rs")
                .to_str()
                .unwrap()
                .into(),
            file_bytes: source.into(),
            file_digest: digest(source.as_bytes()),
            declaration_span: crate::ByteSpan {
                start: 0,
                end: source.len() as u32,
            },
        }
    }

    #[test]
    fn identical_package_spelling_and_bytes_keep_distinct_source_namespaces() {
        let first = context("/one/sample");
        let second = context("/two/sample");
        assert_eq!(first.manifest_digest, second.manifest_digest);
        assert_ne!(first.package_id, second.package_id);
        let a = bind_declaration(&origin(&first), &first, &mut BTreeMap::new()).unwrap();
        let b = bind_declaration(&origin(&second), &second, &mut BTreeMap::new()).unwrap();
        assert_eq!(a.nominal, b.nominal);
        assert_ne!(a.source_file, b.source_file);
        assert!(bind_declaration(&origin(&first), &second, &mut BTreeMap::new()).is_err());
    }

    #[test]
    fn data_nominal_namespaces_bind_to_the_exact_selected_package() {
        let first = context("/one/sample");
        let second = context("/two/sample");
        let mut a = origin(&first);
        let mut b = origin(&second);
        let historical = serde_json::to_value(&a.nominal).unwrap();
        assert!(historical.get("source_identity").is_none());
        a.nominal.source_identity = Some(first.package_id.clone());
        b.nominal.source_identity = Some(second.package_id.clone());
        assert_ne!(a.nominal, b.nominal);
        bind_declaration(&a, &first, &mut BTreeMap::new()).unwrap();
        a.nominal.source_identity = b.nominal.source_identity;
        assert!(bind_declaration(&a, &first, &mut BTreeMap::new()).is_err());
    }

    #[test]
    fn selected_package_and_manifest_evidence_must_agree() {
        let context = context("/one/sample");
        context.validate().unwrap();
        for change in ["name", "version", "digest", "relative", "escape"] {
            let mut changed = origin(&context);
            match change {
                "name" => changed.nominal.package_name.push('x'),
                "version" => changed.nominal.package_version.push('x'),
                "digest" => changed.nominal.manifest_digest = digest(b"different manifest"),
                "relative" => changed.source_file = "src/types.rs".into(),
                "escape" => changed.source_file = "/one/sample/../other/types.rs".into(),
                _ => unreachable!(),
            }
            assert!(
                bind_declaration(&changed, &context, &mut BTreeMap::new()).is_err(),
                "{change}"
            );
        }
        let mut corrupt = context.clone();
        corrupt.manifest_bytes.push('x');
        assert!(corrupt.validate().is_err());
        corrupt = context;
        corrupt.manifest_bytes = "x".repeat(MAX_MANIFEST_BYTES + 1);
        corrupt.manifest_digest = digest(corrupt.manifest_bytes.as_bytes());
        assert!(corrupt.validate().is_err());
    }

    #[test]
    fn one_unit_cannot_assign_different_bytes_to_one_source_name() {
        let context = context("/one/sample");
        let original = origin(&context);
        let mut files = BTreeMap::new();
        bind_declaration(&original, &context, &mut files).unwrap();
        bind_declaration(&original, &context, &mut files).unwrap();
        let mut changed = original;
        changed.file_bytes = format!("{}\n// changed source", changed.file_bytes).into();
        changed.file_digest = digest(changed.file_bytes.as_bytes());
        assert!(bind_declaration(&changed, &context, &mut files).is_err());
    }

    #[test]
    fn function_ranges_require_exact_original_bytes_including_bom_crlf() {
        let function = "pub fn get() -> i32 { 3 }";
        let file = format!("\u{feff}pub struct Counter;\r\n{function}\r\n");
        let start = file.find(function).unwrap() as u32;
        let span = crate::ByteSpan {
            start,
            end: start + function.len() as u32,
        };
        let mut evidence = crate::SourceFacts {
            bytes: function.into(),
            digest: digest(function.as_bytes()),
        };
        check_range(&file, span, &evidence).unwrap();
        evidence.bytes = function.replace('3', "4").into();
        evidence.digest = digest(evidence.bytes.as_bytes());
        assert!(check_range(&file, span, &evidence).is_err());
        evidence.bytes = function.into();
        assert!(check_range(&file, span, &evidence).is_err());
        evidence.digest = digest(function.as_bytes());
        assert!(
            check_range(
                &file,
                crate::ByteSpan {
                    start: start + 1,
                    ..span
                },
                &evidence
            )
            .is_err()
        );
    }
}
