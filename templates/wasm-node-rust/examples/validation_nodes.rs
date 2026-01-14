//! Data Validation Nodes - Validate and sanitize input data
//!
//! This example demonstrates validation nodes for common data types
//! including email, URL, phone numbers, and custom patterns.

use flow_like_wasm_sdk::*;

package! {
    nodes: [
        {
            name: "validate_email",
            friendly_name: "Validate Email",
            description: "Validates an email address format",
            category: "Validation/Format",
            inputs: {
                exec: Exec,
                email: String = "",
            },
            outputs: {
                exec_out: Exec,
                is_valid: Bool,
                normalized: String,
                domain: String,
            },
        },
        {
            name: "validate_url",
            friendly_name: "Validate URL",
            description: "Validates a URL format",
            category: "Validation/Format",
            inputs: {
                exec: Exec,
                url: String = "",
                require_https: Bool = false,
            },
            outputs: {
                exec_out: Exec,
                is_valid: Bool,
                protocol: String,
                host: String,
            },
        },
        {
            name: "validate_number_range",
            friendly_name: "Validate Number Range",
            description: "Checks if a number is within a range",
            category: "Validation/Range",
            inputs: {
                exec: Exec,
                value: F64 = 0.0,
                min: F64 = 0.0,
                max: F64 = 100.0,
                inclusive: Bool = true,
            },
            outputs: {
                exec_out: Exec,
                is_valid: Bool,
                clamped: F64,
            },
        },
        {
            name: "validate_string_length",
            friendly_name: "Validate String Length",
            description: "Checks if string length is within bounds",
            category: "Validation/Length",
            inputs: {
                exec: Exec,
                text: String = "",
                min_length: I64 = 0,
                max_length: I64 = 255,
            },
            outputs: {
                exec_out: Exec,
                is_valid: Bool,
                length: I64,
                truncated: String,
            },
        },
        {
            name: "validate_not_empty",
            friendly_name: "Validate Not Empty",
            description: "Checks if a value is not empty or whitespace-only",
            category: "Validation/Basic",
            inputs: {
                exec: Exec,
                value: String = "",
                trim_whitespace: Bool = true,
            },
            outputs: {
                exec_out: Exec,
                is_valid: Bool,
                trimmed: String,
            },
        },
        {
            name: "sanitize_html",
            friendly_name: "Sanitize HTML",
            description: "Removes potentially dangerous HTML tags",
            category: "Validation/Sanitize",
            inputs: {
                exec: Exec,
                html: String = "",
            },
            outputs: {
                exec_out: Exec,
                result: String,
                removed_count: I64,
            },
        }
    ]
}

run_package!(run_node);

fn run_node(node_name: &str, mut ctx: Context) -> ExecutionResult {
    match node_name {
        "validate_email" => {
            let email = ctx.get_string("email").unwrap_or_default();
            let (is_valid, normalized, domain) = validate_email(&email);
            ctx.set_output("is_valid", is_valid);
            ctx.set_output("normalized", normalized);
            ctx.set_output("domain", domain);
            ctx.success()
        }
        "validate_url" => {
            let url = ctx.get_string("url").unwrap_or_default();
            let require_https = ctx.get_bool("require_https").unwrap_or(false);
            let (is_valid, protocol, host) = validate_url(&url, require_https);
            ctx.set_output("is_valid", is_valid);
            ctx.set_output("protocol", protocol);
            ctx.set_output("host", host);
            ctx.success()
        }
        "validate_number_range" => {
            let value = ctx.get_f64("value").unwrap_or(0.0);
            let min = ctx.get_f64("min").unwrap_or(0.0);
            let max = ctx.get_f64("max").unwrap_or(100.0);
            let inclusive = ctx.get_bool("inclusive").unwrap_or(true);

            let is_valid = if inclusive {
                value >= min && value <= max
            } else {
                value > min && value < max
            };
            let clamped = value.max(min).min(max);

            ctx.set_output("is_valid", is_valid);
            ctx.set_output("clamped", clamped);
            ctx.success()
        }
        "validate_string_length" => {
            let text = ctx.get_string("text").unwrap_or_default();
            let min_length = ctx.get_i64("min_length").unwrap_or(0) as usize;
            let max_length = ctx.get_i64("max_length").unwrap_or(255) as usize;

            let length = text.len();
            let is_valid = length >= min_length && length <= max_length;
            let truncated = if length > max_length {
                text.chars().take(max_length).collect()
            } else {
                text
            };

            ctx.set_output("is_valid", is_valid);
            ctx.set_output("length", length as i64);
            ctx.set_output("truncated", truncated);
            ctx.success()
        }
        "validate_not_empty" => {
            let value = ctx.get_string("value").unwrap_or_default();
            let trim_whitespace = ctx.get_bool("trim_whitespace").unwrap_or(true);

            let trimmed = if trim_whitespace {
                value.trim().to_string()
            } else {
                value
            };
            let is_valid = !trimmed.is_empty();

            ctx.set_output("is_valid", is_valid);
            ctx.set_output("trimmed", trimmed);
            ctx.success()
        }
        "sanitize_html" => {
            let html = ctx.get_string("html").unwrap_or_default();
            let (result, removed_count) = sanitize_html(&html);
            ctx.set_output("result", result);
            ctx.set_output("removed_count", removed_count);
            ctx.success()
        }
        _ => ctx.fail(format!("Unknown node: {}", node_name)),
    }
}

