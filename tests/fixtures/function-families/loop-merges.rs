//! Preselected scalar loop/merge compositions; expectations are in README.md.

pub fn sequential(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

pub fn after_loop(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    if a == 0 { 1 } else { a }
}

pub fn before_loop(mut a: u128, mut b: u128) -> u128 {
    let fallback = if (a | b) == 0 { 1 } else { 0 };
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a + fallback
}

pub fn tuple_step(mut a: u128, mut b: u128) -> (u128, u128) {
    (a, b) = (b, a % b);
    (a, b)
}

pub fn cyclic_merge(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let remainder = if b == 1 { 0 } else { a % b };
        a = b;
        b = remainder;
    }
    a
}

pub fn memory_target(a: &mut u128, b: u128) {
    (*a,) = (b,);
}
