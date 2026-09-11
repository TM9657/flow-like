//! Canonical handling of object-store path strings.
//!
//! `Path::from` percent-encodes its input and encodes an already encoded string
//! a second time, while every string that leaves a `Path` (`as_ref`,
//! `filename`, `ObjectMeta::location`, `FlowPath.path`, manifests, list
//! responses) is already encoded. These helpers accept either form: each
//! segment is percent-decoded and then encoded exactly once, so a raw user
//! name and a listed key resolve to the same object.

use object_store::path::Path;
use percent_encoding::percent_decode_str;
use std::borrow::Cow;

/// Percent-decodes one path segment. Bytes that do not form UTF-8 leave the
/// segment unchanged so a malformed `%` sequence is kept literally.
pub fn decode_path_segment(segment: &str) -> Cow<'_, str> {
    percent_decode_str(segment)
        .decode_utf8()
        .unwrap_or(Cow::Borrowed(segment))
}

/// Appends `relative` to `base`, decoding and re-encoding every segment.
///
/// Empty segments are dropped. `.` and `..` become the inert literals that
/// `Path::from` produces for them, so a relative path can never climb above
/// `base`. An encoded delimiter (`%2F`) stays inside its segment.
pub fn join_object_path(base: &Path, relative: &str) -> Path {
    relative
        .split('/')
        .filter(|segment| !segment.is_empty())
        .fold(base.clone(), |acc, segment| {
            acc.join(decode_path_segment(segment).as_ref())
        })
}

/// The canonical `Path` for a string that may be raw or already encoded.
pub fn normalize_object_path(input: &str) -> Path {
    join_object_path(&Path::default(), input)
}

/// Every segment decoded, joined with `/`. For display and download names
/// only; the result must never be used to address an object.
pub fn display_object_path(path: &Path) -> String {
    path.parts()
        .map(|part| decode_path_segment(part.as_ref()).into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// The decoded last segment, for display and download names.
pub fn display_file_name(path: &Path) -> Option<String> {
    path.filename()
        .map(|name| decode_path_segment(name).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: [&str; 5] = [
        "Übersicht (2).pdf",
        "report#2024.pdf",
        "100%.txt",
        "[draft] plan.md",
        "plain.txt",
    ];

    #[test]
    fn raw_and_encoded_input_resolve_to_the_same_key() {
        for raw in NAMES {
            let once = Path::from(raw);
            assert_eq!(normalize_object_path(raw), once, "raw {raw}");
            assert_eq!(normalize_object_path(once.as_ref()), once, "encoded {raw}");
            let nested = Path::from("apps").join("x").join("upload").join(raw);
            assert_eq!(join_object_path(&Path::from("apps/x/upload"), raw), nested);
            assert_eq!(
                join_object_path(&Path::from("apps/x/upload"), once.as_ref()),
                nested
            );
            assert_eq!(normalize_object_path(nested.as_ref()), nested);
        }
    }

    #[test]
    fn display_reverses_encoding() {
        for raw in NAMES {
            let path = Path::from("dir").join(raw);
            assert_eq!(display_object_path(&path), format!("dir/{raw}"));
            assert_eq!(display_file_name(&path).as_deref(), Some(raw));
        }
        assert_eq!(display_file_name(&Path::default()), None);
    }

    #[test]
    fn traversal_and_delimiters_stay_inert() {
        assert_eq!(
            join_object_path(&Path::from("apps/a/upload"), "../../etc/passwd").as_ref(),
            "apps/a/upload/%2E%2E/%2E%2E/etc/passwd"
        );
        assert_eq!(
            join_object_path(&Path::from("apps/a/upload"), "%2E%2E/x").as_ref(),
            "apps/a/upload/%2E%2E/x"
        );
        assert_eq!(normalize_object_path("a%2Fb").as_ref(), "a%2Fb");
        assert_eq!(normalize_object_path("//a//b/").as_ref(), "a/b");
        assert_eq!(normalize_object_path("").as_ref(), "");
    }

    #[test]
    fn malformed_percent_sequences_are_kept_literally() {
        assert_eq!(normalize_object_path("100%.txt").as_ref(), "100%25.txt");
        assert_eq!(normalize_object_path("bad%FF.txt").as_ref(), "bad%25FF.txt");
    }
}