// ============================================================================
// Validation Helpers
// ============================================================================

fn validate_email(email: &str) -> (bool, String, String) {
    let normalized = email.trim().to_lowercase();

    // Simple email validation
    let parts: Vec<&str> = normalized.split('@').collect();
    if parts.len() != 2 {
        return (false, normalized, String::new());
    }

    let local = parts[0];
    let domain = parts[1];

    // Basic checks
    if local.is_empty() || domain.is_empty() {
        return (false, normalized, String::new());
    }

    if !domain.contains('.') {
        return (false, normalized, String::new());
    }

    // Check for invalid characters (simplified)
    let valid_chars = |c: char| c.is_alphanumeric() || ".+-_".contains(c);
    if !local.chars().all(valid_chars) {
        return (false, normalized, String::new());
    }

    let domain_owned = domain.to_string();
    (true, normalized, domain_owned)
}

fn validate_url(url: &str, require_https: bool) -> (bool, String, String) {
    let url = url.trim();

    // Extract protocol
    let (protocol, rest) = if let Some(idx) = url.find("://") {
        (&url[..idx], &url[idx + 3..])
    } else {
        return (false, String::new(), String::new());
    };

    // Check protocol
    if require_https && protocol != "https" {
        return (false, protocol.to_string(), String::new());
    }

    if protocol != "http" && protocol != "https" {
        return (false, protocol.to_string(), String::new());
    }

    // Extract host
    let host = rest.split('/').next().unwrap_or("");
    let host = host.split('?').next().unwrap_or(host);
    let host = host.split('#').next().unwrap_or(host);

    if host.is_empty() {
        return (false, protocol.to_string(), String::new());
    }

    (true, protocol.to_string(), host.to_string())
}

fn sanitize_html(html: &str) -> (String, i64) {
    let dangerous_patterns = ["<script", "</script>", "<iframe", "</iframe>", "javascript:", "onerror=", "onclick=", "onload="];
    let mut result = html.to_string();
    let mut count = 0i64;

    for pattern in dangerous_patterns {
        while result.to_lowercase().contains(&pattern.to_lowercase()) {
            result = result.replacen(pattern, "", 1);
            count += 1;
        }
    }

    (result, count)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email_valid() {
        let (valid, _, domain) = validate_email("user@example.com");
        assert!(valid);
        assert_eq!(domain, "example.com");
    }

    #[test]
    fn test_validate_email_invalid() {
        let (valid, _, _) = validate_email("invalid-email");
        assert!(!valid);

        let (valid, _, _) = validate_email("@nodomain.com");
        assert!(!valid);
    }

    #[test]
    fn test_validate_url() {
        let (valid, proto, host) = validate_url("https://example.com/path", false);
        assert!(valid);
        assert_eq!(proto, "https");
        assert_eq!(host, "example.com");
    }

    #[test]
    fn test_sanitize_html() {
        let (result, count) = sanitize_html("<p>Hello</p><script>evil()</script>");
        assert!(!result.contains("<script"));
        assert!(count > 0);
    }
}
