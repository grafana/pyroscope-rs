use super::{StackTrace, Tag};
use crate::error::Result;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Rule {
    GlobalTag(Tag),
    ThreadTag(crate::utils::ThreadId, Tag),
}

#[derive(Debug, Default, Clone)]
pub struct Ruleset {
    pub rules: Arc<Mutex<HashSet<Rule>>>,
}

impl Ruleset {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn add_rule(&self, rule: Rule) -> Result<bool> {
        let rules = self.rules.clone();

        // Add the rule to the Ruleset
        let insert = rules.lock()?.insert(rule);

        Ok(insert)
    }

    pub fn remove_rule(&self, rule: Rule) -> Result<bool> {
        let rules = self.rules.clone();

        // Remove the rule from the Ruleset
        let remove = rules.lock()?.remove(&rule);

        Ok(remove)
    }

    pub fn get_global_tags(&self) -> Result<Vec<Tag>> {
        let rules = self.rules.clone();

        let tags = rules
            .lock()?
            .iter()
            .filter_map(|rule| match rule {
                Rule::GlobalTag(tag) => Some(tag.to_owned()),
                _ => None,
            })
            .collect();

        Ok(tags)
    }
}

impl StackTrace {
    pub fn add_tag_rules(self, other: &Ruleset) -> Self {
        let mut metadata = self.metadata;

        if let Ok(global_tags) = other.get_global_tags() {
            for tag in global_tags {
                metadata.add_tag(tag);
            }
        };

        if let Ok(rules) =  other.rules.lock() {
            rules.iter().for_each(|rule| {
                if let Rule::ThreadTag(thread_id, tag) = rule {
                    if let Some(stack_thread_id) = &self.thread_id {
                        if thread_id == stack_thread_id {
                            metadata.add_tag(tag.clone());
                        }
                    }
                }
            })
        }

        Self {
            pid: self.pid,
            thread_id: self.thread_id,
            thread_name: self.thread_name,
            frames: self.frames,
            metadata,
        }
    }
}
