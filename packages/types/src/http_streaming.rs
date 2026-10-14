//! Public HTTP response streaming shared by event sinks and REST servers.

pub const HTTP_RESPONSE_CHUNK: &str = "http_response_chunk";

/// Streaming requires an explicit, acceptable SSE media type. Wildcards keep
/// the buffered response so ordinary HTTP clients do not opt in accidentally.
pub fn accepts_event_stream(accept: &str) -> bool {
    split_unquoted(accept, ',').any(|range| {
        let mut parts = split_unquoted(range, ';');
        if !parts
            .next()
            .is_some_and(|media| media.trim().eq_ignore_ascii_case("text/event-stream"))
        {
            return false;
        }
        let mut quality = None;
        for parameter in parts {
            let Some((name, value)) = parameter.trim().split_once('=') else {
                if parameter.trim().eq_ignore_ascii_case("q") {
                    return false;
                }
                continue;
            };
            if name.trim().eq_ignore_ascii_case("q") {
                if quality.is_some() {
                    return false;
                }
                let value = value.trim();
                let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
                if !matches!(whole, "0" | "1")
                    || fraction.len() > 3
                    || !fraction.bytes().all(|byte| byte.is_ascii_digit())
                    || (whole == "1" && fraction.bytes().any(|byte| byte != b'0'))
                {
                    return false;
                }
                quality = Some(whole == "1" || fraction.bytes().any(|byte| byte != b'0'));
            }
        }
        quality.unwrap_or(true)
    })
}

fn split_unquoted(value: &str, separator: char) -> impl Iterator<Item = &str> {
    let mut quoted = false;
    let mut escaped = false;
    value.split(move |character| {
        if escaped {
            escaped = false;
            return false;
        }
        if quoted && character == '\\' {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        }
        !quoted && character == separator
    })
}

#[cfg(test)]
mod tests {
    use super::accepts_event_stream;

    #[test]
    fn only_explicit_acceptable_sse_enables_streaming() {
        for accept in [
            "text/event-stream",
            " application/json, TEXT/Event-Stream ; charset=utf-8 ",
            "text/event-stream;q=0.001",
            "text/event-stream;q=1.000",
            "text/event-stream;q=1.",
            "text/event-stream;profile=\"a,b;q=0\"",
        ] {
            assert!(accepts_event_stream(accept), "{accept}");
        }
        for accept in [
            "",
            "*/*",
            "text/*",
            "application/json",
            "application/json;profile=\"text/event-stream\"",
            "text/event-streaming",
            "text/event-stream;q=0",
            "text/event-stream;q=0.000, */*;q=1",
            "text/event-stream;q=0;profile=\"a,b\"",
            "text/event-stream;q=invalid",
            "text/event-stream;q=NaN",
            "text/event-stream;q=-1",
            "text/event-stream;q=1.001",
            "text/event-stream;q=0.0001",
            "text/event-stream;q=1;q=0",
            "text/event-stream;q",
        ] {
            assert!(!accepts_event_stream(accept), "{accept}");
        }
    }
}
