#![no_std]



pub type Ptr = u64;

pub struct MemError;

pub struct PthreadKey {
    // pthread_key_t
    pub v: u32,
}

pub trait ReadMem {
    fn read_u64(at: Ptr) -> Result<Ptr, MemError>;
}

pub trait ReadTLS {
    fn read_tls(k: PthreadKey) -> Result<Ptr, MemError>;
}

pub struct PythonOffsets {

}


#[cfg(test)]
mod tests {

    #[test]
    fn test() {

    }
}