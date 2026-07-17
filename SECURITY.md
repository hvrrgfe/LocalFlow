# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, report them privately by emailing the project maintainers. If you do not have an email contact, please open a GitHub issue with the label `security` and mark the issue as confidential if the feature is available.

## Security Design

LocalFlow is designed with security as a first-class concern:

- **No cloud dependency** - Everything runs locally on your machine
- **Secrets encryption** - API keys stored in encrypted vault (AES-256-GCM) or OS keychain
- **SSRF protection** - All outbound HTTP requests validated against private IPs, localhost, and cloud metadata endpoints
- **Log redaction** - Sensitive fields (Authorization, Bearer, api_key, token, cookie, password) automatically redacted from logs
- **Path traversal protection** - All file paths normalized and validated
- **No remote code execution** - No shell, Python, or JavaScript execution nodes by default
- **Permission system** - All dangerous operations disabled by default

## Security Checklist for Contributors

1. Never log raw API keys or secrets
2. Always validate URLs before making HTTP requests
3. Always sanitize file paths to prevent traversal
4. Never expose internal error details to the frontend
5. Always validate input sizes and bounds
6. Never make security decisions based on external API responses
