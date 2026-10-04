pub fn rolling_checksum(bytes: &[u8]) -> u64 {
    let mut total = 0u64;
    let chunk_size = 4;
    let mut offset = 0;

    while offset < bytes.len() {
        let end = (offset + chunk_size).min(bytes.len());
        for byte in &bytes[offset..end] {
            total = total.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        offset = end;
    }

    total
}
