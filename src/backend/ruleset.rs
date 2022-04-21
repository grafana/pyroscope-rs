use super::{StackTrace, Tag};
use crate::error::Result;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Profiling Rule
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Rule {
    /// Global Tag
    GlobalTag(Tag),
    /// Thread Tag
    ThreadTag(u64, Tag),
}

/// Ruleset is a set of rules that can be applied to a stacktrace. The rules
/// are held in a Vector behind an Arc, so that they can be shared between
/// threads.
#[derive(Debug, Default, Clone)]
pub struct Ruleset {
    /// Rules vector
    pub rules: Arc<Mutex<HashSet<Rule>>>,
}

impl Ruleset {
    /// Create a new empty ruleset
    pub fn new() -> Self {
        Self {
            rules: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Add a rule to the ruleset
    pub fn add_rule(&self, rule: Rule) -> Result<bool> {
        let rules = self.rules.clone();

        // Add the rule to the Ruleset
        let insert = rules.lock()?.insert(rule);

        Ok(insert)
    }

    /// Remove a rule from the ruleset
    pub fn remove_rule(&self, rule: Rule) -> Result<bool> {
        let rules = self.rules.clone();

        // Remove the rule from the Ruleset
        let remove = rules.lock()?.remove(&rule);

        Ok(remove)
    }

    /// Return a list of all global tags
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

impl std::ops::Add<&Ruleset> for StackTrace {
    type Output = Self;
    fn add(self, other: &Ruleset) -> Self {
        // Get global Tags
        let global_tags: Vec<Tag> = other.get_global_tags().unwrap_or_default();

        // Filter Thread Tags
        let stack_tags: Vec<Tag> = other
            .rules
            .lock()
            .unwrap()
            .iter()
            .filter_map(|rule| {
                if let Rule::ThreadTag(thread_id, tag) = rule {
                    if let Some(stack_thread_id) = self.thread_id {
                        if thread_id == &stack_thread_id {
                            return Some(tag.clone());
                        }
                    }
                }
                None
            })
            .collect();

        // Add tags to metadata
        let mut metadata = self.metadata.clone();
        for tag in global_tags.iter().chain(stack_tags.iter()) {
            metadata.add_tag(tag.clone());
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
