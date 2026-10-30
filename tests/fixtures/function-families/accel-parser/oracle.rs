// Appended to the complete original dfa::accel module, never a copied body.
#[doc(hidden)]
pub(crate) fn __rstyle_accel_parser_probe(api: u8, input: &[u8]) -> (u8, [u8; 8], &'static str) {
    let result = match api {
        0 => Accel::from_slice(input),
        1 => Accel::from_bytes([input[0], input[1], input[2], input[3]]),
        2 => Ok(Accel::from_bytes_unchecked([
            input[0], input[1], input[2], input[3],
        ])),
        _ => panic!("unknown oracle API"),
    };
    match result {
        Ok(accel) => (0, accel.bytes, ""),
        Err(error) => {
            let (tag, message) = crate::util::wire::__rstyle_accel_parser_error(error);
            (tag, [0; 8], message)
        }
    }
}
