// Appended inside the unchanged original wire module. No replacement bodies.
#[doc(hidden)]
pub(crate) fn __rstyle_usize_reader_probe(
    api: u8,
    slice: &[u8],
    what: &'static str,
) -> (u8, u64, usize, &'static str) {
    let result = match api {
        0 => try_read_u16_as_usize(slice, what).map(|(n, count)| (n as u64, count)),
        1 => try_read_u32_as_usize(slice, what).map(|(n, count)| (n as u64, count)),
        2 => try_read_u16(slice, what).map(|(n, count)| (u64::from(n), count)),
        3 => try_read_u32(slice, what).map(|(n, count)| (u64::from(n), count)),
        4 => Err(DeserializeError::buffer_too_small(what)),
        5 => Err(DeserializeError::invalid_usize(what)),
        _ => panic!("unknown oracle API"),
    };
    match result {
        Ok((value, count)) => (0, value, count, ""),
        Err(DeserializeError(DeserializeErrorKind::BufferTooSmall { what })) => (1, 0, 0, what),
        Err(DeserializeError(DeserializeErrorKind::InvalidUsize { what })) => (2, 0, 0, what),
        Err(_) => panic!("unexpected original reader error"),
    }
}
