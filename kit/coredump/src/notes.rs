use object::elf;
use object::endian::LittleEndian;
use object::read::elf::ProgramHeader as _;

use crate::{CoredumpError, Mapping, ThreadInfo};

pub fn parse_notes(
    endian: LittleEndian,
    program_headers: &[elf::ProgramHeader64<LittleEndian>],
    data: &[u8],
) -> Result<(Vec<Mapping>, Vec<ThreadInfo>), CoredumpError> {
    let mut mappings = Vec::new();
    let mut threads = Vec::new();

    for ph in program_headers {
        let Some(mut iter) = ph
            .notes(endian, data)
            .map_err(|e| CoredumpError::ElfParse(e.to_string()))?
        else {
            continue;
        };

        while let Some(note) = iter
            .next()
            .map_err(|e| CoredumpError::ElfParse(e.to_string()))?
        {
            let name = note.name();
            let n_type = note.n_type(endian);
            let desc = note.desc();

            if name == elf::ELF_NOTE_CORE {
                match n_type {
                    elf::NT_PRSTATUS => {
                        parse_prstatus(desc, &mut threads)?;
                    }
                    elf::NT_FILE => {
                        mappings = parse_nt_file(desc)?;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok((mappings, threads))
}

// x86_64 prstatus layout constants (from `pahole elf_prstatus`).
const PRSTATUS_SIZE_X86_64: usize = 336;
const LWP_OFFSET: usize = 32;
const GPREGS_OFFSET: usize = 112;
const GPREGS_SIZE: usize = 216; // 27 registers × 8 bytes
const FS_BASE_REG_INDEX: usize = 21;

fn parse_prstatus(desc: &[u8], threads: &mut Vec<ThreadInfo>) -> Result<(), CoredumpError> {
    if desc.len() < PRSTATUS_SIZE_X86_64 {
        return Err(CoredumpError::InvalidNote(
            "NT_PRSTATUS too short for x86_64",
        ));
    }

    let lwp = u32::from_le_bytes(desc[LWP_OFFSET..LWP_OFFSET + 4].try_into().unwrap());

    let gp_regs = desc[GPREGS_OFFSET..GPREGS_OFFSET + GPREGS_SIZE].to_vec();

    let fs_base_offset = FS_BASE_REG_INDEX * 8;
    let tp_base = u64::from_le_bytes(
        gp_regs[fs_base_offset..fs_base_offset + 8]
            .try_into()
            .unwrap(),
    );

    threads.push(ThreadInfo {
        lwp,
        gp_regs,
        tp_base,
    });
    Ok(())
}

const ENTRY_SIZE: usize = 24; // start(8) + end(8) + file_offset_pages(8)

fn parse_nt_file(desc: &[u8]) -> Result<Vec<Mapping>, CoredumpError> {
    if desc.len() < 16 {
        return Err(CoredumpError::InvalidNote("NT_FILE too short for header"));
    }

    let num_files = u64::from_le_bytes(desc[0..8].try_into().unwrap()) as usize;
    let page_size = u64::from_le_bytes(desc[8..16].try_into().unwrap());

    let entries_end = 16 + num_files * ENTRY_SIZE;
    if desc.len() < entries_end {
        return Err(CoredumpError::InvalidNote("NT_FILE truncated entries"));
    }

    struct Entry {
        start: u64,
        end: u64,
        file_offset_pages: u64,
    }

    let mut entries = Vec::with_capacity(num_files);
    for i in 0..num_files {
        let base = 16 + i * ENTRY_SIZE;
        let start = u64::from_le_bytes(desc[base..base + 8].try_into().unwrap());
        let end = u64::from_le_bytes(desc[base + 8..base + 16].try_into().unwrap());
        let file_offset_pages = u64::from_le_bytes(desc[base + 16..base + 24].try_into().unwrap());
        entries.push(Entry {
            start,
            end,
            file_offset_pages,
        });
    }

    let mut names_slice = &desc[entries_end..];
    let mut mappings = Vec::with_capacity(num_files);
    for entry in &entries {
        let null_pos =
            names_slice
                .iter()
                .position(|&b| b == 0)
                .ok_or(CoredumpError::InvalidNote(
                    "NT_FILE filename missing null terminator",
                ))?;
        let path = if null_pos > 0 {
            Some(String::from_utf8_lossy(&names_slice[..null_pos]).into_owned())
        } else {
            None
        };
        names_slice = &names_slice[null_pos + 1..];
        mappings.push(Mapping {
            vaddr: entry.start,
            length: entry.end - entry.start,
            flags: 0,
            file_offset: entry.file_offset_pages * page_size,
            path,
        });
    }
    Ok(mappings)
}
