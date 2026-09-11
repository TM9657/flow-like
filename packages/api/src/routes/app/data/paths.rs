//! Resolves client-supplied storage prefixes to keys inside the caller's own
//! namespace.
//!
//! Client input is never taken as a key. The authorized base is always
//! prepended, and every segment is percent-decoded and encoded exactly once,
//! so a raw file name and the encoded `location` a listing returned resolve to
//! the same object, while `.`, `..` and `/` inside a segment can never walk
//! out of the app's namespace.
//!
//! Older clients stored the raw object-store key in widget definitions, and a
//! fork copies those values verbatim, so a key naming *another* app still has to
//! resolve against the caller's own base. Only the exact storage layout is read
//! that way — `apps/<id>/upload[/rest]` and `users/<sub>/apps/<id>[/rest]` —
//! so a folder a user happens to name `apps` stays a plain relative prefix.

use flow_like_storage::{Path, join_object_path};

pub fn app_upload_base(app_id: &str) -> Path {
    Path::from("apps").join(app_id).join("upload")
}

pub fn user_upload_base(sub: &str, app_id: &str) -> Path {
    Path::from("users").join(sub).join("apps").join(app_id)
}

pub fn resolve_app_upload(app_id: &str, prefix: &str) -> Path {
    let relative = strip_app_layout(prefix).unwrap_or(prefix);
    join_object_path(&app_upload_base(app_id), relative)
}

pub fn resolve_user_upload(sub: &str, app_id: &str, prefix: &str) -> Path {
    let relative = strip_user_layout(prefix).unwrap_or(prefix);
    join_object_path(&user_upload_base(sub, app_id), relative)
}

fn strip_app_layout(prefix: &str) -> Option<&str> {
    let rest = prefix.strip_prefix("apps/")?;
    let (_app_id, rest) = rest.split_once('/')?;
    strip_segment(rest, "upload")
}

fn strip_user_layout(prefix: &str) -> Option<&str> {
    let rest = prefix.strip_prefix("users/")?;
    let (_sub, rest) = rest.split_once('/')?;
    let rest = rest.strip_prefix("apps/")?;
    let (_app_id, rest) = rest.split_once('/').unwrap_or((rest, ""));
    Some(rest)
}

