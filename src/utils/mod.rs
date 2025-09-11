use std::net::Ipv4Addr;

use anyhow::bail;

/// Parses gamecode into Ipv4Addr
pub fn parse_gamecode(code: String) -> anyhow::Result<(Ipv4Addr, u16)> {
    let chars: Vec<char> = code.chars().collect();
    if chars.len() % 2 != 0 {
        bail!("String length must be even");
    }

    let mut bytes = Vec::with_capacity(chars.len() / 2);
    for pair in chars.chunks(2) {
        let hi = pair[0] as u8 - b'a';
        let lo = pair[1] as u8 - b'a';
        if hi > 0x0f || lo > 0x0f {
            bail!("Invalid characters: {}{}", pair[0], pair[1]);
        }
        bytes.push((hi << 4) | lo);
    }

    let addr = Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]);
    todo!()

    // Ok(addr)
}
