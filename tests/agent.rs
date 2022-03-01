use pyroscope::pyroscope::PyroscopeConfig;

#[test]
fn test_PyroscopeConfig_new() {
    let config = PyroscopeConfig::new("http://localhost:8080", "myapp");
    assert_eq!(config.url, "http://localhost:8080");
    assert_eq!(config.application_name, "myapp");
    assert_eq!(config.sample_rate, 100u32);
    assert_eq!(config.tags.len(), 0);
}

#[test]
fn test_PyroscopeConfig_sample_rate() {
    let config = PyroscopeConfig::new("http://localhost:8080", "myapp").sample_rate(10);
    assert_eq!(config.sample_rate, 10u32);
}

#[test]
fn test_PyroscopeConfig_tags_empty() {
    let config = PyroscopeConfig::new("http://localhost:8080", "myapp");
    assert_eq!(config.tags.len(), 0);
}

#[test]
fn test_PyroscopeConfig_tags() {
    let config = PyroscopeConfig::new("http://localhost:8080", "myapp").tags(&[("tag", "value")]);
    assert_eq!(config.tags.len(), 1);
    assert_eq!(config.tags.get("tag"), Some(&"value".to_owned()));
}

#[test]
fn test_PyroscopeConfig_tags_multiple() {
    let config = PyroscopeConfig::new("http://localhost:8080", "myapp")
        .tags(&[("tag1", "value1"), ("tag2", "value2")]);
    assert_eq!(config.tags.len(), 2);
    assert_eq!(config.tags.get("tag1"), Some(&"value1".to_owned()));
    assert_eq!(config.tags.get("tag2"), Some(&"value2".to_owned()));
}
