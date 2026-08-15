use url::Url;

use crate::error::{AppError, AppResult};

pub fn dsh_url(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("http://[{host}]:{port}")
    } else {
        format!("http://{host}:{port}")
    }
}

pub fn normalize_dsh_url(input: &str) -> AppResult<String> {
    let mut url = Url::parse(input.trim())
        .map_err(|error| AppError::new("window.error.invalidUrl").technical(error.to_string()))?;

    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(AppError::new("window.error.unsupportedUrl"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::new("window.error.urlCredentials"));
    }

    url.set_fragment(None);
    if (url.scheme() == "http" && url.port() == Some(80))
        || (url.scheme() == "https" && url.port() == Some(443))
    {
        let _ = url.set_port(None);
    }
    if url.path().is_empty() {
        url.set_path("/");
    }
    if url.path() == "/" {
        url.set_path("");
    } else {
        let path = url.path().trim_end_matches('/').to_owned();
        url.set_path(&path);
    }

    Ok(url.to_string().trim_end_matches('/').to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_equivalent_url_forms() {
        assert_eq!(
            normalize_dsh_url(" HTTP://Example.COM:80/ ").unwrap(),
            "http://example.com"
        );
        assert_eq!(
            normalize_dsh_url("https://example.com/workspace///").unwrap(),
            "https://example.com/workspace"
        );
    }

    #[test]
    fn keeps_loopback_host_forms_distinct() {
        assert_ne!(
            normalize_dsh_url("http://localhost:3080").unwrap(),
            normalize_dsh_url("http://127.0.0.1:3080").unwrap()
        );
    }

    #[test]
    fn rejects_non_http_urls_and_credentials() {
        assert!(normalize_dsh_url("file:///tmp/index.html").is_err());
        assert!(normalize_dsh_url("http://user@example.com").is_err());
    }
}
