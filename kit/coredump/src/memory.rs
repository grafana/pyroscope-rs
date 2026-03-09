use object::endian::LittleEndian;
use object::read::elf::ProgramHeader as _;

use crate::{CoredumpError, Segment};

pub fn collect_segments(
    endian: LittleEndian,
    program_headers: &[object::elf::ProgramHeader64<LittleEndian>],
) -> Vec<Segment> {
    program_headers
        .iter()
        .filter(|ph| ph.p_type(endian) == object::elf::PT_LOAD)
        .map(|ph| Segment {
            vaddr: ph.p_vaddr(endian),
            memsz: ph.p_memsz(endian),
            file_offset: ph.p_offset(endian),
            filesz: ph.p_filesz(endian),
        })
        .collect()
}

pub fn read(
    data: &[u8],
    segments: &[Segment],
    addr: u64,
    buf: &mut [u8],
) -> Result<(), CoredumpError> {
    let len = buf.len() as u64;
    for seg in segments {
        if addr >= seg.vaddr && addr.saturating_add(len) <= seg.vaddr.saturating_add(seg.memsz) {
            let seg_rel = addr - seg.vaddr;
            if seg_rel + len > seg.filesz {
                return Err(CoredumpError::ReadOutOfBounds {
                    addr,
                    len: buf.len(),
                });
            }
            let file_pos = seg.file_offset + seg_rel;
            let file_end = file_pos + len;
            if file_end as usize > data.len() {
                return Err(CoredumpError::ReadOutOfBounds {
                    addr,
                    len: buf.len(),
                });
            }
            buf.copy_from_slice(&data[file_pos as usize..file_end as usize]);
            return Ok(());
        }
    }
    Err(CoredumpError::AddressNotMapped(addr))
}
