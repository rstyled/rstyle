use spanning::Spanning;

#[derive(Clone, Debug, Spanning)]
pub struct Written(pub u32);

macro_rules! writes {
    ($name:ident) => {
        pub fn $name() {
            std::fs::write("/unused", b"x").ok();
        }
    };
}

writes!(macro_writes_a_file);

pub fn calls_what_a_macro_wrote() {
    macro_writes_a_file();
}

pub fn calls_what_a_derive_wrote(value: &Written) -> Written {
    value.clone()
}

macro_rules! passes {
    ($value:expr) => {
        $value + 1
    };
}

pub fn passed_through(given: u32) -> u32 {
    passes!(given)
}

pub fn raw(r#type: u32) -> u32 {
    let r#match = r#type + 1;
    r#match
}

#[spanning::wrapping]
pub fn wrapped_by_an_attribute(value: u32) -> u32 {
    let doubled = value * 2;
    doubled + 1
}
