pub struct Dog;
pub struct Cat;
impl Dog {
    pub fn bark(&self) -> String {
        "woof".into()
    }
}
impl Cat {
    pub fn meow(&self) -> String {
        "meow".into()
    }
}

pub fn tag_dog(dog: &Dog, out: &mut Vec<String>) {
    out.push(dog.bark());
    out.push("alpha".to_string());
    out.push(dog.bark());
}

pub fn tag_cat(cat: &Cat, out: &mut Vec<String>) {
    out.push(cat.meow());
    out.push("beta".to_string());
    out.push(cat.meow());
}

pub fn fmt_dog(dog: &Dog, out: &mut Vec<String>) {
    out.push(dog.bark());
    out.push(format!("a long format string prefix {}", dog.bark()));
    out.push(dog.bark());
}

pub fn fmt_cat(cat: &Cat, out: &mut Vec<String>) {
    out.push(cat.meow());
    out.push(format!("another long format string {}", cat.meow()));
    out.push(cat.meow());
}
