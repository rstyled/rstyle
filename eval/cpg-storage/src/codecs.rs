use crate::{
    archive_generated::cpg::storage as fb,
    model::{Archive, Document, Value},
};
use clap::ValueEnum;
use prost::Message;
use serde::Serialize;

#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Codec {
    Json,
    Flatbuffers,
    Flexbuffers,
    Protobuf,
}

pub struct Buffer {
    storage: Vec<u8>,
    start: usize,
}

impl Buffer {
    pub fn as_slice(&self) -> &[u8] {
        &self.storage[self.start..]
    }
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }
}

impl From<Vec<u8>> for Buffer {
    fn from(storage: Vec<u8>) -> Self {
        Self { storage, start: 0 }
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl std::ops::Deref for Buffer {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

pub fn encode(codec: Codec, archive: &Archive) -> Result<Buffer, String> {
    match codec {
        Codec::Json => serde_json::to_vec(archive)
            .map(Buffer::from)
            .map_err(|e| e.to_string()),
        Codec::Flexbuffers => flexbuffers::to_vec(archive)
            .map(Buffer::from)
            .map_err(|e| e.to_string()),
        Codec::Protobuf => Ok(archive.encode_to_vec().into()),
        Codec::Flatbuffers => flat_encode(archive),
    }
}

pub fn decode(codec: Codec, bytes: &[u8]) -> Result<Archive, String> {
    match codec {
        Codec::Json => serde_json::from_slice(bytes).map_err(|e| e.to_string()),
        Codec::Flexbuffers => flexbuffers::from_slice(bytes).map_err(|e| e.to_string()),
        Codec::Protobuf => Archive::decode(bytes).map_err(|e| e.to_string()),
        Codec::Flatbuffers => flat_decode(bytes),
    }
}

fn flat_encode(archive: &Archive) -> Result<Buffer, String> {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let strings: Vec<_> = archive
        .strings
        .iter()
        .map(|s| builder.create_string(s))
        .collect();
    let strings = builder.create_vector(&strings);
    let values = builder.create_vector_from_iter(
        archive
            .values
            .iter()
            .map(|v| fb::Value::new(v.payload, v.start, v.length, v.kind)),
    );
    let edges = builder.create_vector(&archive.edges);
    let documents = builder.create_vector_from_iter(
        archive
            .documents
            .iter()
            .map(|d| fb::Document::new(d.path, d.root)),
    );
    let root = fb::Archive::create(
        &mut builder,
        &fb::ArchiveArgs {
            version: archive.version,
            strings: Some(strings),
            values: Some(values),
            edges: Some(edges),
            documents: Some(documents),
        },
    );
    fb::finish_archive_buffer(&mut builder, root);
    // Transfer builder storage without copying the full output buffer.
    let (storage, start) = builder.collapse();
    Ok(Buffer { storage, start })
}

fn flat_decode(bytes: &[u8]) -> Result<Archive, String> {
    if !fb::archive_buffer_has_identifier(bytes) {
        return Err("Wrong FlatBuffers identifier".into());
    }
    let root = fb::root_as_archive(bytes).map_err(|e| e.to_string())?;
    Ok(Archive {
        version: root.version(),
        strings: root.strings().iter().map(str::to_owned).collect(),
        values: root
            .values()
            .iter()
            .map(|v| Value {
                kind: v.kind(),
                payload: v.payload(),
                start: v.start(),
                length: v.length(),
            })
            .collect(),
        edges: root.edges().iter().collect(),
        documents: root
            .documents()
            .iter()
            .map(|d| Document {
                path: d.path(),
                root: d.root(),
            })
            .collect(),
    })
}
