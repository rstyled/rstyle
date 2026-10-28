#![allow(unused_assignments)]
mod excluded;

pub fn empty() {}

pub fn repeated(target: &mut u32) {
    *target = 1;
    *target = 2;
}

pub fn single_write(target: &mut u32) {
    *target = 1;
}

pub fn local() -> u32 {
    let mut value = 0;
    value = 1;
    value
}

pub fn delegated(target: &mut u32) {
    single_write(target);
    single_write(target);
}

fn clock_leaf() {
    std::thread::yield_now();
}

pub fn clocks() {
    clock_leaf();
    clock_leaf();
}

pub fn unknown() {
    std::hint::black_box(1u32);
}

// rstyle-allow(side_effect_density): Keep this explicit caller-visible write.
pub fn suppressed(target: &mut u32) {
    *target = 1;
}
