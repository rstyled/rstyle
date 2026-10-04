use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum Decision {
    Allow,
    Deny,
    Pass,
}

pub trait RuleSource {
    fn name(&self) -> &'static str;
    fn decide(&self, flag: &str, context: &HashMap<String, String>) -> Option<Decision>;
}
