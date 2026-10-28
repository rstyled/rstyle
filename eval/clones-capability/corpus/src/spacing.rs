//! Direct statement counts are deliberately different from physical line counts.

pub fn four_direct_statements(value: u32) -> u32 {
    let first = value + 1;
    let second = first + 1;
    let third = second + 1;
    third
}

pub fn five_direct_statements(value: u32) -> u32 {
    let first = value + 1;
    let second = first + 1;
    let third = second + 1;
    let fourth = third + 1;
    fourth
}

pub fn five_with_blank_line(value: u32) -> u32 {
    let first = value + 1;
    let second = first + 1;

    let third = second + 1;
    let fourth = third + 1;
    fourth
}

pub fn nested_closure(value: u32) -> u32 {
    let compute = |input: u32| {
        let first = input + 1;
        let second = first + 1;
        let third = second + 1;
        let fourth = third + 1;
        fourth
    };
    compute(value)
}

pub fn multiline_initializer(value: u32) -> u32 {
    let first = {
        let next = value + 1;
        next + 1
    };
    let second = first + 1;
    second
}

// rstyle-allow(fn_spacing): keep this short arithmetic sequence together
pub fn suppressed_five(value: u32) -> u32 {
    let first = value + 1;
    let second = first + 1;
    let third = second + 1;
    let fourth = third + 1;
    fourth
}
