use super::{StackTrace, Tag};
use crate::error::Result;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ThreadTag {
    tid: crate::utils::ThreadId,
    tag: Tag,
}

impl ThreadTag {
    pub fn new(tid: crate::ThreadId, tag: Tag) -> Self {
        Self { tid, tag }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Ruleset {
    pub rules: Arc<Mutex<HashSet<ThreadTag>>>,
}

impl Ruleset {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn add_rule(&self, rule: ThreadTag) -> Result<bool> {
        let rules = self.rules.clone();

        // Add the rule to the Ruleset
        let insert = rules.lock()?.insert(rule);

        Ok(insert)
    }

    pub fn remove_rule(&self, rule: ThreadTag) -> Result<bool> {
        let rules = self.rules.clone();

        // Remove the rule from the Ruleset
        let remove = rules.lock()?.remove(&rule);

        Ok(remove)
    }

    #[cfg(test)]
    pub fn thread_tags(&self, tid: crate::ThreadId) -> Vec<Tag> {
        let s = StackTrace {
            pid: None,
            thread_id: Some(tid.clone()),
            thread_name: None,
            frames: vec![],
            metadata: Default::default(),
        };
        let tags: Vec<Tag> = s.add_tag_rules(self).metadata.tags.into_iter().collect();
        tags
    }
}

impl StackTrace {
    pub fn add_tag_rules(self, other: &Ruleset) -> Self {
        let mut metadata = self.metadata;

        if let Ok(rules) = other.rules.lock() {
            rules.iter().for_each(|rule| {
                if let Some(stack_thread_id) = &self.thread_id {
                    if rule.tid == *stack_thread_id {
                        metadata.add_tag(rule.tag.clone());
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
