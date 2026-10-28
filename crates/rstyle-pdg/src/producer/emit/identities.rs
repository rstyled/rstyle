//! Document transport consistency; these checksums do not authorize evidence.
use super::{DocumentIdentity, Manifest};
use crate::unit_store::transport_checksum;

pub(super) fn identity(key: String, bytes: &[u8]) -> DocumentIdentity {
    DocumentIdentity {
        key,
        checksum: transport_checksum(bytes),
    }
}

pub(super) fn verify_manifest(
    manifest: &Manifest,
    mut documents: [(String, &[u8]); 2],
) -> Result<(), String> {
    let mut claims: Vec<_> = manifest.documents.iter().collect();
    claims.sort_by(|a, b| a.key.cmp(&b.key));
    documents.sort_by(|a, b| a.0.cmp(&b.0));
    if claims.len() != documents.len()
        || !claims.iter().zip(documents).all(|(claim, (key, bytes))| {
            claim.key == key && claim.checksum == transport_checksum(bytes)
        })
    {
        return Err("External document byte identities or paired keys disagree".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests;
