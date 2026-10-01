pub trait Shape {
    fn area(&self) -> u32;
}

pub struct Square(pub u32);

pub struct Counter {
    pub hits: u32,
}

impl Shape for Square {
    fn area(&self) -> u32 {
        side(self.0)
    }
}

fn side(n: u32) -> u32 {
    n * n
}

pub fn total(shapes: &[&dyn Shape]) -> u32 {
    shapes.iter().map(|s| s.area()).sum()
}

pub fn direct(sq: &Square, counter: &mut Counter) -> u32 {
    counter.hits += 1;
    side(sq.0) + sq.area() + Vec::<u32>::new().len() as u32
}
