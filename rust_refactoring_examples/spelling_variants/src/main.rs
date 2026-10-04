mod left;
mod model;
mod right;

use model::{Registry, Sink};

struct Flaky(u32);

impl Sink for Flaky {
    fn put(&mut self, _msg: &str) -> Result<(), String> {
        if self.0 == 0 {
            Ok(())
        } else {
            self.0 -= 1;
            Err("busy".to_string())
        }
    }
    fn note(&mut self, line: &str) {
        println!("{line}");
    }
}

fn main() {
    let mut reg = Registry::default();
    reg.items.insert("a".into(), "1".into());
    reg.fresh_keys.push("a".into());
    println!("{:?}", left::retry_while(&mut Flaky(2), "x"));
    println!("{:?}", right::retry_for(&mut Flaky(2), "x"));
    println!("{}", left::lookup_match(&reg, "a"));
    println!("{}", right::lookup_if_let(&reg, "b"));
    println!("{}", left::react_chain(&reg, "a", 404));
    println!("{}", right::react_match(&reg, "a", 200));
    println!("{:?} {:?}", left::cached_and(&reg, "a"), right::cached_nested(&reg, "z"));
    println!("{:?} {:?}", left::store_try(&reg, &mut Flaky(0), "a"), right::store_match(&reg, &mut Flaky(1), "a"));
    println!("{}", left::flush_all(&mut Flaky(0), &["l".to_string()]));
    println!("{}", right::evict_if_let(&mut reg, "a"));
}
