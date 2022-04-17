use super::Tag;
use crate::error::Result;
use std::sync::{Arc, Mutex};

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Rule {
    GlobalTag(Tag),
    ThreadTag(u64, Tag),
}

#[derive(Debug, Default, Clone)]
pub struct Ruleset {
    pub rules: Arc<Mutex<Vec<Rule>>>,
}

impl Ruleset {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_rule(&self, rule: Rule) -> Result<()> {
        let rules = self.rules.clone();
        rules.lock()?.push(rule);

        Ok(())
    }

    pub fn remove_rule(&self, rule: Rule) -> Result<()> {
        let rules = self.rules.clone();
        rules.lock()?.retain(|r| r != &rule);

        Ok(())
    }

    pub fn get_global_tags(&self) -> Result<Vec<Tag>> {
        let rules = self.rules.clone();
        let rules = rules.lock()?;

        let mut tags = Vec::new();
        for rule in rules.iter() {
            match rule {
                Rule::GlobalTag(tag) => tags.push(tag.to_owned()),
                _ => (),
            }
        }

        Ok(tags)
    }
}
