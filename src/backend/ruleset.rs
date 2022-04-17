use super::{StackTrace, Tag};
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

impl std::ops::Add<&Ruleset> for StackTrace {
    type Output = Self;
    fn add(self, other: &Ruleset) -> Self {
        // Get global Tags
        let global_tags: Vec<Tag> = other.get_global_tags().unwrap_or(Vec::new());

        // Get Stack Tags
        let mut stack_tags: Vec<Tag> = Vec::new();
        for rule in other.rules.lock().unwrap().iter() {
            match rule {
                Rule::ThreadTag(thread_id, tag) => {
                    if let Some(stack_thread_id) = self.thread_id {
                        if thread_id == &stack_thread_id {
                            stack_tags.push(tag.clone());
                        }
                    }
                }
                _ => {}
            }
        }

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

mod tests {
    use super::*;

    #[test]
    fn test_ruleset() {
        let ruleset = Ruleset::new();

        assert_eq!(ruleset.get_global_tags().unwrap(), Vec::new());

        ruleset
            .add_rule(Rule::GlobalTag(Tag::new(
                "key1".to_string(),
                "value".to_string(),
            )))
            .unwrap();

        ruleset
            .add_rule(Rule::GlobalTag(Tag::new(
                "key2".to_string(),
                "value".to_string(),
            )))
            .unwrap();

        ruleset
            .add_rule(Rule::ThreadTag(
                1,
                Tag::new("key1".to_string(), "value".to_string()),
            ))
            .unwrap();

        ruleset
            .add_rule(Rule::ThreadTag(
                2,
                Tag::new("key1".to_string(), "value".to_string()),
            ))
            .unwrap();

        ruleset
            .add_rule(Rule::ThreadTag(
                3,
                Tag::new("key1".to_string(), "value".to_string()),
            ))
            .unwrap();

        assert_eq!(
            ruleset.get_global_tags().unwrap(),
            vec![
                Tag::new("key1".to_string(), "value".to_string()),
                Tag::new("key2".to_string(), "value".to_string())
            ]
        );

        // Remove ThreadTag number 2
        ruleset
            .remove_rule(Rule::ThreadTag(
                2,
                Tag::new("key1".to_string(), "value".to_string()),
            ))
            .unwrap();

        // Verify ThreadTag number 2 is removed from the ruleset Vector
        assert_eq!(
            ruleset.rules.lock().unwrap().clone(),
            vec![
                Rule::GlobalTag(Tag::new("key1".to_string(), "value".to_string(),)),
                Rule::GlobalTag(Tag::new("key2".to_string(), "value".to_string(),)),
                Rule::ThreadTag(1, Tag::new("key1".to_string(), "value".to_string(),)),
                Rule::ThreadTag(3, Tag::new("key1".to_string(), "value".to_string(),))
            ]
        );
    }

    #[test]
    fn test_ruleset_duplicates() {
        let ruleset = Ruleset::new();

        ruleset
            .add_rule(Rule::GlobalTag(Tag::new(
                "key1".to_string(),
                "value".to_string(),
            )))
            .unwrap();

        ruleset
            .add_rule(Rule::GlobalTag(Tag::new(
                "key1".to_string(),
                "value".to_string(),
            )))
            .unwrap();
        assert_eq!(
            ruleset.get_global_tags().unwrap(),
            vec![Tag::new("key1".to_string(), "value".to_string())]
        );
    }

    #[test]
    fn test_ruleset_remove_nonexistent() {
        let ruleset = Ruleset::new();

        ruleset
            .add_rule(Rule::GlobalTag(Tag::new(
                "key1".to_string(),
                "value".to_string(),
            )))
            .unwrap();

        ruleset
            .remove_rule(Rule::GlobalTag(Tag::new(
                "key2".to_string(),
                "value".to_string(),
            )))
            .unwrap();

        assert_eq!(
            ruleset.get_global_tags().unwrap(),
            vec![Tag::new("key1".to_string(), "value".to_string())]
        );
    }

    #[test]
    fn test_stacktrace_add() {
        // Create a Ruleset
        let ruleset = Ruleset::new();

        // Two global Tags
        ruleset
            .add_rule(Rule::GlobalTag(Tag::new(
                "key1".to_string(),
                "value".to_string(),
            )))
            .unwrap();
        ruleset
            .add_rule(Rule::GlobalTag(Tag::new(
                "key2".to_string(),
                "value".to_string(),
            )))
            .unwrap();
        // One Thread tag with id 55
        ruleset
            .add_rule(Rule::ThreadTag(
                55,
                Tag::new("keyA".to_string(), "valueA".to_string()),
            ))
            .unwrap();
        // One Thread tag with id 100
        ruleset
            .add_rule(Rule::ThreadTag(
                100,
                Tag::new("keyB".to_string(), "valueB".to_string()),
            ))
            .unwrap();

        // Create Stacktrace with id 55
        let stacktrace = StackTrace::new(
            Some(1),
            Some(55),
            Some("thread_name".to_string()),
            vec![crate::backend::StackFrame::new(
                Some("file1".to_string()),
                Some("function1".to_string()),
                Some("file1".to_string()),
                Some("file1".to_string()),
                Some("file1".to_string()),
                Some(1),
            )],
        );

        // assert initial metadata of the stacktrace
        assert_eq!(stacktrace.metadata, crate::backend::Metadata::default());

        // Add the Stacktrace to the Ruleset
        let applied_stacktrace = stacktrace + &ruleset;

        // assert that the metadata of the stacktrace is updated
        assert_eq!(
            applied_stacktrace.metadata,
            crate::backend::Metadata {
                tags: vec![
                    Tag::new("key1".to_string(), "value".to_string()),
                    Tag::new("key2".to_string(), "value".to_string()),
                    Tag::new("keyA".to_string(), "valueA".to_string()),
                ]
            }
        );
    }
}
