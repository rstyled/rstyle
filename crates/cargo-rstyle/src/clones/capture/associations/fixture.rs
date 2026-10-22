use super::super::*;
use crate::clones::capture::Target;
use rstyle_pdg::extraction_profile::ExtractionProfile;

pub(super) struct Fixture {
    pub directory: tempfile::TempDir,
    pub units: Vec<Unit>,
    pub functions: Vec<Vec<HirFnFacts>>,
    pub cargo: Vec<Value>,
    pub graph: Value,
}

impl Fixture {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let units = vec![
            unit("consumer", directory.path()),
            unit("calendar", directory.path()),
        ];
        let metadata = emitted(&units[1]);
        std::fs::write(&metadata, native_metadata()).unwrap();
        let mut units = units;
        units[0]
            .invocation
            .extend(["--extern".into(), format!("renamed={}", metadata.display())]);
        let foreign = json!({"identity":IDENTITY,"crate_name":"calendar","stable_crate_id":"stable-calendar",
            "crate_hash":"0123456789abcdeffedcba9876543210","rmeta":metadata,"instance":{"type_arguments":[],"const_arguments":[]}});
        let mut functions = units
            .iter()
            .map(|unit| vec![fact(unit)])
            .collect::<Vec<_>>();
        functions[0][0].body.res = Some(serde_json::from_value(json!({
            "kind":"resolved_direct_callee","target":null,"description":json!({"path":"calendar::days",
                "crate":"calendar","local":false,"definition_identity":"days-id","compiled_dependency":foreign}).to_string()
        })).unwrap());
        let cargo = units.iter().map(artifact).collect::<Vec<_>>();
        let mut graph_units = units.iter().zip(&cargo).map(|(unit, artifact)| json!({
            "pkg_id":unit.target.package,"target":artifact["target"],"mode":"check","platform":null,
            "features":[],"profile":artifact["profile"],"dependencies":[]
        })).collect::<Vec<_>>();
        graph_units[0]["dependencies"] = json!([{"index":1,"extern_crate_name":"renamed"}]);
        let graph = json!({"version":1,"roots":[0],"units":graph_units});
        Self {
            directory,
            units,
            functions,
            cargo,
            graph,
        }
    }

    pub fn run(&self) -> Result<Vec<Value>, String> {
        let observations = self
            .functions
            .iter()
            .enumerate()
            .map(|(index, functions)| ObservedUnit {
                namespace: serde_json::from_value(
                    json!({"process":1,"epoch_nanos":10,"ordinal":index}),
                )
                .unwrap(),
                functions,
            })
            .collect::<Vec<_>>();
        associate(Request {
            graph: &self.graph,
            cargo: &self.cargo,
            units: &self.units,
            observations: &observations,
        })
    }

    pub fn change_foreign(&mut self, change: impl FnOnce(&mut Value)) {
        let resolution = self.functions[0][0].body.res.as_mut().unwrap();
        let mut value: Value = serde_json::from_str(&resolution.description).unwrap();
        change(&mut value);
        resolution.description = value.to_string();
    }
}

fn unit(name: &str, directory: &Path) -> Unit {
    Unit {
        extraction_profile: ExtractionProfile::DataV3,
        target: Target {
            package: format!("registry+https://example.invalid#{name}@1.0.0"),
            manifest_dir: directory.into(),
            package_context: None,
            source: directory.join(format!("{name}.rs")),
            name: name.into(),
            kind: vec!["lib".into()],
            test: true,
        },
        test: false,
        invocation: vec![
            "rustc".into(),
            "--crate-name".into(),
            name.into(),
            "--out-dir".into(),
            directory.display().to_string(),
            "-C".into(),
            "extra-filename=-selected".into(),
        ],
        directory: directory.into(),
        artifacts: Vec::new(),
    }
}

fn emitted(unit: &Unit) -> PathBuf {
    unit.directory
        .join(format!("lib{}-selected.rmeta", unit.target.name))
}

fn artifact(unit: &Unit) -> Value {
    json!({"reason":"compiler-artifact","package_id":unit.target.package,
        "target":{"name":unit.target.name,"src_path":unit.target.source,"kind":["lib"],"crate_types":["lib"]},
        "features":[],"profile":{"opt_level":"0","debuginfo":2,"debug_assertions":true,"overflow_checks":true,"test":false},
        "filenames":[emitted(unit)]})
}

fn fact(unit: &Unit) -> HirFnFacts {
    let mut fact: HirFnFacts = serde_json::from_str(include_str!("fact.json")).unwrap();
    fact.build.features.clear();
    fact.build.frontend_options.insert(
        COMPILED.into(),
        json!({"identity":IDENTITY,
        "crate_name":unit.target.name,"stable_crate_id":format!("stable-{}",unit.target.name)}),
    );
    fact.build
        .frontend_options
        .insert("invocation".into(), json!(unit.invocation));
    fact
}

// Independent fixed-byte little-endian fixture; it is not produced by the reader.
pub(super) fn native_metadata() -> Vec<u8> {
    let mut bytes = vec![0u8; 64];
    bytes[..8].copy_from_slice(b"rust\0\0\0\x0b");
    bytes[8..16].copy_from_slice(&[48, 0, 0, 0, 0, 0, 0, 0]);
    bytes[16..24].copy_from_slice(&[56, 0, 0, 0, 0, 0, 0, 0]);
    bytes[24..40].copy_from_slice(&[
        0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23,
        0x01,
    ]);
    bytes
}
