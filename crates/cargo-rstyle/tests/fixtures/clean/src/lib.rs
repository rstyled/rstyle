pub struct Meter {
    value: u32,
}

impl Meter {
    pub fn reading(&self) -> u32 {
        self.value
    }

    pub fn bump(&mut self) {
        self.value += 1;
    }
}

pub fn add(a: u32, b: u32) -> u32 {
    let sum = a + b;
    sum * 2
}

pub fn describe(items: &[u32]) -> usize {
    items.iter().filter(|item| **item > 1).count()
}
