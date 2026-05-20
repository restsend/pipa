#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Continuation = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

impl OpCode {
    pub fn from_u4(v: u8) -> Option<Self> {
        match v {
            0x0 => Some(OpCode::Continuation),
            0x1 => Some(OpCode::Text),
            0x2 => Some(OpCode::Binary),
            0x8 => Some(OpCode::Close),
            0x9 => Some(OpCode::Ping),
            0xA => Some(OpCode::Pong),
            _ => None,
        }
    }

    pub fn is_control(&self) -> bool {
        matches!(self, OpCode::Close | OpCode::Ping | OpCode::Pong)
    }
}

#[derive(Debug, Clone)]
pub struct WsFrame {
    pub fin: bool,
    pub opcode: OpCode,
    pub mask: Option<[u8; 4]>,
    pub payload: Vec<u8>,
}

impl WsFrame {
    pub fn new_text(payload: Vec<u8>) -> Self {
        WsFrame {
            fin: true,
            opcode: OpCode::Text,
            mask: Some(generate_mask()),
            payload,
        }
    }

    pub fn new_binary(payload: Vec<u8>) -> Self {
        WsFrame {
            fin: true,
            opcode: OpCode::Binary,
            mask: Some(generate_mask()),
            payload,
        }
    }

    pub fn new_close(code: u16, reason: &str) -> Self {
        let mut payload = Vec::with_capacity(2 + reason.len());
        payload.extend_from_slice(&code.to_be_bytes());
        payload.extend_from_slice(reason.as_bytes());
        WsFrame {
            fin: true,
            opcode: OpCode::Close,
            mask: Some(generate_mask()),
            payload,
        }
    }

    pub fn new_pong(payload: Vec<u8>) -> Self {
        WsFrame {
            fin: true,
            opcode: OpCode::Pong,
            mask: Some(generate_mask()),
            payload,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let b0 = if self.fin { 0x80u8 } else { 0u8 } | (self.opcode as u8);
        buf.push(b0);

        let masked = self.mask.is_some() as u8;
        let payload_len = self.payload.len();

        if payload_len < 126 {
            buf.push((masked << 7) | payload_len as u8);
        } else if payload_len <= 0xFFFF {
            buf.push((masked << 7) | 126);
            buf.extend_from_slice(&(payload_len as u16).to_be_bytes());
        } else {
            buf.push((masked << 7) | 127);
            buf.extend_from_slice(&(payload_len as u64).to_be_bytes());
        }

        let mask_key = self.mask.unwrap_or([0u8; 4]);
        if self.mask.is_some() {
            buf.extend_from_slice(&mask_key);
        }

        let mut masked_payload = self.payload.clone();
        if self.mask.is_some() {
            for (i, byte) in masked_payload.iter_mut().enumerate() {
                *byte ^= mask_key[i % 4];
            }
        }
        buf.extend_from_slice(&masked_payload);
        buf
    }

    pub fn parse_all(data: &[u8]) -> Result<Vec<WsFrame>, String> {
        let mut frames = Vec::new();
        let mut pos = 0;
        while pos < data.len() {
            let remaining = data.len() - pos;
            if remaining < 2 {
                break;
            }
            let b0 = data[pos];
            let b1 = data[pos + 1];
            let fin = (b0 & 0x80) != 0;
            let opcode_val = b0 & 0x0F;
            let opcode = OpCode::from_u4(opcode_val)
                .ok_or_else(|| format!("unknown opcode: {opcode_val:#x}"))?;
            let masked = (b1 & 0x80) != 0;
            let mut payload_len = (b1 & 0x7F) as u64;

            let mut header_len = 2;
            if payload_len == 126 {
                header_len += 2;
            } else if payload_len == 127 {
                header_len += 8;
            }
            let mask_len = if masked { 4 } else { 0 };
            let total_header = header_len + mask_len;

            if remaining < total_header {
                break;
            }

            if payload_len == 126 {
                payload_len = u64::from_be_bytes([0, 0, 0, 0, 0, 0, data[pos + 2], data[pos + 3]]);
            } else if payload_len == 127 {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&data[pos + 2..pos + 10]);
                payload_len = u64::from_be_bytes(arr);
            }

            let total_frame = total_header + payload_len as usize;
            if remaining < total_frame {
                break;
            }

            let mut payload = data[pos + total_header..pos + total_frame].to_vec();

            let mask_key = if masked {
                let mk = [
                    data[pos + header_len],
                    data[pos + header_len + 1],
                    data[pos + header_len + 2],
                    data[pos + header_len + 3],
                ];
                for (i, byte) in payload.iter_mut().enumerate() {
                    *byte ^= mk[i % 4];
                }
                Some(mk)
            } else {
                None
            };

            frames.push(WsFrame {
                fin,
                opcode,
                mask: mask_key,
                payload,
            });

            pos += total_frame;
        }
        Ok(frames)
    }
}

fn generate_mask() -> [u8; 4] {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    [
        (nanos & 0xFF) as u8,
        ((nanos >> 8) & 0xFF) as u8,
        ((nanos >> 16) & 0xFF) as u8,
        ((nanos >> 24) & 0xFF) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_text() {
        let frame = WsFrame::new_text(b"Hello".to_vec());
        let encoded = frame.encode();
        let parsed = WsFrame::parse_all(&encoded).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].opcode, OpCode::Text);
        assert!(parsed[0].fin);
        assert_eq!(parsed[0].payload, b"Hello");
    }

    #[test]
    fn test_close_frame() {
        let frame = WsFrame::new_close(1000, "Normal");
        let encoded = frame.encode();
        let parsed = WsFrame::parse_all(&encoded).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].opcode, OpCode::Close);
        assert_eq!(&parsed[0].payload[..2], &[0x03, 0xE8]);
    }
}
