use core::ptr;

/// Converts a 32-bit unsigned integer to a hexadecimal string
pub fn format_hex(num: u32, uppercase: bool) -> heapless::String<10> {
    let mut buf = heapless::String::<10>::new();
    let _ = buf.push_str("0x");

    for i in (0..8).rev() {
        let digit = (num >> (i * 4)) & 0xF;
        let c = match digit {
            0x0..=0x9 => (b'0' + digit as u8) as char,
            0xA..=0xF => {
                if uppercase {
                    (b'A' + (digit as u8 - 10)) as char
                } else {
                    (b'a' + (digit as u8 - 10)) as char
                }
            }
            _ => unreachable!(),
        };
        let _ = buf.push(c);
    }

    buf
}

/// Parses a hexadecimal string into a 32-bit unsigned integer
pub fn parse_hex(hex_str: &str) -> Option<u32> {
    if !hex_str.starts_with("0x") || hex_str.len() != 10 {
        return None;
    }

    let mut result = 0u32;
    for c in hex_str[2..].chars() {
        let digit = c.to_digit(16)?;
        result = result << 4 | digit;
    }

    Some(result)
}

/// Reads a 32-bit unsigned integer from the specified memory address
#[inline]
pub fn read_memory(addr: u32) -> u32 {
    unsafe { ptr::read_volatile(addr as *const u32) }
}

/// Writes a 32-bit unsigned integer to the specified memory address
#[inline]
pub fn write_memory(addr: u32, val: u32) {
    unsafe { ptr::write_volatile(addr as *mut u32, val) }
}
