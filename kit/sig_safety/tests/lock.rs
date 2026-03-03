#[test]
fn mutex_new_lock_mutate_release() {
    let m = sig_safety::Mutex::new(0u32);
    {
        let mut guard = m.lock();
        *guard = 42;
    }
    let guard = m.lock();
    assert_eq!(*guard, 42);
}

#[test]
fn shard_mutex_alias_usable() {
    let m: sig_safety::ShardMutex<u32> = sig_safety::ShardMutex::new(0u32);
    {
        let mut guard = m.lock();
        *guard = 7;
    }
    assert_eq!(*m.lock(), 7);
}
