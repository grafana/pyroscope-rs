use std::{
    env,
    time::{Duration, Instant},
};

#[derive(Debug, Copy, Clone)]
pub struct WorkloadConfig {
    pub duration: Duration,
    pub batch_size: usize,
    pub min_size: usize,
    pub max_size: usize,
    pub size_step: usize,
    pub latency_sample_interval: u64,
    pub latency_sample_limit: usize,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct LatencyPercentiles {
    pub p50_nanos: u128,
    pub p95_nanos: u128,
    pub p99_nanos: u128,
}

#[derive(Debug, Copy, Clone)]
pub struct WorkloadResult {
    pub elapsed: Duration,
    pub allocations: u64,
    pub bytes: u64,
    pub latency_percentiles: Option<LatencyPercentiles>,
}

impl WorkloadConfig {
    pub fn from_env() -> Self {
        Self {
            duration: Duration::from_millis(read_env_u64("MIMALLOC_BENCH_DURATION_MS", 3_000)),
            batch_size: read_env_usize("MIMALLOC_BENCH_BATCH_SIZE", 1024).max(1),
            min_size: read_env_usize("MIMALLOC_BENCH_MIN_SIZE", 64).max(1),
            max_size: read_env_usize("MIMALLOC_BENCH_MAX_SIZE", 64 * 1024).max(1),
            size_step: read_env_usize("MIMALLOC_BENCH_SIZE_STEP", 64).max(1),
            latency_sample_interval: read_env_u64("MIMALLOC_BENCH_LATENCY_SAMPLE_INTERVAL", 1024),
            latency_sample_limit: read_env_usize("MIMALLOC_BENCH_LATENCY_SAMPLE_LIMIT", 4096),
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
    let start = Instant::now();
    let mut allocations = 0_u64;
    let mut bytes = 0_u64;
    let mut latency_samples = Vec::with_capacity(
        config
            .latency_sample_limit
            .min(config.duration.as_millis() as usize),
    );

    while start.elapsed() < config.duration {
        for _ in 0..config.batch_size {
            let size = config.next_size(allocations);
            if should_sample_latency(config, allocations, latency_samples.len()) {
                let allocation_start = Instant::now();
                let allocation = vec![0_u8; size];
                std::hint::black_box(&allocation);
                latency_samples.push(allocation_start.elapsed().as_nanos());
            } else {
                let allocation = vec![0_u8; size];
                std::hint::black_box(&allocation);
            }
            allocations = allocations.saturating_add(1);
            bytes = bytes.saturating_add(size as u64);
        }
    }

    WorkloadResult {
        elapsed: start.elapsed(),
        allocations,
        bytes,
        latency_percentiles: calculate_latency_percentiles(latency_samples),
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
    println!("latency_sample_interval={}", config.latency_sample_interval);
    println!("latency_sample_limit={}", config.latency_sample_limit);
    println!("elapsed_ms={}", result.elapsed.as_millis());
    println!("allocations={}", result.allocations);
    println!("bytes={}", result.bytes);
    println!("allocations_per_sec={allocations_per_sec:.2}");
    println!("mib_per_sec={mib_per_sec:.2}");
    if let Some(percentiles) = result.latency_percentiles {
        println!("allocation_latency_p50_ns={}", percentiles.p50_nanos);
        println!("allocation_latency_p95_ns={}", percentiles.p95_nanos);
        println!("allocation_latency_p99_ns={}", percentiles.p99_nanos);
    }
}

fn should_sample_latency(
    config: WorkloadConfig,
    allocation_index: u64,
    collected_samples: usize,
) -> bool {
    config.latency_sample_interval > 0
        && config.latency_sample_limit > collected_samples
        && allocation_index % config.latency_sample_interval == 0
}

fn calculate_latency_percentiles(mut samples: Vec<u128>) -> Option<LatencyPercentiles> {
    if samples.is_empty() {
        return None;
    }

    samples.sort_unstable();
    Some(LatencyPercentiles {
        p50_nanos: percentile(&samples, 50),
        p95_nanos: percentile(&samples, 95),
        p99_nanos: percentile(&samples, 99),
    })
}

fn percentile(sorted_samples: &[u128], percentile: usize) -> u128 {
    let index = sorted_samples
        .len()
        .saturating_mul(percentile)
        .saturating_add(99)
        .checked_div(100)
        .unwrap_or_default()
        .saturating_sub(1)
        .min(sorted_samples.len() - 1);
    sorted_samples[index]
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
