#[cfg(all(
    feature = "backend-pprof-rs",
    any(target_arch = "aarch64", target_arch = "x86_64")
))]
mod tests {
    use pyroscope::backend::{pprof_backend, BackendConfig, PprofConfig, ReportData};
    use std::time::{Duration, Instant};

    #[inline(never)]
    fn busy_leaf(mut n: u64) {
        unsafe {
            #[cfg(target_arch = "aarch64")]
            std::arch::asm!("2:", "subs {n}, {n}, #1", "b.ne 2b", n = inout(reg) n, options(nomem, nostack));
            #[cfg(target_arch = "x86_64")]
            std::arch::asm!("2:", "dec {n}", "jnz 2b", n = inout(reg) n, options(nomem, nostack));
        }
        std::hint::black_box(n);
    }

    #[inline(never)]
    fn busy_caller(duration: Duration) {
        let start = Instant::now();
        while start.elapsed() < duration {
            busy_leaf(10_000_000);
        }
    }

    #[test]
    fn test_interrupted_leaf_frame_is_on_top() {
        let mut backend =
            pprof_backend(PprofConfig { sample_rate: 1000 }, BackendConfig::default())
                .initialize()
                .expect("failed to initialize pprof backend");

        busy_caller(Duration::from_secs(2));

        let ReportData::Reports(reports) = backend.report().unwrap().data else {
            panic!("expected stack trace reports");
        };

        let (mut total, mut leaf_on_top) = (0, 0);
        for (stack, count) in reports.iter().flat_map(|r| r.iter()) {
            let names: Vec<&str> = stack
                .frames
                .iter()
                .filter_map(|f| f.name.as_deref())
                .collect();
            if !names.iter().any(|name| name.ends_with("busy_caller")) {
                continue;
            }
            total += count;
            if names
                .first()
                .is_some_and(|name| name.ends_with("busy_leaf"))
            {
                leaf_on_top += count;
            }
        }

        assert!(total > 100, "too few samples: {total}");
        assert!(
            leaf_on_top * 10 >= total * 9,
            "busy_leaf on top in {leaf_on_top} of {total} samples"
        );
    }
}
