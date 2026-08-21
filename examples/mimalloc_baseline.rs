//! Baseline mimalloc allocation workload for local overhead comparisons.
//!
//! ```sh
//! cargo run --release --example mimalloc_baseline --features backend-mimalloc
//! ```

#[path = "mimalloc_benchmark/support.rs"]
mod support;

use support::{print_workload, run_workload, WorkloadConfig};

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    let config = WorkloadConfig::from_env();
    let result = run_workload(config);
    print_workload("mimalloc_baseline", config, result);
}
