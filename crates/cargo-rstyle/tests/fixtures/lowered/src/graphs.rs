//! Constructs the dependence graph treats specially: collection views, per-item
//! closures, constructors through `Self`, plain loops and literals of every kind.

pub struct Meters(pub f64);

impl Meters {
    pub fn doubled(&self) -> Self {
        Self(self.0 * 2.0)
    }
}

/// `iter()` is a view of `values`, and `for_each` iterates it like a `for`.
pub fn total(values: &[u32]) -> u32 {
    let mut sum = 0;
    values.iter().for_each(|value| sum += value);
    sum
}

/// A function item passed as a value, and a loop that breaks with a value.
pub fn first_even(values: &[u32]) -> Option<u32> {
    let halves: Vec<u32> = values.iter().copied().map(half).collect();
    let mut index = 0;
    loop {
        if index >= halves.len() {
            break None;
        }
        if !halves[index].is_multiple_of(2) {
            index += 1;
            continue;
        }
        break Some(halves[index]);
    }
}

fn half(value: u32) -> u32 {
    value / 2
}

/// Literals of each kind.
pub fn literals() -> (u8, char, f64, bool, &'static [u8], i32) {
    (b'a', 'z', 1.5, true, b"raw", -7)
}
