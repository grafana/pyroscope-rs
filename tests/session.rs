use pyroscope::{
    pyroscope::PyroscopeConfig,
    session::{Session, SessionManager, SessionSignal},
};
use std::{collections::HashMap, time::Duration};

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
        url: "http://localhost:8080".to_string(),
        application_name: "test".to_string(),
        tags: HashMap::new(),
        sample_rate: 100,
    };

    let report = vec![1, 2, 3];

    let session = Session::new(1950, config, report).unwrap();

    assert_eq!(session.from, 1940);
    assert_eq!(session.until, 1950);
}

#[test]
fn test_session_send_error() {
    let config = PyroscopeConfig {
        url: "http://invalid_url".to_string(),
        application_name: "test".to_string(),
        tags: HashMap::new(),
        sample_rate: 100,
    };

    let report = vec![1, 2, 3];

    let _session = Session::new(1950, config, report).unwrap();

    // TODO: to figure this out
}
