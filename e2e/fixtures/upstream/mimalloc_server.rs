use rustfs_mimalloc_profiling::SamplingAllocator;

#[global_allocator]
static ALLOC: SamplingAllocator<mimalloc::MiMalloc> =
    SamplingAllocator::with_allocator(mimalloc::MiMalloc);

#[path = "../mimalloc_server.rs"]
mod fixture;
