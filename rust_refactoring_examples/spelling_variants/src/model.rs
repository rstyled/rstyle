use std::collections::HashMap;

pub trait Sink {
    fn put(&mut self, msg: &str) -> Result<(), String>;
    fn note(&mut self, line: &str);
}

#[derive(Default)]
pub struct Registry {
    pub items: HashMap<String, String>,
    pub fresh_keys: Vec<String>,
    pub log: std::cell::RefCell<Vec<String>>,
}

impl Registry {
    pub fn find(&self, key: &str) -> Option<String> {
        self.items.get(key).cloned()
    }
    pub fn has(&self, key: &str) -> bool {
        self.items.contains_key(key)
    }
    pub fn fresh(&self, key: &str) -> bool {
        self.fresh_keys.iter().any(|k| k == key)
    }
    pub fn read(&self, key: &str) -> String {
        self.items.get(key).cloned().unwrap_or_default()
    }
    pub fn touch(&self, key: &str) {
        self.log.borrow_mut().push(format!("touch {key}"));
    }
    pub fn miss(&self, key: &str) {
        self.log.borrow_mut().push(format!("miss {key}"));
    }
    pub fn note(&self, line: &str) {
        self.log.borrow_mut().push(line.to_string());
    }
}