fn strip_segment<'a>(rest: &'a str, segment: &str) -> Option<&'a str> {
    match rest.strip_prefix(segment)? {
        "" => Some(""),
        tail => tail.strip_prefix('/'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_storage::display_file_name;

    const APP: &str = "app-1";
    const SUB: &str = "sub-1";

    #[test]
    fn relative_prefixes_land_under_the_app_base() {
        assert_eq!(
            resolve_app_upload(APP, "media/logo.jpg").as_ref(),
            "apps/app-1/upload/media/logo.jpg"
        );
        assert_eq!(resolve_app_upload(APP, "").as_ref(), "apps/app-1/upload");
        assert_eq!(
            resolve_app_upload(APP, "//media//logo.jpg/").as_ref(),
            "apps/app-1/upload/media/logo.jpg"
        );
    }

    #[test]
    fn legacy_object_store_keys_resolve_against_the_callers_own_base() {
        assert_eq!(
            resolve_app_upload(APP, "apps/app-1/upload/media/logo.jpg").as_ref(),
            "apps/app-1/upload/media/logo.jpg"
        );
        // A fork copies widget values verbatim, so a key naming the source app
        // still has to resolve inside the fork.
        assert_eq!(
            resolve_app_upload(APP, "apps/source-app/upload/media/logo.jpg").as_ref(),
            "apps/app-1/upload/media/logo.jpg"
        );
        assert_eq!(
            resolve_app_upload(APP, "apps/app-1/upload").as_ref(),
            "apps/app-1/upload"
        );
    }

    #[test]
    fn folders_named_like_the_layout_stay_relative() {
        assert_eq!(
            resolve_app_upload(APP, "apps/logo.jpg").as_ref(),
            "apps/app-1/upload/apps/logo.jpg"
        );
        assert_eq!(
            resolve_app_upload(APP, "apps/nested/deeper/logo.jpg").as_ref(),
            "apps/app-1/upload/apps/nested/deeper/logo.jpg"
        );
        assert_eq!(
            resolve_app_upload(APP, "apps/x/uploads/logo.jpg").as_ref(),
            "apps/app-1/upload/apps/x/uploads/logo.jpg"
        );
        assert_eq!(
            resolve_user_upload(SUB, APP, "users/notes.txt").as_ref(),
            "users/sub-1/apps/app-1/users/notes.txt"
        );
    }

    #[test]
    fn traversal_attempts_cannot_escape_the_base() {
        assert_eq!(
            resolve_app_upload(APP, "../../secrets.env").as_ref(),
            "apps/app-1/upload/%2E%2E/%2E%2E/secrets.env"
        );
        assert_eq!(
            resolve_app_upload(APP, "apps/other/upload/../../../etc/passwd").as_ref(),
            "apps/app-1/upload/%2E%2E/%2E%2E/%2E%2E/etc/passwd"
        );
        assert_eq!(
            resolve_app_upload(APP, "%2E%2E/secrets.env").as_ref(),
            "apps/app-1/upload/%2E%2E/secrets.env"
        );
        // An encoded delimiter is decoded and re-encoded inside its segment,
        // so it cannot inject a new path segment either.
        assert_eq!(
            resolve_app_upload(APP, "a%2Fb").as_ref(),
            "apps/app-1/upload/a%2Fb"
        );
        assert_eq!(resolve_app_upload(APP, "a%2Fb").parts().count(), 4);
    }

    #[test]
    fn raw_and_listed_names_resolve_to_the_same_key() {
        const NAME: &str = "Übersicht (2)#1.pdf";
        const ENCODED: &str = "%C3%9Cbersicht (2)%231.pdf";
        let listed = Path::from(NAME);
        assert_eq!(listed.as_ref(), ENCODED);

        let app_key = resolve_app_upload(APP, NAME);
        assert_eq!(app_key.as_ref(), format!("apps/app-1/upload/{ENCODED}"));
        assert_eq!(resolve_app_upload(APP, listed.as_ref()), app_key);
        assert_eq!(resolve_app_upload(APP, app_key.as_ref()), app_key);
        assert_eq!(
            resolve_app_upload(APP, &format!("media/{NAME}")),
            resolve_app_upload(APP, &format!("media/{ENCODED}"))
        );
        assert_eq!(display_file_name(&app_key).as_deref(), Some(NAME));

        let user_key = resolve_user_upload(SUB, APP, NAME);
        assert_eq!(
            user_key.as_ref(),
            format!("users/sub-1/apps/app-1/{ENCODED}")
        );
        assert_eq!(resolve_user_upload(SUB, APP, listed.as_ref()), user_key);
        assert_eq!(resolve_user_upload(SUB, APP, user_key.as_ref()), user_key);
        assert_eq!(display_file_name(&user_key).as_deref(), Some(NAME));
    }

    #[test]
    fn user_keys_resolve_against_the_callers_own_sub() {
        assert_eq!(
            resolve_user_upload(SUB, APP, "notes/todo.txt").as_ref(),
            "users/sub-1/apps/app-1/notes/todo.txt"
        );
        assert_eq!(
            resolve_user_upload(SUB, APP, "users/sub-1/apps/app-1/notes/todo.txt").as_ref(),
            "users/sub-1/apps/app-1/notes/todo.txt"
        );
        // Another user's key never reaches their namespace — the caller's own
        // sub is re-imposed.
        assert_eq!(
            resolve_user_upload(SUB, APP, "users/sub-2/apps/app-1/notes/todo.txt").as_ref(),
            "users/sub-1/apps/app-1/notes/todo.txt"
        );
    }
}
