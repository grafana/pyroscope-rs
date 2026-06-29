#[cfg(feature = "backend-mimalloc")]
mod tests {
    use prost::Message;
    use pyroscope::backend::mimalloc::{mimalloc_backend, MimallocConfig, SamplingMiMalloc};
    use pyroscope::backend::ReportData;
    use pyroscope::encode::gen::google::Profile;

    #[global_allocator]
    static ALLOC: SamplingMiMalloc = SamplingMiMalloc::new();

    #[test]
    fn mimalloc_backend_reports_raw_memory_pprof() {
        let mut backend = mimalloc_backend(MimallocConfig {
            sample_interval_bytes: 1024,
            ..MimallocConfig::default()
        })
        .initialize()
        .expect("initialize mimalloc backend");

        let allocations: Vec<Vec<u8>> = (0..4096).map(|_| vec![0_u8; 1024]).collect();
        std::hint::black_box(&allocations);

        let batch = backend.report().expect("report memory profile");
        assert_eq!(batch.profile_type, "memory");

        let ReportData::RawPprof(bytes) = batch.data else {
            panic!("expected raw pprof memory profile");
        };
        let profile = Profile::decode(bytes.as_slice()).expect("decode memory pprof");
        assert!(profile.string_table.iter().any(|s| s == "alloc_space"));
    }
}
