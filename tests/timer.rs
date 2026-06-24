use assert_matches::assert_matches;
use pyroscope::timer::{Timer, TimerSignal};
use std::time::Duration;

#[test]
fn test_timer() {
    // Initialize Timer with the default 10s interval
    let mut timer = Timer::initialize(Duration::from_secs(10)).unwrap();

    // Attach a listener
    let (tx, rx) = std::sync::mpsc::channel();
    timer.attach_listener(tx).unwrap();

    // Wait for event (should arrive within 10s)
    let planned = rx.recv().unwrap();
    assert_matches!(planned, TimerSignal::NextSnapshot(planned) => {
        // Get current time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check that recv and now are within 10s of each other
        assert!(planned.abs_diff(now) < 10);

        // Check that recv is aligned to a 10s bucket boundary
        assert!(planned % 10 == 0);
    })
}
