#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod tests {
    use notlibc::mmap::{Mmap, MmapMut, page_size};

    #[test]
    fn page_size_is_power_of_two() {
        let ps = page_size();
        assert!(ps > 0);
        assert!(ps.is_power_of_two(), "page_size={ps} is not a power of two");
    }

    #[test]
    fn mmap_mut_write_read() {
        let mut map = MmapMut::map_anon(page_size()).expect("MmapMut::map_anon");
        assert_eq!(map.len(), page_size());

        let pattern: &[u8] = b"hello sig_safety";
        map[..pattern.len()].copy_from_slice(pattern);
        assert_eq!(&map[..pattern.len()], pattern);
    } // <-- Drop unmaps

    #[test]
    fn mmap_mut_drop_unmaps() {
        // Allocate and immediately drop — should not crash or leak.
        let map = MmapMut::map_anon(page_size()).expect("MmapMut::map_anon");
        drop(map);
    }

    #[test]
    fn make_read_only_transition() {
        let mut map = MmapMut::map_anon(page_size()).expect("MmapMut::map_anon");
        map[0] = 42;
        let ro: Mmap = map.make_read_only().expect("make_read_only");
        assert_eq!(ro[0], 42);
        // Deref to &[u8] works.
        assert_eq!(ro.len(), page_size());
    }

    #[test]
    fn make_read_only_then_make_mut() {
        let map = MmapMut::map_anon(page_size()).expect("MmapMut::map_anon");
        let ro = map.make_read_only().expect("make_read_only");
        let mut rw = ro.make_mut().expect("make_mut");
        rw[0] = 0xff;
        assert_eq!(rw[0], 0xff);
    }

    #[test]
    fn make_exec_transition() {
        // We just verify the mprotect succeeds; actually executing the mapping
        // is out of scope for this test.
        let map = MmapMut::map_anon(page_size()).expect("MmapMut::map_anon");
        map.make_exec().expect("make_exec");
    }

    #[test]
    fn zero_len_mapping_is_empty() {
        // memmap2 allows zero-length maps; we follow the same convention.
        let map = MmapMut::map_anon(0).expect("MmapMut::map_anon(0)");
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn multi_page_allocation() {
        let size = page_size() * 4;
        let mut map = MmapMut::map_anon(size).expect("MmapMut::map_anon 4 pages");
        // Write to first and last byte of every page.
        for i in 0..4usize {
            let base = i * page_size();
            map[base] = i as u8;
            map[base + page_size() - 1] = (i + 10) as u8;
        }
        for i in 0..4usize {
            let base = i * page_size();
            assert_eq!(map[base], i as u8);
            assert_eq!(map[base + page_size() - 1], (i + 10) as u8);
        }
    }
}
