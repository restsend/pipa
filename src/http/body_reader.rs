use crate::http::chunked::ChunkedDecoder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyMode {
    ContentLength(usize),
    Chunked,
    ConnectionClose,
    None,
}

#[derive(Debug)]
pub struct BodyReader {
    mode: BodyMode,
    buffer: Vec<u8>,
    total_read: usize,
    done: bool,
    chunked_decoder: Option<ChunkedDecoder>,
}

impl BodyReader {
    pub fn new(mode: BodyMode) -> Self {
        let chunked_decoder = if matches!(mode, BodyMode::Chunked) {
            Some(ChunkedDecoder::new())
        } else {
            None
        };
        BodyReader {
            mode,
            buffer: Vec::new(),
            total_read: 0,
            done: false,
            chunked_decoder,
        }
    }

    pub fn feed(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
        if self.done {
            return Ok(Vec::new());
        }

        match self.mode {
            BodyMode::ContentLength(len) => {
                let remaining = len.saturating_sub(self.total_read);
                let take = data.len().min(remaining);
                let chunk = data[..take].to_vec();
                self.buffer.extend_from_slice(&chunk);
                self.total_read += take;
                if self.total_read >= len {
                    self.done = true;
                }
                Ok(chunk)
            }
            BodyMode::Chunked => {
                let decoder = self
                    .chunked_decoder
                    .as_mut()
                    .ok_or("chunked decoder not initialized")?;
                let decoded = decoder.feed(data)?;
                if !decoded.is_empty() {
                    self.buffer.extend_from_slice(&decoded);
                    self.total_read += decoded.len();
                }
                if decoder.is_done() {
                    self.done = true;
                }
                Ok(decoded)
            }
            BodyMode::ConnectionClose => {
                self.buffer.extend_from_slice(data);
                self.total_read += data.len();
                Ok(data.to_vec())
            }
            BodyMode::None => {
                self.done = true;
                Ok(Vec::new())
            }
        }
    }

    pub fn finish(&mut self) {
        self.done = true;
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn total_read(&self) -> usize {
        self.total_read
    }

    pub fn mode(&self) -> BodyMode {
        self.mode
    }

    pub fn take_body(&mut self) -> Vec<u8> {
        self.done = true;
        core::mem::take(&mut self.buffer)
    }

    pub fn body_text(&self) -> Result<String, String> {
        String::from_utf8(self.buffer.clone())
            .map_err(|e| format!("body utf8 decode: {e}"))
    }
}
