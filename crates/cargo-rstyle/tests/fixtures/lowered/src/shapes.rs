//! Expressions, patterns and calls beyond the lowered constructs.

const LIMIT: usize = 4;

#[derive(Clone, Default)]
pub struct Point {
    pub x: i64,
    pub y: i64,
    pub tags: [bool; 2],
}

pub trait Shape {
    fn area(&self) -> i64;
}

impl Shape for Point {
    fn area(&self) -> i64 {
        self.x * self.y
    }
}

/// Assignments, indexing, aggregates, a nested item and a constant block.
pub fn build(values: &mut [i64], origin: &Point) -> Point {
    fn double(value: i64) -> i64 {
        value * 2
    }
    values[0] = double(values[0]);
    let grid = [0u8; LIMIT];
    let pair = (grid.len(), const { 3 });
    let mut point = Point { x: pair.0 as i64, ..origin.clone() };
    point.tags[1] = !point.tags[0];
    point.y = -point.x;
    point
}

/// Guards, wildcards, ranges, or-patterns, slices and reference patterns.
pub fn classify(values: &[i64], flag: bool) -> &'static str {
    if let Some(&head) = values.first()
        && head < -100
    {
        return "deep";
    }
    match values {
        [] => "empty",
        [first, .., last] if *first == *last || flag => "bracketed",
        [1..=9, rest @ ..] if !rest.is_empty() => "digit",
        [value] if *value < 0 => "negative",
        [0 | 1] => "bit",
        _ => "other",
    }
}

/// Arguments from parameters, constants, field chains and other values; dynamic
/// dispatch, a generic bound and a call that never returns.
pub fn dispatch<T: Shape>(shape: &dyn Shape, generic: &T, origin: &Point, count: usize) -> i64 {
    let total = combine(origin.x, LIMIT as i64, count as i64, origin.tags.len() as i64);
    let spread = combine(shape.area(), generic.area(), total, origin.y);
    if spread == i64::MIN {
        unreachable_value();
    }
    'rows: for row in 0..count {
        if row == LIMIT {
            continue 'rows;
        }
    }
    spread
}

fn combine(a: i64, b: i64, c: i64, d: i64) -> i64 {
    a + b + c + d
}

fn unreachable_value() -> ! {
    panic!("unreachable")
}

pub struct Pair(pub i64, pub i64);

/// Destructuring assignments: one statement each, writing their source places.
pub fn destructure(values: [i64; 3], point: &mut Point) -> i64 {
    let (mut a, mut b, mut last) = (0, 0, 0);
    (a, b) = (values[0], values[1]);
    Pair(a, b) = Pair(b, a);
    [.., last] = values;
    Point { x: point.y, tags: [point.tags[1], _], .. } = Point::default();
    _ = last;
    match last {
        0 => _ = a,
        _ => (a, b) = (b, a),
    }
    a + b + last
}
