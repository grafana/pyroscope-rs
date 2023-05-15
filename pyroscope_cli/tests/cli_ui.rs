#[test]
fn cli_ui_doc_text_tests() {
    let t = trycmd::TestCases::new();
    let cli = trycmd::cargo::cargo_bin("pyroscope-cli");
    t.register_bin("pyroscope-cli", &cli);
    t.case("tests/cli-ui/*.toml");
}
