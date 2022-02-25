use pyroscope::timer::Timer;

#[test]
fn test_timer() {
    // Initialize Timer
    let mut timer = Timer::initialize().unwrap();

    // Attach a listener
    let (tx, rx) = std::sync::mpsc::channel();
    timer.attach_listener(tx).unwrap();

    // Wait for event (should arrive in 10s)
    let recv: u64 = rx.recv().unwrap();

    // Get current time
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Check that recv and now are within 10s of each other
    assert!(recv - now < 10);

    // Check that recv is divisible by 10
    assert!(recv % 10 == 0);
}
