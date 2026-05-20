#[derive(Debug)]
pub struct ChunkedDecoder {
    state: ChunkState,
    chunk_size: usize,
    buffer: Vec<u8>,
    done: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChunkState {
    Size,
    Data,
    Trailer,
    Done,
}

impl ChunkedDecoder {
    pub fn new() -> Self {
        ChunkedDecoder {
            state: ChunkState::Size,
            chunk_size: 0,
            buffer: Vec::new(),
            done: false,
        }
    }

    pub fn feed(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        self.buffer.extend_from_slice(data);
        let mut output = Vec::new();

        loop {
            match self.state {
                ChunkState::Size => {
                    if let Some(end) = self.buffer.iter().position(|&b| b == b'\r') {
                        if end + 1 >= self.buffer.len() {
                            break;
                        }
                        let line = &self.buffer[..end];
                        let size_str = core::str::from_utf8(line)
                            .map_err(|e| format!("chunk size utf8: {e}"))?;
                        let size_end = size_str.find(|c: char| c == ';' || c == ' ');
                        let size_hex = match size_end {
                            Some(pos) => &size_str[..pos],
                            None => size_str,
                        };
                        let size = usize::from_str_radix(size_hex.trim(), 16)
                            .map_err(|e| format!("chunk size parse: {e}"))?;
                        self.chunk_size = size;
                        self.buffer.drain(..end + 2);
                        if size == 0 {
                            self.state = ChunkState::Trailer;
                        } else {
                            self.state = ChunkState::Data;
                        }
                    } else {
                        break;
                    }
                }
                ChunkState::Data => {
                    if self.buffer.len() >= self.chunk_size + 2 {
                        let chunk_data = self.buffer.drain(..self.chunk_size).collect::<Vec<_>>();
                        self.buffer.drain(..2);
                        output.extend(chunk_data);
                        self.state = ChunkState::Size;
                    } else {
                        break;
                    }
                }
                ChunkState::Done => {
                    break;
                }
                ChunkState::Trailer => {
                    if self.buffer.len() >= 2 {
                        if self.buffer[0] == b'\r' && self.buffer[1] == b'\n' {
                            self.buffer.drain(..2);
                            self.state = ChunkState::Done;
                            self.done = true;
                            break;
                        }
                        if let Some(end) = self.buffer.windows(2).position(|w| w == b"\r\n") {
                            self.buffer.drain(..end + 2);
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(output)
    }

    pub fn is_done(&self) -> bool {
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_chunk() {
        let mut dec = ChunkedDecoder::new();
        let out = dec.feed(b"5\r\nHello\r\n0\r\n\r\n").unwrap();
        assert_eq!(out, b"Hello");
        assert!(dec.is_done());
    }

    #[test]
    fn test_multi_chunk() {
        let mut dec = ChunkedDecoder::new();
        let out = dec
            .feed(b"6\r\nHello \r\n6\r\nWorld!\r\n0\r\n\r\n")
            .unwrap();
        assert_eq!(out, b"Hello World!");
        assert!(dec.is_done());
    }

    #[test]
    fn test_partial_feed() {
        let mut dec = ChunkedDecoder::new();
        let out1 = dec.feed(b"5\r\nHel").unwrap();
        assert!(out1.is_empty());
        assert!(!dec.is_done());

        let out2 = dec.feed(b"lo\r\n0\r\n\r\n").unwrap();
        assert_eq!(out2, b"Hello");
        assert!(dec.is_done());
    }
}
