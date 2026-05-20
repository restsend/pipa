use crate::http::body_reader::BodyReader;
use crate::http::headers::Headers;
use crate::http::status::HttpStatus;

#[derive(Debug)]
pub struct HttpResponse {
    pub status: HttpStatus,
    pub status_text: String,
    pub headers: Headers,
    pub body_reader: BodyReader,
    pub url: String,
}

impl HttpResponse {
    pub fn new(
        status: HttpStatus,
        status_text: String,
        headers: Headers,
        body_reader: BodyReader,
        url: String,
    ) -> Self {
        HttpResponse {
            status,
            status_text,
            headers,
            body_reader,
            url,
        }
    }

    pub fn ok(&self) -> bool {
        self.status.is_success()
    }

    pub fn redirect_count(&self) -> u32 {
        0
    }
}
