#[cfg(test)]
use crate::backend::{
    BackendConfig, Report, Rule, Ruleset, StackBuffer, StackFrame, StackTrace, Tag,
};
#[cfg(test)]
use std::collections::{HashMap, HashSet};

#[test]
fn test_stack_frame_display() {
    let frame = StackFrame::new(
        Some("module".to_string()),
        Some("name".to_string()),
        Some("filename".to_string()),
        Some("relative_path".to_string()),
        Some("absolute_path".to_string()),
        Some(1),
    );

    assert_eq!(format!("{}", frame), "filename:1 - name");
}

#[test]
fn test_stack_trace_display() {
    let frames = vec![
        StackFrame::new(
            Some("module".to_string()),
            Some("name".to_string()),
            Some("filename".to_string()),
            Some("relative_path".to_string()),
            Some("absolute_path".to_string()),
            Some(1),
        ),
        StackFrame::new(
            Some("module".to_string()),
            Some("name".to_string()),
            Some("filename".to_string()),
            Some("relative_path".to_string()),
            Some("absolute_path".to_string()),
            Some(2),
        ),
    ];

    let stack_trace = StackTrace::new(&BackendConfig::default(), None, None, None, frames);

    assert_eq!(
        format!("{}", stack_trace),
        "filename:2 - name;filename:1 - name"
    );
}

#[test]
fn test_report_record() {
    let mut report = Report::new(HashMap::new());

    let stack_trace = StackTrace::new(&BackendConfig::default(), None, None, None, vec![]);

    assert!(report.record(stack_trace).is_ok());
    assert_eq!(report.data.len(), 1);
}

#[test]
fn test_report_clear() {
    let mut report = Report::new(HashMap::new());

    let stack_trace = StackTrace::new(&BackendConfig::default(), None, None, None, vec![]);

    assert!(report.record(stack_trace).is_ok());

    report.clear();

    assert_eq!(report.data.len(), 0);
}

#[test]
fn test_report_display() {
    // Dummy StackTrace
    let frames = vec![
        StackFrame::new(
            Some("module".to_string()),
            Some("name".to_string()),
            Some("filename".to_string()),
            Some("absolute_path".to_string()),
            Some("relative_path".to_string()),
            Some(1),
        ),
        StackFrame::new(
            Some("module".to_string()),
            Some("name".to_string()),
            Some("filename".to_string()),
            Some("absolute_path".to_string()),
            Some("relative_path".to_string()),
            Some(2),
        ),
    ];

    let stack_trace = StackTrace::new(&BackendConfig::default(), None, None, None, frames);

    let mut report = Report::new(HashMap::new());

    report.record(stack_trace.clone()).unwrap();
    report.record(stack_trace).unwrap();

    assert_eq!(
        format!("{}", report),
        "filename:2 - name;filename:1 - name 2"
    );
}

#[test]
fn test_tag_new() {
    let tag = Tag::new("key".to_string(), "value".to_string());

    assert_eq!(tag.key, "key");
    assert_eq!(tag.value, "value");
}

#[test]
fn test_rule_new() {
    let rule = Rule::ThreadTag(0, Tag::new("key".to_string(), "value".to_string()));

    assert_eq!(
        rule,
        Rule::ThreadTag(0, Tag::new("key".to_string(), "value".to_string()))
    );
}

#[test]
fn test_ruleset_new() {
    let ruleset = Ruleset::new();

    assert_eq!(ruleset.rules.lock().unwrap().len(), 0);
}

#[test]
fn test_ruleset_add_rule() {
    let ruleset = Ruleset::new();

    let rule = Rule::ThreadTag(0, Tag::new("key".to_string(), "value".to_string()));

    ruleset.add_rule(rule).unwrap();

    assert_eq!(ruleset.rules.lock().unwrap().len(), 1);
}

