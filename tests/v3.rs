#[cfg(feature = "backend-pprof-rs")]
mod tests {

    use pyroscope::v3experimental::{Agent, PprofConfig};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn next_size(hasher: &mut DefaultHasher) -> usize {
        Instant::now().hash(hasher);
        (hasher.finish() as usize) % (256 * 1024) + 1
    }

    #[test]
    fn test_agent_v3() {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();

        let alloc_thread = std::thread::spawn(move || {
            let mut hasher = DefaultHasher::new();
            while !stop_thread.load(Ordering::Acquire) {
                let size = next_size(&mut hasher);
                let v: Vec<u8> = vec![0u8; size];
                drop(v);
            }
        });

        let profiler_thread = std::thread::spawn(move || {
            let backend = Agent::new(PprofConfig::default());
            let mut backend = match backend {
                Ok(backend) => backend,
                Err(_) => {
                    panic!("err")
                }
            };

            std::thread::sleep(Duration::from_secs(5));

            backend.report().expect("failed to dump report");

            stop.store(true, Ordering::Release);
        });

        alloc_thread.join().expect("alloc thread panicked");
        profiler_thread.join().expect("alloc thread panicked");
    }
}
