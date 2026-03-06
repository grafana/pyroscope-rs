#[test]
fn error_on_nonexistent_file() {
    let result = coredump::Coredump::open("/tmp/nonexistent-core-file-38493729");
    assert!(result.is_err());
}
