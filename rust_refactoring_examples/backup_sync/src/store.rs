use std::collections::BTreeMap;

pub trait ObjectStore {
    fn put(&mut self, key: &str, bytes: &[u8]) -> Result<(), String>;
    fn exists(&self, key: &str) -> bool;
    fn list(&self) -> Vec<String>;
}

#[derive(Default)]
pub struct MemoryStore {
    objects: BTreeMap<String, Vec<u8>>,
}

impl ObjectStore for MemoryStore {
    fn put(&mut self, key: &str, bytes: &[u8]) -> Result<(), String> {
        self.objects.insert(key.to_string(), bytes.to_vec());
        Ok(())
    }

    fn exists(&self, key: &str) -> bool {
        self.objects.contains_key(key)
    }

    fn list(&self) -> Vec<String> {
        self.objects.keys().cloned().collect()
    }
}
