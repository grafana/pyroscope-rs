use rustfs_mimalloc_profiling::SamplingAllocator;

#[global_allocator]
static ALLOC: SamplingAllocator<rustfs_mimalloc::MiMalloc> =
    SamplingAllocator::with_allocator(rustfs_mimalloc::MiMalloc);

#[path = "../mimalloc_server.rs"]
mod fixture;
