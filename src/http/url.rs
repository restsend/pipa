#[derive(Debug, Clone)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub query: String,
    pub full: String,
}

impl Url {
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();

        let (scheme, rest) = match input.find("://") {
            Some(pos) => {
                let s = &input[..pos].to_lowercase();
                let rest = &input[pos + 3..];
                (s.to_string(), rest)
            }
            None => return Err(format!("missing scheme in url: {input}")),
        };

        if scheme != "http" && scheme != "https" && scheme != "ws" && scheme != "wss" {
            return Err(format!("unsupported scheme: {scheme}"));
        }

        let default_port: u16 = match scheme.as_str() {
            "https" | "wss" => 443,
            _ => 80,
        };
        let is_tls = scheme == "https" || scheme == "wss";

        let (host_str, path_and_query) = match rest.find('/') {
            Some(pos) => (&rest[..pos], &rest[pos..]),
            None => (rest, "/"),
        };

        let (host, port) = if let Some(colon_pos) = host_str.rfind(':') {
            let h = &host_str[..colon_pos];
            let p_str = &host_str[colon_pos + 1..];
            let p: u16 = p_str
                .parse()
                .map_err(|_| format!("invalid port in url: {input}"))?;
            (h.to_string(), p)
        } else {
            (host_str.to_string(), default_port)
        };

        if host.is_empty() {
            return Err(format!("empty host in url: {input}"));
        }

        let (path, query) = if let Some(qmark) = path_and_query.find('?') {
            let p = if qmark == 0 { "" } else { &path_and_query[..qmark] };
            let p = if p.is_empty() { "/" } else { p };
            (p.to_string(), path_and_query[qmark..].to_string())
        } else {
            let p = if path_and_query.is_empty() { "/" } else { path_and_query };
            (p.to_string(), String::new())
        };

        Ok(Url {
            scheme: if is_tls { "https".into() } else { "http".into() },
            host,
            port,
            path,
            query,
            full: input.to_string(),
        })
    }

    pub fn origin(&self) -> String {
        if self.port == 80 || self.port == 443 {
            format!("{}://{}", self.scheme, self.host)
        } else {
            format!("{}://{}:{}", self.scheme, self.host, self.port)
        }
    }

    pub fn is_tls(&self) -> bool {
        self.port == 443 || self.scheme == "https"
    }

    pub fn request_target(&self) -> String {
        if self.query.is_empty() {
            self.path.clone()
        } else {
            format!("{}?{}", self.path, self.query)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_https() {
        let u = Url::parse("https://example.com/path?a=1").unwrap();
        assert_eq!(u.scheme, "https");
        assert_eq!(u.host, "example.com");
        assert_eq!(u.port, 443);
        assert_eq!(u.path, "/path");
        assert_eq!(u.query, "?a=1");
        assert!(u.is_tls());
    }

    #[test]
    fn test_parse_http_with_port() {
        let u = Url::parse("http://localhost:8080/api").unwrap();
        assert_eq!(u.scheme, "http");
        assert_eq!(u.host, "localhost");
        assert_eq!(u.port, 8080);
        assert_eq!(u.path, "/api");
        assert!(!u.is_tls());
    }

    #[test]
    fn test_parse_ws() {
        let u = Url::parse("ws://echo.example.com/chat").unwrap();
        assert_eq!(u.host, "echo.example.com");
        assert_eq!(u.port, 80);
    }

    #[test]
    fn test_parse_root() {
        let u = Url::parse("http://example.com").unwrap();
        assert_eq!(u.path, "/");
        assert_eq!(u.query, "");
    }

    #[test]
    fn test_parse_no_scheme() {
        assert!(Url::parse("example.com/path").is_err());
    }

    #[test]
    fn test_parse_bad_port() {
        assert!(Url::parse("http://example.com:abc/path").is_err());
    }

    #[test]
    fn test_parse_empty_host() {
        assert!(Url::parse("http:///path").is_err());
    }
}
