use pyroscope::{
    backend::Report,
    pyroscope::PyroscopeConfig,
    session::{Session, SessionManager, SessionSignal},
};
use std::collections::HashMap;
use pyroscope::backend::{StackFrame, StackTrace};
use pyroscope::pyroscope::ReportEncoding;

#[test]
fn test_session_manager_new() {
    let session_manager = SessionManager::new().unwrap();
    assert!(session_manager.handle.is_some());
}

#[test]
fn test_session_manager_push_kill() {
    let session_manager = SessionManager::new().unwrap();
    session_manager.push(SessionSignal::Kill).unwrap();
    assert_eq!(session_manager.handle.unwrap().join().unwrap().unwrap(), ());
}

#[test]
fn test_session_new() {
    let config = PyroscopeConfig {
        url: "http://localhost:4040".to_string(),
        application_name: "test".to_string(),
        tags: HashMap::new(),
        sample_rate: 100u32,
        spy_name: "test-rs".to_string(),
        // report_encoding: ReportEncoding::PPROF,
        report_encoding: ReportEncoding::FOLDED,
        ..Default::default()
    };
    let f1 = StackFrame {
        module: None,
        name: Some("f1".to_string()),

        filename: None,
        relative_path: None,
        absolute_path: None,
        line: None,
    };
    let f2 = StackFrame {
        module: None,
        name: Some("f2".to_string()),
        filename: None,
        relative_path: None,
        absolute_path: None,
        line: None,
    };

    let s1 = StackTrace{
        pid: None,
        thread_id: None,
        thread_name: None,
        frames: vec![f1.clone(), f2.clone()],
        metadata: Default::default()
    };
    let s2 = StackTrace{
        pid: None,
        thread_id: None,
        thread_name: None,
        frames: vec![f2.clone(), f1.clone()],
        metadata: Default::default()
    };


    let report = vec![Report::new(HashMap::from([(s1, 2), (s2, 4)]))];

    let session = Session::new(1661783070, config, report).unwrap();

    session.send().unwrap()
}

#[test]
fn test_session_send_error() {
    let config = PyroscopeConfig {
        url: "http://invalid_url".to_string(),
        application_name: "test".to_string(),
        tags: HashMap::new(),
        sample_rate: 100u32,
        spy_name: "test-rs".to_string(),
        ..Default::default()
    };

    let report = vec![Report::new(HashMap::new())];

    let _session = Session::new(1950, config, report).unwrap();
}
