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
}

impl StackTrace {
    pub fn add_tag_rules(self, other: &Ruleset) -> Self {
        let mut metadata = self.metadata;

        other
            .rules
            .lock()
            .unwrap()
            .iter()
            .for_each(|rule| match (self.thread_id, rule) {
                (Some(stacktrace_thread_id), Rule::ThreadTag(rule_thread_id, tag)) => {
                    if stacktrace_thread_id == *rule_thread_id {
                        metadata.add_tag(tag.clone());
                    }
                }
                _ => {}
            });

        Self {
            pid: self.pid,
            thread_id: self.thread_id,
            thread_name: self.thread_name,
            frames: self.frames,
            metadata,
        }
    }
}
