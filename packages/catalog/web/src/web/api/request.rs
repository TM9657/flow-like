use std::collections::HashMap;

pub mod get_header;
pub mod get_headers;
pub mod get_method;
pub mod get_url;
pub mod make;
pub mod set_accept;
pub mod set_bearer_auth;
pub mod set_bytes_body;
pub mod set_content_type;
pub mod set_form_body;
pub mod set_header;
pub mod set_headers;
pub mod set_method;
pub mod set_string_body;
pub mod set_struct_body;
pub mod set_url;

pub(crate) fn encode_form_fields(fields: &HashMap<String, String>) -> String {
    if fields.is_empty() {
        return String::new();
    }

    let mut pairs = fields.iter().collect::<Vec<_>>();
    pairs.sort_by(|left, right| left.0.cmp(right.0));

    pairs
        .into_iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                encode_form_component(key),
                encode_form_component(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn encode_form_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }

    encoded
}
