//! Call-site probes only; the selected bodies remain in the original dependency.
use regex_automata::Input;

pub fn new_bytes(value: &[u8]) -> Input<'_> {
    Input::new::<[u8]>(value)
}

pub fn from_bytes(value: &[u8]) -> Input<'_> {
    <Input<'_> as From<&[u8]>>::from(value)
}

pub fn new_text(value: &str) -> Input<'_> {
    Input::new::<str>(value)
}

pub fn from_text(value: &str) -> Input<'_> {
    <Input<'_> as From<&str>>::from(value)
}
