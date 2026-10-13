//! Per-invocation core bytes from compiler-selected crate sources, not a cache.

use super::Options;
use rstyle_pdg::{HirFnFacts, HirNode, data_library::*};
use rustc_hir::{
    HirId, ItemLocalId, OwnerId,
    def_id::{DefId, DefIndex, LocalDefId},
};
use rustc_middle::ty::TyCtxt;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, path::Path};

pub fn capture(
    tcx: TyCtxt<'_>,
    options: &Options<'_>,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    if options.selected() != rstyle_pdg::extraction_profile::ExtractionProfile::DataV1 {
        return Ok(());
    }
    let mut callees = BTreeMap::new();
    for function in facts.iter() {
        collect(tcx, &function.body, &mut callees)?;
    }

    let libraries = callees
        .into_values()
        .map(|callee| library(tcx, callee))
        .collect::<Result<Vec<_>, _>>()?;
    let artifacts = Artifacts {
        identity: IDENTITY.into(),
        libraries,
    };
    let value = serde_json::to_value(artifacts).map_err(|error| error.to_string())?;
    for function in facts {
        function
            .build
            .frontend_options
            .insert(KEY.into(), value.clone());
    }
    Ok(())
}

fn collect(
    tcx: TyCtxt<'_>,
    node: &HirNode,
    callees: &mut BTreeMap<String, DefId>,
) -> Result<(), String> {
    if operation(node).is_some() {
        let id = HirId {
            owner: OwnerId {
                def_id: LocalDefId {
                    local_def_index: DefIndex::from_u32(node.hir.owner),
                },
            },
            local_id: ItemLocalId::from_u32(node.hir.local),
        };
        let callee = tcx
            .typeck(id.owner.def_id)
            .type_dependent_def_id(id)
            .ok_or("Missing compiler-selected library callee")?;
        if !callee.is_local() && tcx.crate_name(callee.krate).as_str() == "core" {
            callees.insert(tcx.crate_hash(callee.krate).to_string(), callee);
        }
    }
    for child in &node.children {
        collect(tcx, child, callees)?;
    }
    Ok(())
}

fn library(tcx: TyCtxt<'_>, callee: DefId) -> Result<Library, String> {
    let source = tcx.used_crate_source(callee.krate);
    let files = [
        ("rmeta", source.rmeta.as_ref()),
        ("rlib", source.rlib.as_ref()),
        ("dylib", source.dylib.as_ref()),
        ("sdylib_interface", source.sdylib_interface.as_ref()),
    ]
    .into_iter()
    .filter_map(|(format, path)| path.map(|path| (format, path)))
    .map(|(format, path)| file(format, path))
    .collect::<Result<Vec<_>, _>>()?;
    if files.is_empty() {
        return Err("Compiler-selected core has no artifact source".into());
    }
    Ok(Library {
        crate_name: "core".into(),
        crate_hash: tcx.crate_hash(callee.krate).to_string(),
        files,
    })
}

fn file(format: &str, path: &Path) -> Result<File, String> {
    let input = open_source(path)?;
    let checksum = hash_stream(input)?;
    descriptor(format, path, checksum)
}

fn open_source(path: &Path) -> Result<std::fs::File, String> {
    let input = std::fs::File::open(path).map_err(|error| error.to_string())?;
    if input.metadata().map_err(|error| error.to_string())?.len() > MAX_FILE_BYTES {
        return Err("Core artifact exceeds capture byte budget".into());
    }
    Ok(input)
}

struct Checksum {
    digest: Sha256,
    bytes: u64,
}
impl Checksum {
    fn push(&mut self, chunk: &[u8]) -> Result<(), String> {
        self.bytes += chunk.len() as u64;
        if self.bytes > MAX_FILE_BYTES {
            return Err("Core artifact exceeds capture byte budget".into());
        }
        self.digest.update(chunk);
        Ok(())
    }
}

fn hash_stream(mut input: std::fs::File) -> Result<Checksum, String> {
    let mut checksum = Checksum {
        digest: Sha256::new(),
        bytes: 0,
    };
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = input.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        checksum.push(&buffer[..count])?;
    }
    Ok(checksum)
}

fn descriptor(format: &str, path: &Path, checksum: Checksum) -> Result<File, String> {
    Ok(File {
        format: format.into(),
        path: path.to_str().ok_or("Non-UTF8 core artifact path")?.into(),
        bytes: checksum.bytes,
        sha256: format!("sha256:{:x}", checksum.digest.finalize()),
    })
}
