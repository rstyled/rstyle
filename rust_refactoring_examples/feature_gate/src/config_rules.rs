use std::collections::HashMap;

use crate::rules::{Decision, RuleSource};

pub struct ConfigRules {
    rules: HashMap<String, String>,
}

impl ConfigRules {
    pub fn new(rules: HashMap<String, String>) -> Self {
        Self { rules }
    }
}

impl RuleSource for ConfigRules {
    fn name(&self) -> &'static str {
        "config"
    }

    fn decide(&self, flag: &str, context: &HashMap<String, String>) -> Option<Decision> {
        let expression = self.rules.get(flag)?;
        let (key, expected) = expression.split_once('=')?;
        match context.get(key) {
            Some(actual) if actual == expected => Some(Decision::Allow),
            Some(_) => Some(Decision::Deny),
            None => None,
        }
    }
}
