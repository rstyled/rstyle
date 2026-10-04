pub struct Dog {
    pub name: String,
}
pub struct Cat {
    pub name: String,
}

impl Dog {
    pub fn new(name: String) -> Self {
        Dog { name }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn bark(&self) -> String {
        format!("{}: woof", self.name)
    }
    pub fn chase(&self, other: &Cat) -> Dog {
        Dog::new(format!("{}>{}", self.name, other.name))
    }
}

impl Cat {
    pub fn new(name: String) -> Self {
        Cat { name }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn meow(&self) -> String {
        format!("{}: meow", self.name)
    }
    pub fn chase(&self, other: &Dog) -> Cat {
        Cat::new(format!("{}>{}", self.name, other.name))
    }
}

pub trait Speak {
    fn speak(&self) -> String;
}
pub struct Cow;
pub struct Hen;
impl Speak for Cow {
    fn speak(&self) -> String {
        "moo".into()
    }
}
impl Speak for Hen {
    fn speak(&self) -> String {
        "cluck".into()
    }
}

pub enum Pet {
    Dog(Dog),
    Cat(Cat),
}

/// Unrelated getters that happen to share `(&Self) -> String`.
pub struct Rock {
    label: String,
}
pub struct Pebble {
    tag: String,
}
impl Rock {
    pub fn label(&self) -> String {
        self.label.clone()
    }
}
impl Pebble {
    pub fn tag(&self) -> String {
        self.tag.clone()
    }
}

pub fn emit_dogs(dogs: &[Dog], out: &mut Vec<String>) {
    for dog in dogs {
        out.push(dog.bark());
    }
}

pub fn emit_cats(cats: &[Cat], out: &mut Vec<String>) {
    for cat in cats {
        out.push(cat.meow());
    }
}

pub fn emit_cats_each(cats: &[Cat], out: &mut Vec<String>) {
    cats.iter().for_each(|cat| out.push(cat.meow()));
}

pub fn pet_sound(pet: &Pet) -> String {
    match pet {
        Pet::Dog(dog) => dog.bark(),
        Pet::Cat(cat) => cat.meow(),
    }
}

/// Behind `Speak` already: the miner must report this as abstracted, not as a candidate.
pub fn emit_cow(cow: &Cow, loud: bool, out: &mut Vec<String>) {
    if loud {
        out.push(cow.speak());
    }
}

pub fn emit_hen(hen: &Hen, loud: bool, out: &mut Vec<String>) {
    if loud {
        out.push(hen.speak());
    }
}

/// Same ADT, different concrete type arguments: still two owners.
pub struct Wrapper<T>(pub T);
impl Wrapper<u8> {
    pub fn width(&self) -> u32 {
        8
    }
}
impl Wrapper<u16> {
    pub fn width(&self) -> u32 {
        16
    }
}
