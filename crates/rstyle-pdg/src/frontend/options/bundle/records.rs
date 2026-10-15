//! Original file references, retaining complete semantic evidence and nulls.
use super::*;
use crate::{
    ByteSpan,
    concrete_types::Nominal,
    declaration_origins::{Origin, Origins},
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
            files,
        };
        Ok(Entry::Census(Record::new(Arc::new(census), Vec::new())))
    }
}

#[derive(Serialize)]
pub(super) struct OriginsExport<'a> {
    identity: &'a str,
    declarations: Vec<OriginExport<'a>>,
    nulls: &'a [String],
}
#[derive(Serialize)]
struct OriginExport<'a> {
    nominal: &'a Nominal,
    source_file: &'a str,
    snapshot: SourceId,
    file_digest: &'a str,
    declaration_span: ByteSpan,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OriginsImport {
    identity: String,
    declarations: Vec<OriginImport>,
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
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Self {
            identity: &record.value.identity,
            declarations,
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
                })
            })
            .collect::<Result<_, String>>()?;
        let origins = Origins {
            identity: self.identity,
            declarations,
        };
        check_nulls(&origins, &self.nulls)?;
        Ok(Entry::Origins(Record::new(origins, self.nulls)))
    }
}

fn check_nulls(origins: &Origins, nulls: &[String]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for path in nulls {
        let origin = path
            .strip_prefix("/declarations/")
            .and_then(|path| path.strip_suffix("/nominal"))
            .and_then(|index| index.parse::<usize>().ok())
            .filter(|index| path == &format!("/declarations/{index}/nominal"))
            .and_then(|index| origins.declarations.get(index));
        if origin.is_none_or(|origin| origin.nominal.source_identity.is_some())
            || !seen.insert(path)
        {
            return Err("Invalid source-origin null-presence record".into());
        }
    }
    Ok(())
}
