#[test]
fn mutex_new_lock_mutate_release() {
    let m = notlibc::Mutex::new(0u32);
    {
        let mut guard = m.lock();
        *guard = 42;
    }
    let guard = m.lock();
    assert_eq!(*guard, 42);
}

#[test]
fn shard_mutex_alias_usable() {
    let m: notlibc::ShardMutex<u32> = notlibc::ShardMutex::new(0u32);
    {
        let mut guard = m.lock();
        *guard = 7;
    }
    assert_eq!(*m.lock(), 7);
}
