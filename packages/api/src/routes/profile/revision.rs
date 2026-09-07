use chrono::{DateTime, Duration, FixedOffset};

pub(crate) fn next_profile_revision(
    now: DateTime<FixedOffset>,
    previous: Option<DateTime<FixedOffset>>,
    requested: Option<&str>,
) -> DateTime<FixedOffset> {
    // A client clock can be ahead of the hub. Keep its acknowledged revision
    // so the same upload is not treated as a fresh edit on every sync.
    let previous = previous.and_then(|value| value.checked_add_signed(Duration::milliseconds(1)));
    let requested = requested.and_then(|value| DateTime::parse_from_rfc3339(value).ok());
    previous
        .into_iter()
        .chain(requested)
        .fold(now, std::cmp::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(value: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(value).unwrap()
    }

    #[test]
    fn acknowledges_an_upload_ahead_of_the_server_clock() {
        let now = time("2026-09-05T10:00:00Z");
        let requested = "2026-09-05T10:01:00Z";
        let revision = next_profile_revision(now, Some(now), Some(requested));
        assert_eq!(revision, time(requested));
        assert_eq!(next_profile_revision(now, None, Some(requested)), revision);
    }

    #[test]
    fn later_web_edits_advance_past_an_ahead_of_clock_acknowledgement() {
        let now = time("2026-09-05T10:00:00Z");
        let previous = time("2026-09-05T10:01:00Z");
        assert!(next_profile_revision(now, Some(previous), None) > previous);
    }

    #[test]
    fn normal_and_legacy_uploads_use_the_server_clock() {
        let now = time("2026-09-05T10:01:00Z");
        let previous = time("2026-09-05T10:00:00Z");
        for requested in [None, Some("invalid"), Some("2026-09-05T10:00:30Z")] {
            assert_eq!(next_profile_revision(now, Some(previous), requested), now);
        }
    }
}
