use std::ffi::c_void;

pub fn uwprintln(s: &str) {
    uwprint(s);
    uwprint("\n")
}

pub fn uwprint(s: &str) {
    unsafe {
        libc::write(1, s.as_ptr() as *const c_void, s.len());
    }
}

pub fn uwprint_hex(v : usize) {
    uwprint(" 0x");
    let mut buf = [0u8; 16];
    let mut i = 0;
    let mut v = v;
    while v > 0 {
        let c = (v & 0xf) as u8;
        buf[i] = if c < 10 {
            c + '0' as u8
        } else {
            c - 10 + 'a' as u8
        };
        v >>= 4;
        i += 1;
    }
    if i == 0 {
        buf[i] = '0' as u8;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        unsafe {
            libc::write(1, &buf[i] as *const u8 as *const c_void, 1);
        }
    }
    uwprint(" ");
}