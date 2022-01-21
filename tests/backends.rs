use pyroscope::backends::{State};

#[test]
fn test_state_default() {
    assert_eq!(State::default(), State::Uninitialized);
}
