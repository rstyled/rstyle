//! Original file references, retaining complete semantic evidence and nulls.
use super::*;
use crate::{
    ByteSpan,
    concrete_types::Nominal,
    declaration_origins::{BuiltinCopy, GeneratedOrigin, Origin, Origins},
    nominal_boundary::{
        UnitContext,
        census::{Census, UnitFile},
    },
    source_table::{Imported, Owners, SourceId},
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
pub(super) struct CensusExport<'a> {
    identity: &'a str,
    context: &'a UnitContext,
    files: Vec<FileExport<'a>>,
    #[serde(
        skip_serializing_if = "<[crate::nominal_boundary::census::expansions::ModuleExpansion]>::is_empty"
    )]
    expansions: &'a [crate::nominal_boundary::census::expansions::ModuleExpansion],
    #[serde(
        skip_serializing_if = "<[crate::nominal_boundary::census::documentation::Input]>::is_empty"
    )]
    documentation: &'a [crate::nominal_boundary::census::documentation::Input],
    #[serde(
        skip_serializing_if = "<[crate::nominal_boundary::census::external_imports::Import]>::is_empty"
    )]
    external_imports: &'a [crate::nominal_boundary::census::external_imports::Import],
}
#[derive(Serialize)]
struct FileExport<'a> {
    path: &'a Path,
    snapshot: SourceId,
    digest: &'a str,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CensusImport {
    identity: String,
    context: UnitContext,
    files: Vec<FileImport>,
    #[serde(default)]
    expansions: Vec<crate::nominal_boundary::census::expansions::ModuleExpansion>,
    #[serde(default)]
    documentation: Vec<crate::nominal_boundary::census::documentation::Input>,
    #[serde(default)]
    external_imports: Vec<crate::nominal_boundary::census::external_imports::Import>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileImport {
    path: PathBuf,
    snapshot: SourceId,
    digest: String,
}

