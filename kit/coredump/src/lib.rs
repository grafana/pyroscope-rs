mod memory;
mod notes;

use object::elf;
use object::endian::LittleEndian;
use object::read::elf::FileHeader as _;

/// A PT_LOAD segment from the core file, used for virtual address resolution.
#[derive(Debug, Clone)]
pub struct Segment {
    pub vaddr: u64,
    pub memsz: u64,
    pub file_offset: u64,
    pub filesz: u64,
}

/// A memory mapping entry parsed from the NT_FILE note.
#[derive(Debug, Clone)]
pub struct Mapping {
    pub vaddr: u64,
    pub length: u64,
    pub flags: u32,
    pub file_offset: u64,
    pub path: Option<String>,
}

/// Thread register state parsed from NT_PRSTATUS.
#[derive(Debug, Clone)]
pub struct ThreadInfo {
    /// Light Weight Process ID (thread ID).
    pub lwp: u32,
    /// General purpose registers (raw bytes, x86_64: 27 × 8 = 216 bytes).
    pub gp_regs: Vec<u8>,
    /// Thread pointer base (TLS base). On x86_64 this is fs_base.
    pub tp_base: u64,
}

/// An opened and parsed ELF core file.
pub struct Coredump {
    data: memmap2::Mmap,
    pub mappings: Vec<Mapping>,
    pub threads: Vec<ThreadInfo>,
    segments: Vec<Segment>,
}

/// Errors that can occur when opening or reading a coredump.
#[derive(Debug)]
pub enum CoredumpError {
    Io(std::io::Error),
    ElfParse(String),
    NotCoreFile,
    InvalidNote(&'static str),
    AddressNotMapped(u64),
    ReadOutOfBounds { addr: u64, len: usize },
}

impl std::fmt::Display for CoredumpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoredumpError::Io(e) => write!(f, "I/O error: {e}"),
            CoredumpError::ElfParse(msg) => write!(f, "ELF parse error: {msg}"),
            CoredumpError::NotCoreFile => write!(f, "not an ELF core file"),
            CoredumpError::InvalidNote(msg) => write!(f, "invalid note: {msg}"),
            CoredumpError::AddressNotMapped(addr) => {
                write!(f, "address 0x{addr:x} not mapped in coredump")
            }
            CoredumpError::ReadOutOfBounds { addr, len } => {
                write!(f, "read of {len} bytes at 0x{addr:x} out of bounds")
            }
        }
    }
}

impl std::error::Error for CoredumpError {}

impl From<std::io::Error> for CoredumpError {
    fn from(e: std::io::Error) -> Self {
        CoredumpError::Io(e)
    }
}

impl Coredump {
    /// Open and parse an ELF core file at the given path.
    pub fn open(path: &str) -> Result<Self, CoredumpError> {
        let file = std::fs::File::open(path)?;
        // SAFETY: the file is a read-only coredump; we never write through the mapping.
        let data = unsafe { memmap2::Mmap::map(&file) }?;
        Self::parse(data)
    }

    fn parse(data: memmap2::Mmap) -> Result<Self, CoredumpError> {
        let endian = LittleEndian;

        let header = elf::FileHeader64::<LittleEndian>::parse(&*data)
            .map_err(|e| CoredumpError::ElfParse(e.to_string()))?;

        if header.e_type(endian) != elf::ET_CORE {
            return Err(CoredumpError::NotCoreFile);
        }

        let program_headers = header
            .program_headers(endian, &*data)
            .map_err(|e| CoredumpError::ElfParse(e.to_string()))?;

        let segments = memory::collect_segments(endian, program_headers);
        let (mappings, threads) = notes::parse_notes(endian, program_headers, &data)?;

        Ok(Coredump {
            data,
            mappings,
            threads,
            segments,
        })
    }

    /// Read bytes from the coredump at the given virtual address.
    pub fn read(&self, addr: u64, buf: &mut [u8]) -> Result<(), CoredumpError> {
        memory::read(&self.data, &self.segments, addr, buf)
    }

    /// Read a little-endian u64 from the coredump at the given virtual address.
    pub fn read_u64(&self, addr: u64) -> Result<u64, CoredumpError> {
        let mut buf = [0u8; 8];
        self.read(addr, &mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    /// Read a little-endian u32 from the coredump at the given virtual address.
    pub fn read_u32(&self, addr: u64) -> Result<u32, CoredumpError> {
        let mut buf = [0u8; 4];
        self.read(addr, &mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }
}
