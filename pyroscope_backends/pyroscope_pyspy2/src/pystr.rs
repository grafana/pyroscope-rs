use crate::kindasafe;
use crate::kindasafe::read_u64;


pub struct pystr {
    // buf array of 256
    pub buf: [u8; 256],
    pub len: usize,
}

#[derive(Debug)]
pub enum PyyStrError {
    NotCompact,
    ReadError(kindasafe::Error),
}

impl From<kindasafe::Error> for PyyStrError {
    fn from(value: kindasafe::Error) -> Self {
        return PyyStrError::ReadError(value);
    }
}


// struct _object {
//     Py_ssize_t ob_refcnt;
//     _typeobject *ob_type;
// }

// struct {
//     PyObject ob_base;
//     Py_ssize_t length; // 0x10
//     Py_hash_t hash; // 0x18
//     struct {
//     unsigned int interned : 2;
//     unsigned int kind : 3;
//     unsigned int compact : 1;
//     unsigned int ascii : 1;
//     unsigned int ready : 1;
//     };
//     PyASCIIObject::(unnamed struct) state; // 0x20
//     wchar_t *wstr;
// }


// int state_interned(int state) {
// return state & 0x3;
// }
//
// int state_kind(int state) {
// return (state >> 2) & 0x7;
// }
//
// int state_compact(int state) {
// return (state >> 5) & 0x1;
// }
//
// int state_ascii(int state) {
// return (state >> 6) & 0x1;
// }

fn state_interned(state: u32) -> u32 {
    return state & 0x3;
}

fn state_kind(state: u32) -> u32 {
    return (state >> 2) & 0x7;
}

fn state_compact(state: u32) -> u32 {
    return (state >> 5) & 0x1;
}

fn state_ascii(state: u32) -> u32 {
    return (state >> 6) & 0x1;
}


pub fn read(at: usize, s: &mut pystr) -> std::result::Result<(), PyyStrError>{
    let o_len = 0x10;
    let len = read_u64(at + 0x10)? as usize; //todo
    let state = read_u64(at + 0x20)? as usize as u32; //todo
    if state_compact(state) == 0 {
        return Err(PyyStrError::NotCompact);
    }
    //todo check if it is ascii
    // println!("str len {:016x} {:08x}", len, state);
    let mut i = 0;

    while i < 255 && i < len { // todo
        s.buf[i] = read_u64(at + 0x30 + i).unwrap() as u8;//todo
        i += 1;
    }
    s.buf[i] = 0;
    s.len = len;
    return Ok(());
}
