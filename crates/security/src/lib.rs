use localflow_core::error::{CoreError, CoreResult};
use url::Url;

/// Validates and sanitizes file paths to prevent directory traversal attacks.
pub fn sanitize_path(base_path: &str, user_path: &str) -> CoreResult<String> {
    // First, normalize the path
    let base = std::path::Path::new(base_path)
        .canonicalize()
        .map_err(|e| CoreError::validation(format!("Invalid base path '{base_path}': {e}")))?;

    let combined = base.join(user_path);
    let canonical = combined
        .canonicalize()
        .map_err(|e| CoreError::validation(format!("Invalid path '{user_path}': {e}")))?;

    // Ensure the canonical path is within the base path
    if !canonical.starts_with(&base) {
        return Err(CoreError::validation(format!(
            "Path traversal detected: '{user_path}' resolves outside base directory"
        )));
    }

    Ok(canonical.to_string_lossy().to_string())
}

/// Checks whether a URL is allowed to be accessed, enforcing SSRF protection.
///
/// Returns an error if the URL targets:
/// - Private/internal IP ranges (10.x.x.x, 172.16-31.x.x, 192.168.x.x)
/// - Localhost/loopback (127.0.0.1, localhost, ::1)
/// - Cloud metadata endpoints (169.254.x.x)
/// - file:// protocol
pub fn validate_url(
    url_str: &str,
    allowed_hosts: &[String],
    allow_loopback: bool,
) -> CoreResult<Url> {
    let url = Url::parse(url_str)
        .map_err(|e| CoreError::validation(format!("Invalid URL '{url_str}': {e}")))?;

    // Reject file:// protocol
    if url.scheme() == "file" {
        return Err(CoreError::validation(
            "file:// protocol is not allowed for HTTP requests",
        ));
    }

    // Only allow http and https
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(CoreError::validation(format!(
            "Unsupported protocol '{}': only http and https are allowed",
            url.scheme()
        )));
    }

    let host = url.host_str().unwrap_or("");

    // Check allowed_hosts whitelist first (wildcard support: *.example.com)
    if is_host_allowed(host, allowed_hosts) {
        return Ok(url);
    }

    // Check loopback
    if is_loopback(host) {
        if allow_loopback {
            return Ok(url);
        }
        return Err(CoreError::validation(format!(
            "Loopback address '{host}' is not allowed (set allow_loopback=true to enable)"
        )));
    }

    // Check private/internal IP ranges
    if is_private_ip(host) {
        return Err(CoreError::validation(format!(
            "Private/internal IP '{host}' is not allowed"
        )));
    }

    // Check cloud metadata IP
    if is_cloud_metadata_ip(host) {
        return Err(CoreError::validation(format!(
            "Cloud metadata address '{host}' is not allowed"
        )));
    }

    // If no allowlist is configured, allow all non-private URLs
    if allowed_hosts.is_empty() {
        return Ok(url);
    }

    Err(CoreError::validation(format!(
        "Host '{host}' is not in the allowed hosts list"
    )))
}

fn is_host_allowed(host: &str, allowed_hosts: &[String]) -> bool {
    allowed_hosts.iter().any(|pattern| {
        if pattern == "*" {
            return true;
        }
        if let Some(suffix) = pattern.strip_prefix("*.") {
            host == suffix || host.ends_with(&format!(".{suffix}"))
        } else {
            host == pattern
        }
    })
}

fn is_loopback(host: &str) -> bool {
    host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host == "0.0.0.0"
        || host.starts_with("127.")
}

fn is_private_ip(host: &str) -> bool {
    // 10.x.x.x
    if host.starts_with("10.") {
        return true;
    }
    // 172.16.x.x - 172.31.x.x
    if let Some(rest) = host.strip_prefix("172.")
        && let Some(first_octet) = rest.split('.').next()
        && let Ok(n) = first_octet.parse::<u8>()
        && (16..=31).contains(&n)
    {
        return true;
    }
    // 192.168.x.x
    if host.starts_with("192.168.") {
        return true;
    }
    false
}

fn is_cloud_metadata_ip(host: &str) -> bool {
    host.starts_with("169.254.")
}

/// Redacts sensitive information from a string for safe logging.
pub fn redact_sensitive(input: &str) -> String {
    localflow_core::redact_sensitive(input)
}

/// Validates that a string value doesn't exceed a maximum size.
pub fn validate_size(value: &str, max_bytes: usize, field_name: &str) -> CoreResult<()> {
    if value.len() > max_bytes {
        return Err(CoreError::validation(format!(
            "'{field_name}' exceeds maximum size of {max_bytes} bytes (got {})",
            value.len()
        )));
    }
    Ok(())
}

/// Validates that a JSON value doesn't exceed a maximum size.
pub fn validate_json_size(
    value: &serde_json::Value,
    max_bytes: usize,
    field_name: &str,
) -> CoreResult<()> {
    let serialized = serde_json::to_string(value)
        .map_err(|e| CoreError::validation(format!("Failed to serialize '{field_name}': {e}")))?;
    validate_size(&serialized, max_bytes, field_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_https_allowed() {
        let result = validate_url("https://api.openai.com/v1/chat", &[], false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_url_rejects_localhost() {
        let result = validate_url("http://localhost:8080/api", &[], false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Loopback"));
    }

    #[test]
    fn test_validate_url_allows_localhost_with_flag() {
        let result = validate_url("http://localhost:8080/api", &[], true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_url_rejects_private_ip() {
        let result = validate_url("http://192.168.1.1/admin", &[], false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not allowed"));
    }

    #[test]
    fn test_validate_url_rejects_cloud_metadata() {
        let result = validate_url("http://169.254.169.254/latest/meta-data/", &[], false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_rejects_file_protocol() {
        let result = validate_url("file:///etc/passwd", &[], false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_allows_wildcard_host() {
        let result = validate_url("https://api.openai.com/v1", &["*".to_string()], false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_url_with_allowed_hosts() {
        let allowed = vec!["api.openai.com".to_string()];
        let result = validate_url("https://api.openai.com/v1/chat", &allowed, false);
        assert!(result.is_ok());

        let result = validate_url("https://evil.com/hack", &allowed, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_wildcard_subdomain() {
        let allowed = vec!["*.openai.com".to_string()];
        let result = validate_url("https://api.openai.com/v1", &allowed, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_size_exceeds() {
        let result = validate_size("hello", 3, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_size_ok() {
        let result = validate_size("hi", 10, "test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_redact_sensitive() {
        let input = "Authorization: Bearer sk-test-key-12345";
        let result = redact_sensitive(input);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("sk-test-key-12345"));
    }

    #[test]
    fn test_rejects_10_dot() {
        let result = validate_url("http://10.0.0.1/api", &[], false);
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_172_16_31() {
        let result = validate_url("http://172.20.0.1/api", &[], false);
        assert!(result.is_err());
    }

    #[test]
    fn test_allows_public_ip() {
        let result = validate_url("https://8.8.8.8", &[], false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rejects_unsupported_protocol() {
        let result = validate_url("ftp://files.example.com", &[], false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_rejects_0_0_0_0() {
        let result = validate_url("http://0.0.0.0:8080", &[], false);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_traversal_detection() {
        if std::path::Path::new("/tmp").exists() {
            let result = sanitize_path("/tmp", "../etc/passwd");
            assert!(result.is_err() || !result.unwrap().contains("/etc/passwd"));
        }
    }
}