#[test]
fn test_ruleset_remove_rule() {
    let ruleset = Ruleset::new();

    let add_rule = Rule::ThreadTag(0, Tag::new("key".to_string(), "value".to_string()));

    ruleset.add_rule(add_rule).unwrap();

    assert_eq!(ruleset.rules.lock().unwrap().len(), 1);

    let remove_rule = Rule::ThreadTag(0, Tag::new("key".to_string(), "value".to_string()));

    ruleset.remove_rule(remove_rule).unwrap();

    assert_eq!(ruleset.rules.lock().unwrap().len(), 0);
}

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
        HashSet::from([
            Rule::GlobalTag(Tag::new("key1".to_string(), "value".to_string(),)),
            Rule::GlobalTag(Tag::new("key2".to_string(), "value".to_string(),)),
            Rule::ThreadTag(1, Tag::new("key1".to_string(), "value".to_string(),)),
            Rule::ThreadTag(3, Tag::new("key1".to_string(), "value".to_string(),))
        ])
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

    let mut backend_config = BackendConfig::default();
    backend_config.report_pid = true;
    backend_config.report_thread_id = true;
    backend_config.report_thread_name = true;

    // Create Stacktrace with id 55
    let stacktrace = StackTrace::new(
        &backend_config,
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
    let mut initial_metadata = crate::backend::Metadata::default();
    initial_metadata.add_tag(Tag::new("pid".to_string(), "1".to_string()));
    initial_metadata.add_tag(Tag::new("thread_id".to_string(), "55".to_string()));
    initial_metadata.add_tag(Tag::new(
        "thread_name".to_string(),
        "thread_name".to_string(),
    ));

    assert_eq!(stacktrace.metadata, initial_metadata);

    // Add the Stacktrace to the Ruleset
    let applied_stacktrace = stacktrace + &ruleset;

    initial_metadata.add_tag(Tag::new("key1".to_string(), "value".to_string()));
    initial_metadata.add_tag(Tag::new("key2".to_string(), "value".to_string()));
    initial_metadata.add_tag(Tag::new("keyA".to_string(), "valueA".to_string()));

    // assert that the metadata of the stacktrace is updated
    assert_eq!(applied_stacktrace.metadata, initial_metadata);

    // Re-apply the Ruleset
    let re_applied_stacktrace = applied_stacktrace + &ruleset;

    // assert that the metadata of the stacktrace is the same
    assert_eq!(re_applied_stacktrace.metadata, initial_metadata);
}

#[test]
fn test_stackbuffer_record() {
    let mut buffer = StackBuffer::new(HashMap::new());
    let stack_trace = StackTrace::new(
        &BackendConfig::default(),
        None,
        None,
        None,
        vec![StackFrame::new(
            None,
            Some("test_record".to_string()),
            None,
            None,
            None,
            None,
        )],
    );
    // First record
    buffer.record(stack_trace.clone()).unwrap();
    assert_eq!(buffer.data.len(), 1);
    assert_eq!(buffer.data[&stack_trace], 1);

    // Second record
    buffer.record(stack_trace.clone()).unwrap();
    assert_eq!(buffer.data.len(), 1);
    assert_eq!(buffer.data[&stack_trace], 2);
}

#[test]
fn test_stackbuffer_record_with_count() {
    let mut buffer = StackBuffer::new(HashMap::new());
    let stack_trace = StackTrace::new(
        &BackendConfig::default(),
        None,
        None,
        None,
        vec![StackFrame::new(
            None,
            Some("test_record".to_string()),
            None,
            None,
            None,
            None,
        )],
    );
    // First record
    buffer.record_with_count(stack_trace.clone(), 1).unwrap();
    assert_eq!(buffer.data.len(), 1);
    assert_eq!(buffer.data[&stack_trace], 1);

    // Second record
    buffer.record_with_count(stack_trace.clone(), 2).unwrap();
    assert_eq!(buffer.data.len(), 1);
    assert_eq!(buffer.data[&stack_trace], 3);
}

#[test]
fn test_stackbuffer_clear() {
    let mut buffer = StackBuffer::new(HashMap::new());
    let stack_trace = StackTrace::new(
        &BackendConfig::default(),
        None,
        None,
        None,
        vec![StackFrame::new(
            None,
            Some("test_record".to_string()),
            None,
            None,
            None,
            None,
        )],
    );
    // First record
    buffer.record(stack_trace.clone()).unwrap();
    assert_eq!(buffer.data.len(), 1);
    assert_eq!(buffer.data[&stack_trace], 1);

    // Second record
    buffer.record(stack_trace.clone()).unwrap();
    assert_eq!(buffer.data.len(), 1);
    assert_eq!(buffer.data[&stack_trace], 2);

    // Clear
    buffer.clear();
    assert_eq!(buffer.data.len(), 0);
}
