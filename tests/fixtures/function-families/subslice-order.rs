//! Independent Rust execution/MIR oracle for the pending subslice contract.
//! Run with the pinned workspace rustc; these are semantic observations,
//! not a claim that these functions currently admit through rstyle.
#![allow(unused_assignments)]

fn binding_snapshot() {
    let first = [11u8, 12];
    let second = [21u8, 22];
    let mut selected: &[u8] = &first;
    let view = &selected[{ selected = &second; 0 }..1];
    assert_eq!(view, &[11]);
    assert_eq!(selected, &[21, 22]);
}

fn field_snapshot() {
    struct Holder<'a> { bytes: &'a [u8] }
    let first = [11u8, 12];
    let second = [21u8, 22];
    let mut holder = Holder { bytes: &first };
    let view = &holder.bytes[{ holder = Holder { bytes: &second }; 0 }..1];
    assert_eq!(view, &[11]);
    assert_eq!(holder.bytes, &[21, 22]);
}

fn computed_snapshot_and_endpoint_order() {
    let first = [11u8, 12];
    let second = [21u8, 22];
    let mut selected: &[u8] = &first;
    let mut order = 0;
    let view = &({ order = order * 10 + 1; selected })[
        { order = order * 10 + 2; selected = &second; 0 }
        ..{ order = order * 10 + 3; 1 }
    ];
    assert_eq!(view, &[11]);
    assert_eq!(selected, &[21, 22]);
    assert_eq!(order, 123);
}

fn main() {
    binding_snapshot();
    field_snapshot();
    computed_snapshot_and_endpoint_order();
}
