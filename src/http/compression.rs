use std::io::Read;

pub enum Decompressor {
    None,
    Gzip,
    Deflate,
}

impl Decompressor {
    pub fn new(encoding: Option<&str>) -> Self {
        match encoding {
            Some("gzip") | Some("x-gzip") => Decompressor::Gzip,
            Some("deflate") => Decompressor::Deflate,
            _ => Decompressor::None,
        }
    }

    pub fn decompress(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        match self {
            Decompressor::None => Ok(input.to_vec()),
            Decompressor::Gzip => {
                let mut decoder = flate2::read::GzDecoder::new(input);
                let mut out = Vec::new();
                decoder
                    .read_to_end(&mut out)
                    .map_err(|e| format!("gzip error: {e}"))?;
                Ok(out)
            }
            Decompressor::Deflate => {
                let mut decoder = flate2::read::ZlibDecoder::new(input);
                let mut out = Vec::new();
                decoder
                    .read_to_end(&mut out)
                    .map_err(|e| format!("deflate error: {e}"))?;
                Ok(out)
            }
        }
    }
}
