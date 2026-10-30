//! Source-authored acceptance cases; labels are independent of detector output.

pub mod excluded;
pub mod spacing;

pub mod source_rules {
    pub fn length_at_limit(value: u32) -> u32 {
        let first = value + 1;
        let second = first + 1;
        second
    }

    pub fn length_over_limit(value: u32) -> u32 {
        let first = value + 1;
        let second = first + 1;
        let third = second + 1;
        third
    }

    pub fn parameters_at_limit(a: u32, b: u32, c: u32, d: u32) -> u32 {
        a + b + c + d
    }

    pub fn parameters_over_limit(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        a + b + c + d + e
    }

    pub fn unsupported_body_parameters(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        let closure = |value: u32| value + 1;
        closure(a + b + c + d + e)
    }
}

pub mod strict_pairs {
    pub fn above_left(value: u32, threshold: u32) -> bool {
        value > threshold
    }

    pub fn above_right(input: u32, minimum: u32) -> bool {
        input > minimum
    }
}

pub mod negative_abstraction {
    pub fn forward_left(value: u32) -> u32 {
        value
    }

    pub fn forward_right(input: u32) -> u32 {
        input
    }
}

pub mod future_guards {
    pub fn nested_positive(value: u32) -> u32 {
        if value > 0 {
            let next = value + 1;
            next + 1
        } else {
            0
        }
    }

    pub fn already_flat(value: u32) -> u32 {
        if value == 0 {
            return 0;
        }
        let next = value + 1;
        next + 1
    }

    pub fn both_branches_do_work(value: u32) -> u32 {
        if value > 4 {
            let next = value + 1;
            next + 1
        } else {
            let previous = value + 2;
            previous + 2
        }
    }
}

pub mod future_differences {
    pub fn add_one(value: u32) -> u32 {
        value + 1
    }

    pub fn add_two(input: u32) -> u32 {
        input + 2
    }

    pub fn subtract_reordered(left: u32, right: u32) -> u32 {
        right - left
    }

    pub fn subtract_original(left: u32, right: u32) -> u32 {
        left - right
    }
}
