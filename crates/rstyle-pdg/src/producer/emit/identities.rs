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
            claim.key == key && matches_checksum(&claim.checksum, bytes)
        })
    {
        return Err("External document byte identities or paired keys disagree".into());
    }

    Ok(())
}

fn matches_checksum(claim: &str, bytes: &[u8]) -> bool {
    if claim.starts_with("xxh3-64:") {
        return claim == transport_checksum(bytes);
    }
    // Read historical manifests without computing SHA for new publications.
    if claim.starts_with("sha256:") {
        use sha2::{Digest, Sha256};
        return claim == format!("sha256:{:x}", Sha256::digest(bytes));
    }

    false
}

#[cfg(test)]
mod tests;
