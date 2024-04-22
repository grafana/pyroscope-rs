use crate::kindasafe;
use crate::kindasafe::read_u64;


pub struct pystr {
    // buf array of 256
    pub buf: [u8; 256],
    pub len: usize,
}

#[derive(Debug)]
pub enum PyyStrError {
    ReadError(kindasafe::Error),
}

impl From<kindasafe::Error> for PyyStrError {
    fn from(value: kindasafe::Error) -> Self {
        return PyyStrError::ReadError(value);
    }
}




pub fn read(at: usize, s: &mut pystr) -> std::result::Result<(), PyyStrError>{
    let o_len = 0x10;
    let len = read_u64(at + 0x10)? as usize; //todo
    let state = read_u64(at + 0x20)? as usize as u32; //todo
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