impl<'a> CensusExport<'a> {
    pub fn new(census: &'a Census, sources: &mut Owners<'a>) -> Result<Self, String> {
        let files = census
            .files
            .iter()
            .map(|file| {
                Ok(FileExport {
                    path: &file.path,
                    snapshot: sources.snapshot(&file.bytes)?,
                    digest: &file.digest,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Self {
            identity: &census.identity,
            context: &census.context,
            expansions: &census.expansions,
            documentation: &census.documentation,
            external_imports: &census.external_imports,
            files,
        })
    }
}
pub(super) trait SourceRecord {
    fn decode(self, sources: &mut Imported<'_>) -> Result<Entry, String>;
}

impl SourceRecord for CensusImport {
    fn decode(self, sources: &mut Imported<'_>) -> Result<Entry, String> {
        let files = self
            .files
            .into_iter()
            .map(|file| {
                Ok(UnitFile {
                    path: file.path,
                    bytes: sources.snapshot(file.snapshot)?,
                    digest: file.digest,
                })
            })
            .collect::<Result<_, String>>()?;
        let census = Census {
            identity: self.identity,
            context: self.context,
            expansions: self.expansions,
            documentation: self.documentation,
            external_imports: self.external_imports,
            files,
        };
        Ok(Entry::Census(Record::new(Arc::new(census), Vec::new())))
    }
}

#[derive(Serialize)]
pub(super) struct OriginsExport<'a> {
    identity: &'a str,
    declarations: Vec<OriginExport<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    generated: Vec<GeneratedExport<'a>>,
    nulls: &'a [String],
}
#[derive(Serialize)]
struct GeneratedExport<'a> {
    nominal: &'a Nominal,
    definition: &'a crate::generated_sources::capture::Definition,
    origin: &'a crate::generated_sources::capture::Origin,
    defining_expansion: &'a str,
    snapshot: SourceId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratedImport {
    nominal: Nominal,
    definition: crate::generated_sources::capture::Definition,
    origin: crate::generated_sources::capture::Origin,
    defining_expansion: String,
    snapshot: SourceId,
}
#[derive(Serialize)]
struct OriginExport<'a> {
    nominal: &'a Nominal,
    source_file: &'a str,
    snapshot: SourceId,
    file_digest: &'a str,
    declaration_span: ByteSpan,
    #[serde(skip_serializing_if = "Option::is_none")]
    builtin_copy: Option<&'a BuiltinCopy>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OriginsImport {
    identity: String,
    declarations: Vec<OriginImport>,
    #[serde(default)]
    generated: Vec<GeneratedImport>,
    nulls: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginImport {
    nominal: Nominal,
    source_file: String,
    snapshot: SourceId,
    file_digest: String,
    declaration_span: ByteSpan,
    #[serde(default)]
    builtin_copy: Option<BuiltinCopy>,
}
impl<'a> OriginsExport<'a> {
    pub fn new(record: &'a Record<Origins>, sources: &mut Owners<'a>) -> Result<Self, String> {
        let declarations = record
            .value
            .declarations
            .iter()
            .map(|origin| {
                Ok(OriginExport {
                    nominal: &origin.nominal,
                    source_file: &origin.source_file,
                    snapshot: sources.snapshot(&origin.file_bytes)?,
                    file_digest: &origin.file_digest,
                    declaration_span: origin.declaration_span,
                    builtin_copy: origin.builtin_copy.as_ref(),
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Self {
            identity: &record.value.identity,
            declarations,
            generated: record
                .value
                .generated
                .iter()
                .map(|origin| {
                    Ok(GeneratedExport {
                        nominal: &origin.nominal,
                        definition: &origin.definition,
                        origin: &origin.origin,
                        defining_expansion: &origin.defining_expansion,
                        snapshot: sources.snapshot(&origin.file_bytes)?,
                    })
                })
                .collect::<Result<_, String>>()?,
            nulls: &record.nulls,
        })
    }
}
impl SourceRecord for OriginsImport {
    fn decode(self, sources: &mut Imported<'_>) -> Result<Entry, String> {
        let declarations = self
            .declarations
            .into_iter()
            .map(|origin| {
                Ok(Origin {
                    nominal: origin.nominal,
                    source_file: origin.source_file,
                    file_bytes: sources.snapshot(origin.snapshot)?,
                    file_digest: origin.file_digest,
                    declaration_span: origin.declaration_span,
                    builtin_copy: origin.builtin_copy,
                })
            })
            .collect::<Result<_, String>>()?;
        let origins = Origins {
            identity: self.identity,
            declarations,
            generated: self
                .generated
                .into_iter()
                .map(|origin| {
                    Ok(GeneratedOrigin {
                        nominal: origin.nominal,
                        definition: origin.definition,
                        origin: origin.origin,
                        defining_expansion: origin.defining_expansion,
                        file_bytes: sources.snapshot(origin.snapshot)?,
                    })
                })
                .collect::<Result<_, String>>()?,
        };
        check_nulls(&origins, &self.nulls)?;
        Ok(Entry::Origins(Record::new(origins, self.nulls)))
    }
}

fn check_nulls(origins: &Origins, nulls: &[String]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for path in nulls {
        let nominal = null_nominal(origins, path);
        if nominal.is_none_or(|nominal| nominal.source_identity.is_some()) || !seen.insert(path) {
            return Err("Invalid source-origin null-presence record".into());
        }
    }
    Ok(())
}

fn null_nominal<'a>(origins: &'a Origins, path: &str) -> Option<&'a Nominal> {
    for prefix in ["declarations", "generated"] {
        let index = path
            .strip_prefix(&format!("/{prefix}/"))
            .and_then(|path| path.strip_suffix("/nominal"))
            .and_then(|index| index.parse::<usize>().ok())
            .filter(|index| path == format!("/{prefix}/{index}/nominal"));
        if let Some(index) = index {
            return if prefix == "declarations" {
                origins
                    .declarations
                    .get(index)
                    .map(|origin| &origin.nominal)
            } else {
                origins.generated.get(index).map(|origin| &origin.nominal)
            };
        }
    }
    None
}
