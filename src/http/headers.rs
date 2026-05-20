use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Headers {
    map: HashMap<String, String>,
}

impl Headers {
    pub fn new() -> Self {
        Headers {
            map: HashMap::new(),
        }
    }

    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), String> {
        let mut map = HashMap::new();
        let mut pos = 0;
        let len = data.len();

        loop {
            if pos >= len {
                return Err("unexpected end of headers".into());
            }
            if data[pos] == b'\r' {
                if pos + 1 < len && data[pos + 1] == b'\n' {
                    return Ok((Headers { map }, pos + 2));
                }
                return Err("malformed header terminator".into());
            }

            let line_start = pos;
            while pos < len && data[pos] != b'\r' {
                pos += 1;
            }
            if pos >= len {
                return Err("unexpected end of headers".into());
            }
            let line_end = pos;
            if pos + 1 >= len || data[pos + 1] != b'\n' {
                return Err("malformed header line".into());
            }
            pos += 2;

            let line = &data[line_start..line_end];
            if line.is_empty() {
                return Err("empty header line".into());
            }

            let colon_pos = line.iter().position(|&b| b == b':');
            match colon_pos {
                Some(cpos) => {
                    let name = String::from_utf8_lossy(&line[..cpos]).trim().to_lowercase();
                    let value = String::from_utf8_lossy(&line[cpos + 1..])
                        .trim()
                        .to_string();
                    map.insert(name, value);
                }
                None => {
                    return Err(format!(
                        "malformed header (no colon): {:?}",
                        String::from_utf8_lossy(line)
                    ));
                }
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.map.get(&name.to_lowercase()).map(|s| s.as_str())
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.map.insert(name.to_lowercase(), value.to_string());
    }

    pub fn remove(&mut self, name: &str) {
        self.map.remove(&name.to_lowercase());
    }

    pub fn contains(&self, name: &str) -> bool {
        self.map.contains_key(&name.to_lowercase())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.map.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn to_request_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for (name, value) in &self.map {
            buf.extend_from_slice(name.as_bytes());
            buf.extend_from_slice(b": ");
            buf.extend_from_slice(value.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
        buf
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl Default for Headers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_headers() {
        let data = b"Content-Type: text/plain\r\nContent-Length: 42\r\n\r\n";
        let (headers, consumed) = Headers::from_bytes(data).unwrap();
        assert_eq!(consumed, data.len());
        assert_eq!(headers.get("content-type").unwrap(), "text/plain");
        assert_eq!(headers.get("Content-Length").unwrap(), "42");
    }

    #[test]
    fn test_case_insensitive() {
        let data = b"X-Custom: value\r\n\r\n";
        let (headers, _) = Headers::from_bytes(data).unwrap();
        assert_eq!(headers.get("x-custom").unwrap(), "value");
        assert_eq!(headers.get("X-CUSTOM").unwrap(), "value");
    }

    #[test]
    fn test_serialize() {
        let mut h = Headers::new();
        h.set("Host", "example.com");
        h.set("Accept", "*/*");
        let bytes = h.to_request_bytes();
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("host: example.com\r\n"));
        assert!(s.contains("accept: */*\r\n"));
    }
}
