use crate::http::headers::Headers;
use crate::http::status::HttpStatus;

const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub struct WsHandshake;

impl WsHandshake {
    pub fn build_request(host: &str, _path: &str, key: &str) -> Headers {
        let mut headers = Headers::new();
        headers.set("Host", host);
        headers.set("Upgrade", "websocket");
        headers.set("Connection", "Upgrade");
        headers.set("Sec-WebSocket-Key", key);
        headers.set("Sec-WebSocket-Version", "13");
        headers
    }

    pub fn generate_key() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let input = format!("{ts:x}").as_bytes().to_vec();
        crate::builtins::base64::base64_encode_standard(&input)
    }

    pub fn validate_response(status: HttpStatus, headers: &Headers) -> Result<String, String> {
        if status.0 != 101 {
            return Err(format!("expected 101, got {}", status.0));
        }
        let upgrade = headers.get("upgrade").ok_or("missing Upgrade header")?;
        if !upgrade.eq_ignore_ascii_case("websocket") {
            return Err(format!("unexpected Upgrade: {upgrade}"));
        }
        let accept = headers
            .get("sec-websocket-accept")
            .ok_or("missing Sec-WebSocket-Accept")?;
        Ok(accept.to_string())
    }

    pub fn compute_accept(key: &str) -> String {
        let input = format!("{key}{WS_GUID}");
        let hash = sha1(input.as_bytes());
        crate::builtins::base64::base64_encode_standard(&hash)
    }

    pub fn verify_accept(key: &str, accept: &str) -> bool {
        let expected = Self::compute_accept(key);
        expected == accept
    }
}

fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let len_bits = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&len_bits.to_be_bytes());

    for chunk in padded.chunks(64) {
        let mut w = [0u32; 80];
        for (i, word) in w.iter_mut().enumerate().take(16) {
            let idx = i * 4;
            *word =
                u32::from_be_bytes([chunk[idx], chunk[idx + 1], chunk[idx + 2], chunk[idx + 3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for i in 0..80 {
            let (f, k): (u32, u32) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    let mut result = [0u8; 20];
    for (i, val) in h.iter().enumerate() {
        result[i * 4..(i + 1) * 4].copy_from_slice(&val.to_be_bytes());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha1_basic() {
        let hash = sha1(b"hello");
        assert_eq!(
            hash,
            [
                0xaa, 0xf4, 0xc6, 0x1d, 0xdc, 0xc5, 0xe8, 0xa2, 0xda, 0xbe, 0xde, 0x0f, 0x3b, 0x48,
                0x2c, 0xd9, 0xae, 0xa9, 0x43, 0x4d
            ]
        );
    }

    #[test]
    fn test_compute_accept() {
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = WsHandshake::compute_accept(key);
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn test_generate_key_roundtrip() {
        let key = WsHandshake::generate_key();
        assert!(!key.is_empty());
        let accept = WsHandshake::compute_accept(&key);
        assert!(WsHandshake::verify_accept(&key, &accept));
    }
}
