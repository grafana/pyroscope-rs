#![no_std]

use python_unwind::RawFrame;

/// Default size of each per-shard bbqueue buffer in bytes (256 KiB).
/// Can be overridden at compile time via the `ring-512k` or `ring-1m` features.
#[cfg(feature = "ring-1m")]
pub const RING_SIZE: usize = 1024 * 1024;
#[cfg(all(feature = "ring-512k", not(feature = "ring-1m")))]
pub const RING_SIZE: usize = 512 * 1024;
#[cfg(not(any(feature = "ring-512k", feature = "ring-1m")))]
pub const RING_SIZE: usize = 256 * 1024;

/// Default notification interval: notify the reader thread every N sample writes.
/// Can be overridden at runtime via the `notify_interval` field in profiler state.
pub const DEFAULT_NOTIFY_INTERVAL: u32 = 32;

/// Size of the record header: thread_id (u32) + depth (u32).
const HEADER_SIZE: usize = 8;

/// Size of one `RawFrame` in bytes (code_object: u64 + instr_offset: u64).
const FRAME_SIZE: usize = core::mem::size_of::<RawFrame>();

/// Compute the byte length of a record with `depth` frames.
fn record_len(depth: usize) -> usize {
    HEADER_SIZE + depth * FRAME_SIZE
}

/// Write a stack trace record into a framed bbqueue producer.
///
/// Record layout: `[thread_id: u32][depth: u32][frames[0..depth]]`
///
/// Called from the signal handler inside a shard lock.
/// Returns `true` on success, `false` if the queue is full.
///
/// Generic over const `N` to support different ring buffer sizes.
pub fn write<const N: usize>(
    producer: &mut bbqueue::framed::FrameProducer<'static, N>,
    tid: u32,
    frames: &[RawFrame],
    depth: usize,
) -> bool {
    let len = record_len(depth);

    // bbqueue 0.6 switched framed producers to exact-size grants.
    let mut grant = match producer.grant_exact(len) {
        Ok(g) => g,
        Err(_) => return false,
    };

    // Write header.
    grant[0..4].copy_from_slice(&tid.to_ne_bytes());
    grant[4..8].copy_from_slice(&(depth as u32).to_ne_bytes());

    // Write frames as raw bytes.
    // SAFETY: RawFrame is #[repr(C)], Copy, contains only u64 fields.
    // We reinterpret the &[RawFrame] as &[u8] for the copy.
    let frames_src = &frames[..depth];
    let src_bytes = unsafe {
        core::slice::from_raw_parts(frames_src.as_ptr() as *const u8, depth * FRAME_SIZE)
    };
    grant[HEADER_SIZE..HEADER_SIZE + src_bytes.len()].copy_from_slice(src_bytes);

    grant.commit(len);
    true
}

/// Parsed view of a record from a bbqueue read grant.
///
/// Holds a reference to the raw grant bytes. Frames are read via
/// [`frame()`](RecordView::frame) which handles potentially unaligned data.
pub struct RecordView<'a> {
    pub tid: u32,
    pub depth: u32,
    buf: &'a [u8],
}

impl<'a> RecordView<'a> {
    /// Read the frame at index `i`.
    ///
    /// Uses `read_unaligned` because bbqueue frame grants are not guaranteed
    /// to be 8-byte aligned.
    pub fn frame(&self, i: usize) -> RawFrame {
        let offset = HEADER_SIZE + i * FRAME_SIZE;
        let code_ptr = self.buf[offset..].as_ptr() as *const u64;
        let instr_ptr = self.buf[offset + 8..].as_ptr() as *const u64;
        // SAFETY: bounds checked by parse_record, and we use read_unaligned.
        unsafe {
            RawFrame {
                code_object: core::ptr::read_unaligned(code_ptr),
                instr_offset: core::ptr::read_unaligned(instr_ptr),
            }
        }
    }
}

/// Parse a raw byte slice (from a `FrameGrantR`) into a [`RecordView`].
///
/// Returns `None` if the buffer is too small or the depth field is
/// inconsistent with the buffer length.
pub fn parse_record(buf: &[u8]) -> Option<RecordView<'_>> {
    if buf.len() < HEADER_SIZE {
        return None;
    }

    let tid = u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let depth = u32::from_ne_bytes([buf[4], buf[5], buf[6], buf[7]]);

    let expected_len = record_len(depth as usize);
    if buf.len() < expected_len {
        return None;
    }

    Some(RecordView { tid, depth, buf })
}
