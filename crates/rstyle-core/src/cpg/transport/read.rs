use super::{Encoding, Observed, SealedDocument};
use serde::{
    Deserialize,
    de::{DeserializeOwned, IgnoredAny, MapAccess, Visitor},
};
use std::{
    fmt,
    fs::File,
    io::{self, BufReader, Read, Seek},
    path::Path,
};

struct Counted<R> {
    inner: R,
    observed: Observed,
    limit: Option<u64>,
}

impl<R: Read> Read for Counted<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let len = self.inner.read(bytes)?;
        self.observed.update(&bytes[..len])?;
        if self.limit.is_some_and(|limit| self.observed.bytes > limit) {
            return Err(io::Error::other("CPG transport exceeds its declared size"));
        }
        Ok(len)
    }
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    read(path, Encoding::from_path(path), None)
}

pub fn read_sealed<T: DeserializeOwned>(
    directory: &Path,
    seal: &SealedDocument,
) -> Result<T, String> {
    read(&directory.join(&seal.file), seal.encoding, Some(seal))
}

fn read<T: DeserializeOwned>(
    path: &Path,
    encoding: Encoding,
    seal: Option<&SealedDocument>,
) -> Result<T, String> {
    let file = File::open(path).map_err(|error| {
        format!(
            "missing captured artifact {}: {error}",
            path.file_name().unwrap_or_default().to_string_lossy()
        )
    })?;
    let mut stored = Counted {
        inner: file,
        observed: Observed::default(),
        limit: seal.map(|s| s.bytes),
    };
    if let Some(seal) = seal {
        io::copy(&mut stored, &mut io::sink()).map_err(|error| error.to_string())?;
        seal.verify_stored(&stored.observed)?;
        stored.inner.rewind().map_err(|error| error.to_string())?;
        stored.observed = Observed::default();
    }
    let value = match encoding {
        Encoding::Json => parsed(BufReader::new(&mut stored), seal),
        Encoding::Zstd => {
            let mut decoder =
                zstd::stream::read::Decoder::new(&mut stored).map_err(|error| error.to_string())?;
            decoder
                .window_log_max(27)
                .map_err(|error| error.to_string())?;
            parsed(decoder, seal)
        }
    }
    .map_err(|error| format!("{}: {error}", path.display()))?;
    if let Some(seal) = seal {
        seal.verify_stored(&stored.observed)?;
    }
    Ok(value)
}

fn parsed<T: DeserializeOwned>(
    reader: impl Read,
    seal: Option<&SealedDocument>,
) -> Result<T, String> {
    let mut decoded = Counted {
        inner: reader,
        observed: Observed::default(),
        limit: seal.and_then(|s| s.decoded_bytes),
    };
    let mut json = serde_json::Deserializer::from_reader(BufReader::new(&mut decoded));
    let value = T::deserialize(&mut json).map_err(|error| error.to_string())?;
    json.end().map_err(|error| error.to_string())?;
    if let Some(seal) = seal {
        seal.verify_decoded(&decoded.observed)?;
    }
    Ok(value)
}

/// Syntax-only object validation without retaining a document tree.
pub fn validate_object(path: &Path, encoding: Encoding) -> Result<(), String> {
    let _: Object = read(path, encoding, None)?;
    Ok(())
}

struct Object;
impl<'de> Deserialize<'de> for Object {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = Object;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a CPG JSON object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Object, A::Error> {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(Object)
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}
