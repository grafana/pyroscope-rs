use std::{env, time::Duration};

#[derive(Debug, Copy, Clone)]
pub struct WorkloadConfig {
    pub duration: Duration,
    pub batch_size: usize,
    pub min_size: usize,
    pub max_size: usize,
    pub size_step: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct WorkloadResult {
    pub elapsed: Duration,
    pub allocations: u64,
    pub bytes: u64,
}

impl WorkloadConfig {
    pub fn from_env() -> Self {
        Self {
            duration: Duration::from_millis(read_env_u64("MIMALLOC_BENCH_DURATION_MS", 3_000)),
            batch_size: read_env_usize("MIMALLOC_BENCH_BATCH_SIZE", 1024).max(1),
            min_size: read_env_usize("MIMALLOC_BENCH_MIN_SIZE", 64).max(1),
            max_size: read_env_usize("MIMALLOC_BENCH_MAX_SIZE", 64 * 1024).max(1),
            size_step: read_env_usize("MIMALLOC_BENCH_SIZE_STEP", 64).max(1),
        }
    }

    fn next_size(&self, allocation_index: u64) -> usize {
        let span = self.max_size.saturating_sub(self.min_size);
        if span == 0 {
            return self.min_size;
        }

        let slots = span / self.size_step + 1;
        self.min_size + (allocation_index as usize % slots) * self.size_step
    }
}

pub fn run_workload(config: WorkloadConfig) -> WorkloadResult {
    let start = std::time::Instant::now();
    let mut allocations = 0_u64;
    let mut bytes = 0_u64;

    while start.elapsed() < config.duration {
        for _ in 0..config.batch_size {
            let size = config.next_size(allocations);
            let allocation = vec![0_u8; size];
            std::hint::black_box(&allocation);
            allocations = allocations.saturating_add(1);
            bytes = bytes.saturating_add(size as u64);
        }
    }

    WorkloadResult {
        elapsed: start.elapsed(),
        allocations,
        bytes,
    }
}

pub fn print_workload(label: &str, config: WorkloadConfig, result: WorkloadResult) {
    let elapsed_secs = result.elapsed.as_secs_f64();
    let allocations_per_sec = result.allocations as f64 / elapsed_secs;
    let mib_per_sec = result.bytes as f64 / elapsed_secs / 1024.0 / 1024.0;

    println!("label={label}");
    println!("duration_ms={}", config.duration.as_millis());
    println!("batch_size={}", config.batch_size);
    println!("min_size={}", config.min_size);
    println!("max_size={}", config.max_size);
    println!("size_step={}", config.size_step);
    println!("elapsed_ms={}", result.elapsed.as_millis());
    println!("allocations={}", result.allocations);
    println!("bytes={}", result.bytes);
    println!("allocations_per_sec={allocations_per_sec:.2}");
    println!("mib_per_sec={mib_per_sec:.2}");
}

fn read_env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn read_env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
