//! Constructs rustc lowers before analysis, for `cpg emit` and `cpg compare`.

mod shapes;

use std::collections::BTreeMap;

static mut COUNTER: u32 = 0;

pub struct Store {
    pub count: u32,
    items: Vec<Vec<u32>>,
    names: BTreeMap<String, u32>,
}

impl Store {
    fn bump(&mut self) {
        self.count += 1;
    }

    /// A `for` loop ending another loop's body.
    pub fn nested(&self) -> u32 {
        let mut total = 0;
        for row in &self.items {
            for item in row {
                total += item;
            }
        }
        total
    }

    /// A `while` loop, whose lowering adds a block of its own.
    pub fn countdown(limit: u32) -> u32 {
        let mut total = limit;
        while total > 3 {
            total -= 1;
        }
        total
    }

    /// `while let` and a labeled loop left from inside another.
    pub fn drain(&self, limit: u32) -> u32 {
        let mut pending = vec![1, 2, 3];
        let mut total = 0;
        while let Some(next) = pending.pop() {
            total += next;
        }
        'outer: for row in &self.items {
            for item in row {
                if *item == limit {
                    break 'outer;
                }
            }
        }
        total
    }

    /// `?` ending a block inside a loop.
    pub fn check_all(&self) -> Result<(), String> {
        for row in &self.items {
            for item in row {
                match item {
                    0 => continue,
                    value => {
                        checked(*value)?
                    }
                }
            }
        }
        Ok(())
    }

    /// A macro ending a block inside a loop, and `?` in a closure operand.
    pub fn report(&self, text: &str) -> Result<u32, String> {
        let first: u32 = text.parse().map_err(|error| format!("{error}"))?;
        for (name, value) in &self.names {
            if name.is_empty() {
                println!("empty name for {value}")
            }
        }
        let Some(row) = self.items.first() else {
            return Err("no rows".into());
        };
        Ok(first + row.len() as u32)
    }

    /// Effects: delegated reborrows, unsafe code, a mutable static and value calls.
    pub fn effects(&mut self, callback: fn(u32) -> u32) -> u32 {
        self.bump();
        let local = |value: u32| value + 1;
        unsafe { COUNTER += 1 };
        println!("{}", self.count);
        let shown = (if self.count > 1 { callback(self.count) } else { local(self.count) }) + 1;
        shown
    }
}

fn checked(value: u32) -> Result<(), String> {
    if value > 100 { Err(format!("{value} is too large")) } else { Ok(()) }
}
