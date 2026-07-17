use std::io;

use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Initializes structured logging with JSON format and environment-based filtering.
///
/// Reads `LOCALFLOW_LOG` env var for filter directives (default: "info").
/// When `LOCALFLOW_LOG_FORMAT` is "json", outputs JSON-formatted logs.
pub fn init_logging() {
    let filter =
        EnvFilter::try_from_env("LOCALFLOW_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let is_json = std::env::var("LOCALFLOW_LOG_FORMAT")
        .map(|v| v == "json")
        .unwrap_or(false);

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(true);

    if is_json {
        let json_layer = fmt_layer.json().flatten_event(true);
        tracing_subscriber::registry()
            .with(filter)
            .with(json_layer)
            .init();
    } else {
        let plain_layer = fmt_layer.with_writer(io::stderr).compact();
        tracing_subscriber::registry()
            .with(filter)
            .with(plain_layer)
            .init();
    }
}

/// Redacts sensitive information from log messages.
///
/// This is a safety net — secrets should never reach the logging system in the first place.
/// Patterns redacted: Authorization headers, Bearer tokens, api_key values, tokens, cookies, passwords.
pub fn redact_sensitive(input: &str) -> String {
    let patterns = [
        (
            r#"(?i)(Authorization:\s*(Bearer\s+)?)[A-Za-z0-9\-._~+/]+=*"#,
            "${1}[REDACTED]",
        ),
        (
            r#"(?i)(api[\s_]?key["\s:=]+\s*)[A-Za-z0-9\-._~+/]{8,}"#,
            "${1}[REDACTED]",
        ),
        (
            r#"(?i)(token["\s:=]+\s*)[A-Za-z0-9\-._~+/]{8,}"#,
            "${1}[REDACTED]",
        ),
        (
            r#"(?i)(cookie["\s:=]+\s*)[A-Za-z0-9\-._~+/]{8,}"#,
            "${1}[REDACTED]",
        ),
        (
            r#"(?i)(password["\s:=]+\s*)[A-Za-z0-9\-._~+/]{8,}"#,
            "${1}[REDACTED]",
        ),
        (
            r#"(?i)(secret["\s:=]+\s*)[A-Za-z0-9\-._~+/]{8,}"#,
            "${1}[REDACTED]",
        ),
        (
            r#"(?i)(x-api-key["\s:=]+\s*)[A-Za-z0-9\-._~+/]+"#,
            "${1}[REDACTED]",
        ),
    ];

    let mut result = input.to_string();
    for (pattern, replacement) in &patterns {
        let re = regex::Regex::new(pattern).expect("Invalid regex pattern");
        result = re.replace_all(&result, *replacement).to_string();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_authorization() {
        let input = "Authorization: Bearer sk-1234567890abcdef";
        let result = redact_sensitive(input);
        assert!(!result.contains("sk-1234567890abcdef"));
        assert!(result.contains("Bearer [REDACTED]"));
    }

    #[test]
    fn test_redact_api_key() {
        let input = r#"{"api_key": "sk-proj-1234567890abcdef"}"#;
        let result = redact_sensitive(input);
        assert!(!result.contains("sk-proj-1234567890abcdef"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_token() {
        let input = "token = \"ghp_1234567890abcdef\"";
        let result = redact_sensitive(input);
        assert!(!result.contains("ghp_1234567890abcdef"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_password() {
        let input = "password = \"hunter23\"";
        let result = redact_sensitive(input);
        assert!(!result.contains("hunter23"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn test_no_false_positive_short_strings() {
        let input = "token = \"abc\"";
        let result = redact_sensitive(input);
        // Short strings (< 8 chars) should not be redacted to avoid noise
        assert_eq!(result, input);
    }

    #[test]
    fn test_redact_x_api_key_header() {
        let input = "x-api-key: a1b2c3d4e5f6g7h8i9j0";
        let result = redact_sensitive(input);
        assert!(!result.contains("a1b2c3d4e5f6g7h8i9j0"));
        assert!(result.contains("[REDACTED]"));
    }
}
