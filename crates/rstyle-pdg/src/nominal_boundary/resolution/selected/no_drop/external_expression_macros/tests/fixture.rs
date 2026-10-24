//! Original powerfmt buffers and retained time export observation. Producer
//! local records are reconstructed test inputs, never captured source authority.
use super::super::*;
use crate::{
    dependency_namespaces::capture::{Binding, Declaration, Site},
    nominal_binding::{PACKAGE_IDENTITY, PackageContext},
    nominal_boundary::{UnitContext, census::UnitFile},
};
use syn::spanned::Spanned;

pub(super) struct Fixture {
    pub census: Arc<Census>,
    pub namespaces: UnitCapture,
    pub export: Export,
    pub definition: FileSpan,
    pub cfgs: Vec<String>,
}

impl Fixture {
    pub fn original() -> Self {
        let census = census();
        let export: Export = serde_json::from_str(include_str!("export.json")).unwrap();
        let (definition, declaration, leaf) = coordinates(&census.files[1]);
        let namespaces = namespaces(&export, declaration, leaf);
        Self {
            census,
            namespaces,
            export,
            definition,
            cfgs: vec![],
        }
    }

    pub fn input(&self) -> Input<'_> {
        Input {
            census: Arc::clone(&self.census),
            namespaces: &self.namespaces,
            export: &self.export,
            definition: &self.definition,
            cfgs: &self.cfgs,
        }
    }
}

fn file(name: &str, bytes: &str) -> UnitFile {
    let bytes: crate::SourceSnapshot = bytes.into();
    UnitFile {
        path: format!("/original/powerfmt/src/{name}").into(),
        digest: bytes.checksum().into(),
        bytes,
    }
}

fn census() -> Arc<Census> {
    let manifest: crate::SourceSnapshot = include_str!("Cargo.toml.txt").into();
    Arc::new(Census {
        identity: crate::nominal_boundary::census::IDENTITY.into(),
        context: UnitContext {
            crate_name: "powerfmt".into(),
            root_source: "/original/powerfmt/src/lib.rs".into(),
            package: PackageContext {
                identity: PACKAGE_IDENTITY.into(),
                package_id: "registry+original#powerfmt@0.2.0".into(),
                name: "powerfmt".into(),
                version: "0.2.0".into(),
                manifest_path: "/original/powerfmt/Cargo.toml".into(),
                manifest_digest: manifest.checksum().into(),
                manifest_bytes: manifest,
            },
        },
        files: vec![
            file("lib.rs", include_str!("../lib.rs.txt")),
            file("smart_display.rs", source::MODULE),
        ],
        expansions: vec![],
        documentation: vec![],
        external_imports: vec![],
    })
}

fn coordinates(file: &UnitFile) -> (FileSpan, FileSpan, FileSpan) {
    let parsed = syn::parse_file(&file.bytes).unwrap();
    let definition = parsed
        .items
        .iter()
        .find(|item| {
            matches!(item, syn::Item::Macro(item)
        if item.ident.as_ref().is_some_and(|name| name == "__not_public_at_root__padded_width_of"))
        })
        .unwrap();
    // Independently retained rustc spans from run-YMvQCn, invocation-dtH0fe.
    // In particular the declaration starts at `pub`, after its cfg attribute.
    let observed = |start, end| FileSpan {
        path: file.path.clone(),
        digest: file.digest.clone(),
        span: crate::ByteSpan { start, end },
    };
    (
        span(file, definition.span()),
        observed(8241, 8306),
        observed(8249, 8286),
    )
}

fn span(file: &UnitFile, span: proc_macro2::Span) -> FileSpan {
    FileSpan {
        path: file.path.clone(),
        digest: file.digest.clone(),
        span: crate::source_evidence::original_span(&file.bytes, span),
    }
}

fn local(definition: &Definition) -> Definition {
    let mut local = definition.clone();
    local.local = true;
    local.path = local.path.strip_prefix("powerfmt::").unwrap().into();
    local
}

fn namespaces(export: &Export, declaration: FileSpan, leaf: FileSpan) -> UnitCapture {
    let target = local(&export.target);
    let definition = local(&export.route[0].declaration);
    let owner = Definition {
        path: "smart_display".into(),
        definition_identity: "test:original-smart-display-owner".into(),
        ..target.clone()
    };
    let site = Site {
        definition,
        declaration,
        leaf,
        owner,
        scope: vec!["smart_display".into()],
        path: vec!["__not_public_at_root__padded_width_of".into()],
        binding: Binding::Named("padded_width_of".into()),
        public: true,
        target,
        target_kind: export.target_kind.clone(),
    };
    UnitCapture {
        identity: crate::dependency_namespaces::capture::IDENTITY.into(),
        imports: vec![],
        declarations: vec![Declaration { site }],
        unsupported: vec![],
    }
}
