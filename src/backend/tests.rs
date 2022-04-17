use std::collections::HashMap;

use super::*;

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
    let mut frames = Vec::new();
    frames.push(StackFrame::new(
        Some("module".to_string()),
        Some("name".to_string()),
        Some("filename".to_string()),
        Some("relative_path".to_string()),
        Some("absolute_path".to_string()),
        Some(1),
    ));
    frames.push(StackFrame::new(
        Some("module".to_string()),
        Some("name".to_string()),
        Some("filename".to_string()),
        Some("relative_path".to_string()),
        Some("absolute_path".to_string()),
        Some(2),
    ));

    let stack_trace = StackTrace::new(None, None, None, frames);

    assert_eq!(
        format!("{}", stack_trace),
        "filename:2 - name;filename:1 - name"
    );
}

#[test]
fn test_report_record() {
    let mut report = Report::new(HashMap::new());

    let stack_trace = StackTrace::new(None, None, None, vec![]);

    assert!(report.record(stack_trace).is_ok());
    assert_eq!(report.data.len(), 1);
}

#[test]
fn test_report_clear() {
    let mut report = Report::new(HashMap::new());

    let stack_trace = StackTrace::new(None, None, None, vec![]);

    assert!(report.record(stack_trace).is_ok());

    report.clear();

    assert_eq!(report.data.len(), 0);
}

#[test]
fn test_report_display() {
    // Dummy StackTrace
    let mut frames = Vec::new();
    frames.push(StackFrame::new(
        Some("module".to_string()),
        Some("name".to_string()),
        Some("filename".to_string()),
        Some("absolute_path".to_string()),
        Some("relative_path".to_string()),
        Some(1),
    ));
    frames.push(StackFrame::new(
        Some("module".to_string()),
        Some("name".to_string()),
        Some("filename".to_string()),
        Some("absolute_path".to_string()),
        Some("relative_path".to_string()),
        Some(2),
    ));
    let stack_trace = StackTrace::new(None, None, None, frames);

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

    ruleset.add_rule(rule);

    assert_eq!(ruleset.rules.lock().unwrap().len(), 1);
}

#[test]
fn test_ruleset_remove_rule() {
    let ruleset = Ruleset::new();

    let add_rule = Rule::ThreadTag(0, Tag::new("key".to_string(), "value".to_string()));

    ruleset.add_rule(add_rule);

    assert_eq!(ruleset.rules.lock().unwrap().len(), 1);

    let remove_rule = Rule::ThreadTag(0, Tag::new("key".to_string(), "value".to_string()));

    ruleset.remove_rule(remove_rule);

    assert_eq!(ruleset.rules.lock().unwrap().len(), 0);
}
