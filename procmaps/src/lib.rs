// this is a copypaste of linux mod from proc-maps 0.4.0
use libc;
use std;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub type Pid = libc::pid_t;

#[derive(Debug, Clone, PartialEq)]
pub struct Flags {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub private: bool,
}

impl Flags {
    fn from_str(s: &str) -> Self {
        Self {
            read: s.contains("r"),
            write: s.contains("w"),
            execute: s.contains("x"),
            private: s.contains("p"),
        }
    }
}

impl Default for Flags {
    fn default() -> Self {
        Self {
            read: false,
            write: false,
            execute: false,
            private: false,
        }
    }
}

/// A struct representing a single virtual memory region.
///
/// While this structure is only for Linux, the macOS, Windows, and FreeBSD
/// variants have identical exposed methods
#[derive(Debug, Clone, PartialEq)]
pub struct MapRange {
    pub range_start: usize,
    pub range_end: usize,
    pub offset: usize,
    pub dev: String, // todo make it not string
    pub flags: Flags,
    pub inode: usize,
    pub pathname: Option<PathBuf>,
}

impl MapRange {
    pub fn filename(&self) -> Option<(&PathBuf, &str)> {
        let path = if let Some(path) = &self.pathname {
            path
        } else {
            return None;
        };
        if let Some(name) = path.file_name() {
            if let Some(filename) = name.to_str() {
                Some((path, filename))
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn executable(&self) -> bool {
        self.flags.execute
    }
}

//todo we don't need all Maps, rewrite with iterators, lazy, do not read the whole maps file in memory
pub fn get_process_maps(/*pid: Pid*/) -> std::io::Result<Vec<MapRange>> {
    // Parses /proc/PID/maps into a Vec<MapRange>
    // let maps_file = format!("/proc/{}/maps", pid);
    let maps_file = "/proc/self/maps";
    let mut file = File::open(maps_file)?;

    // Check that the file is not too big
    let metadata = file.metadata()?;
    if metadata.len() > 0x10000000 {
        return Err(std::io::Error::from_raw_os_error(libc::EFBIG));
    }

    let mut contents = String::with_capacity(metadata.len() as usize);
    file.read_to_string(&mut contents)?;
    parse_proc_maps(&contents)
}

fn parse_proc_maps(contents: &str) -> std::io::Result<Vec<MapRange>> {
    let mut vec: Vec<MapRange> = Vec::new();
    for line in contents.split("\n") {
        let mut split = line.split_whitespace();
        let range = match split.next() {
            None => break,
            Some(s) => s,
        };

        let mut range_split = range.split("-");
        let range_start = match range_split.next() {
            None => return Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
            Some(s) => match usize::from_str_radix(s, 16) {
                Err(_) => return Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
                Ok(i) => i,
            },
        };
        let range_end = match range_split.next() {
            None => return Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
            Some(s) => match usize::from_str_radix(s, 16) {
                Err(_) => return Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
                Ok(i) => i,
            },
        };
        if range_split.next().is_some() || range_start >= range_end {
            return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
        }

        let flags = match split.next() {
            None => return Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
            Some(s) if s.len() < 3 => return Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
            Some(s) => Flags::from_str(s),
        };
        let offset = match split.next() {
            None => return Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
            Some(s) => match usize::from_str_radix(s, 16) {
                Err(_) => return Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
                // mmap: offset must be a multiple of the page size as returned by sysconf(_SC_PAGE_SIZE).
                Ok(i) if i & 0xfff != 0 => {
                    return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
                }
                Ok(i) => i,
            },
        };
        let dev = match split.next() {
            None => return Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
            Some(s) => s.to_string(),
        };
        let inode = match split.next() {
            None => return Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
            Some(s) => match usize::from_str_radix(s, 10) {
                Err(_) => return Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
                Ok(i) => i,
            },
        };
        let pathname = match Some(split.collect::<Vec<&str>>().join(" ")).filter(|x| !x.is_empty())
        {
            Some(s) => Some(PathBuf::from(s)),
            None => None,
        };


        vec.push(MapRange {
            range_start,
            range_end,
            offset,
            dev,
            flags,
            inode,
            pathname,
        });
    }
    Ok(vec)
}

fn map_contain_addr(map: &MapRange, addr: usize) -> bool {
    let start = map.range_start;
    (addr >= start) && (addr < (start + (map.range_end - map.range_start)))
}

/// Returns whether or not any MapRange contains the given address
/// Note: this will only work correctly on macOS and Linux.
pub fn maps_contain_addr(addr: usize, maps: &[MapRange]) -> bool {
    maps.iter().any(|map| map_contain_addr(map, addr))
}

/// Returns whether or not any MapRange contains the given address range.
/// Note: this will only work correctly on macOS and Linux.
pub fn maps_contain_addr_range(mut addr: usize, mut size: usize, maps: &[MapRange]) -> bool {
    if size == 0 || addr.checked_add(size).is_none() {
        return false;
    }

    while size > 0 {
        match maps.iter().find(|map| map_contain_addr(map, addr)) {
            None => return false,
            Some(map) => {
                let end = map.range_end;
                if addr + size <= end {
                    return true;
                } else {
                    size -= end - addr;
                    addr = end;
                }
            }
        }
    }

    true
}
#[test]
fn test_parse_maps() {
    let contents = "00400000-00507000 r-xp 00000000 00:14 205736                             /usr/bin/fish
00708000-0070a000 rw-p 00000000 00:00 0
0178c000-01849000 rw-p 00000000 00:00 0                                  [heap]
7f438050-7f438060 r--p 00000000 fd:01 59034409                           /usr/lib/x86_64-linux-gnu/libgmodule-2.0.so.0.4200.6 (deleted)
";
    let vec = parse_proc_maps(contents).unwrap();
    let expected = vec![
        MapRange {
            range_start: 0x00400000,
            range_end: 0x00507000,
            offset: 0,
            dev: "00:14".to_string(),
            flags: Flags {
                read: true,
                write: false,
                execute: true,
                private: true,
            },
            inode: 205736,
            pathname: Some(PathBuf::from("/usr/bin/fish")),
        },
        MapRange {
            range_start: 0x00708000,
            range_end: 0x0070a000,
            offset: 0,
            dev: "00:00".to_string(),
            flags: Flags { //             flags: "rw-p".to_string(),

                read: true,
                write: true,
                execute: false,//todo should fail
                private: true,
            },
            inode: 0,
            pathname: None,
        },
        MapRange {
            range_start: 0x0178c000,
            range_end: 0x01849000,
            offset: 0,
            dev: "00:00".to_string(),
            flags:  Flags { // "rw-p".to_string(),
                read: true,
                write: true,
                execute: false,
                private: true,
            },
            inode: 0,
            pathname: Some(PathBuf::from("[heap]")),
        },
        MapRange {
            range_start: 0x7f438050,
            range_end: 0x7f438060,
            offset: 0,
            dev: "fd:01".to_string(),
            flags: Flags { // "r--p".to_string(),
                read: true,
                write: false,
                execute: false,
                private: true,
            },
            inode: 59034409,
            pathname: Some(PathBuf::from(
                "/usr/lib/x86_64-linux-gnu/libgmodule-2.0.so.0.4200.6 (deleted)",
            )),
        },
    ];
    assert_eq!(vec, expected);

    // Also check that maps_contain_addr works as expected
    assert_eq!(maps_contain_addr(0x00400000, &vec), true);
    assert_eq!(maps_contain_addr(0x00300000, &vec), false);
}

#[test]
fn test_contains_addr_range() {
    let vec = vec![
        MapRange {
            range_start: 0x00400000,
            range_end: 0x00500000,
            offset: 0,
            dev: "00:14".to_string(),
            flags: Flags::default(),
            inode: 205736,
            pathname: Some(PathBuf::from("/usr/bin/fish")),
        },
        MapRange {
            range_start: 0x00600000,
            range_end: 0x00700000,
            offset: 0,
            dev: "00:14".to_string(),
            flags: Flags::default(),
            inode: 205736,
            pathname: Some(PathBuf::from("/usr/bin/fish")),
        },
        MapRange {
            range_start: 0x00700000,
            range_end: 0x00800000,
            offset: 0,
            dev: "00:14".to_string(),
            flags: Flags::default(),
            inode: 205736,
            pathname: Some(PathBuf::from("/usr/bin/fish")),
        },
    ];

    assert_eq!(maps_contain_addr_range(0x00400000, 0x1, &vec), true);
    assert_eq!(maps_contain_addr_range(0x00400000, 0x100000, &vec), true);
    assert_eq!(maps_contain_addr_range(0x00500000 - 1, 1, &vec), true);
    assert_eq!(maps_contain_addr_range(0x00600000, 0x100001, &vec), true);
    assert_eq!(maps_contain_addr_range(0x00600000, 0x200000, &vec), true);

    assert_eq!(maps_contain_addr_range(0x00400000, 0x100001, &vec), false);
    assert_eq!(maps_contain_addr_range(0x00400000, usize::MAX, &vec), false);
    assert_eq!(maps_contain_addr_range(0x00400000, 0, &vec), false);
    assert_eq!(maps_contain_addr_range(0x00400000, 0x00200000, &vec), false);
    assert_eq!(maps_contain_addr_range(0x00400000, 0x00200001, &vec), false);
}
