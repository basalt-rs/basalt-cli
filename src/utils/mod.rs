use std::net::Ipv4Addr;

use anyhow::bail;

pub fn make_game_code<const N: usize>(bytes: [u8; N]) -> String {
    let mut s = String::with_capacity(2 * N);
    for b in bytes {
        s.push(char::from((b >> 4) + b'a'));
        s.push(char::from((b & 0xf) + b'a'));
    }
    s
}

pub fn code_to_address(code: &str) -> anyhow::Result<(Ipv4Addr, u16)> {
    // TODO(Jack): Support RLE

    let mut raw_bytes: Vec<u8> = Vec::new();
    for pair in code.as_bytes().chunks(2) {
        if let [c1, c2] = pair {
            let mut b: u8 = (c1 - b'a') << 4;
            b += c2 - b'a';
            raw_bytes.push(b);
        } else {
            bail!("Invalid gamecode provided")
        }
    }

    dbg!(raw_bytes.len());

    let mut x = [0; 6];
    x[..4].copy_from_slice(&raw_bytes[..4]);
    x[4..].copy_from_slice(&raw_bytes[4..]);
    let ip = Ipv4Addr::new(x[0], x[1], x[2], x[3]);
    let port = u16::from_be_bytes([x[4], x[5]]);
    Ok((ip, port))
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use crate::utils::code_to_address;

    #[test]
    fn from_game_code() {
        assert_eq!(
            code_to_address("makiabecbpja").unwrap(),
            (Ipv4Addr::new(192, 168, 1, 66), 8080)
        );
    }
}
