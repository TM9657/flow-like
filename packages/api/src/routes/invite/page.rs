pub(super) struct InvitationPage {
    pub name: String,
    pub description: String,
    pub image: Option<String>,
    pub icon: Option<String>,
    pub desktop_url: String,
    pub web_url: String,
    pub share_url: String,
}

// Keep the script static so the response can authorize its exact CSP hash.
pub(super) const SCRIPT: &str = r#"(() => {
  document.querySelectorAll('img').forEach((image) => {
    const hideBrokenImage = () => { image.hidden = true; };
    image.addEventListener('error', hideBrokenImage);
    if (image.complete && image.naturalWidth === 0) hideBrokenImage();
  });
  const desktop = document.getElementById('open-desktop');
  const status = document.getElementById('handoff-status');
  if (!desktop || !status) return;
  let leftPage = false;
  let interacted = false;
  document.addEventListener('pointerdown', () => { interacted = true; }, { once: true });
  document.addEventListener('keydown', () => { interacted = true; }, { once: true });
  window.addEventListener('pagehide', () => { leftPage = true; });
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') leftPage = true;
  });
  const launch = () => {
    status.textContent = 'Opening Flow-Like…';
    window.setTimeout(() => {
      if (!leftPage && document.visibilityState === 'visible') {
        status.textContent = 'App didn’t open? Try the button above, or continue in your browser.';
      }
    }, 1800);
  };
  desktop.addEventListener('click', () => { leftPage = false; launch(); });
  // Opening an external app may require a click. Both links remain usable.
  window.setTimeout(() => {
    if (interacted || leftPage || document.visibilityState !== 'visible') return;
    launch();
    try { window.location.href = desktop.href; } catch (_) {}
  }, 600);
})();"#;

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn document(title: &str, description: &str, head: &str, body: &str, script: bool) -> String {
    let title = escape(title);
    let description = escape(description);
    let style = include_str!("style.css");
    let script = if script {
        format!("<script>{SCRIPT}</script>")
    } else {
        String::new()
    };
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark">
<meta name="theme-color" content="#17181d">
<meta name="robots" content="noindex, nofollow, noarchive">
<meta name="referrer" content="no-referrer">
<title>{title}</title>
<meta name="description" content="{description}">
<meta property="og:type" content="website">
<meta property="og:site_name" content="Flow-Like">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{description}">
<meta name="twitter:title" content="{title}">
<meta name="twitter:description" content="{description}">
{head}
<style>{style}</style>
</head>
<body>
<div class="shell">
<header class="brand">Flow-Like<span class="brand-caption">Project invitation</span></header>
<main>{body}</main>
<footer>Shared with you on Flow-Like.</footer>
</div>
{script}
</body>
</html>"##
    )
}

pub(super) fn render_invitation(page: &InvitationPage) -> String {
    let name = escape(&page.name);
    let description = escape(&page.description);
    let desktop_url = escape(&page.desktop_url);
    let web_url = escape(&page.web_url);
    let share_url = escape(&page.share_url);
    let mut head = format!(
        r#"<link rel="canonical" href="{share_url}">
<meta property="og:url" content="{share_url}">"#
    );
    let cover = if let Some(image) = &page.image {
        let image = escape(image);
        head.push_str(&format!(
            r#"
<meta property="og:image" content="{image}">
<meta property="og:image:alt" content="Cover for {name}">
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:image" content="{image}">
<meta name="twitter:image:alt" content="Cover for {name}">"#
        ));
        format!(r#"<img class="cover-image" src="{image}" alt="" referrerpolicy="no-referrer">"#)
    } else {
        head.push_str("\n<meta name=\"twitter:card\" content=\"summary\">");
        String::new()
    };
    let icon = page.icon.as_ref().map_or_else(String::new, |icon| {
        format!(
            r#"<img class="project-icon" src="{}" alt="" width="56" height="56" referrerpolicy="no-referrer">"#,
            escape(icon)
        )
    });
    let body = format!(
        r#"<article class="invitation" aria-labelledby="project-name">
<div class="visual" aria-hidden="true">
{cover}
<div class="visual-shade"></div>
<div class="visual-caption"><span class="eyebrow">Shared with you on Flow-Like</span></div>
</div>
<div class="details">
<div class="project-heading">{icon}<span class="eyebrow">You’re invited</span></div>
<h1 id="project-name">{name}</h1>
<p class="description">{description}</p>
<div class="actions">
<p class="action-intro">Choose where to open your invitation.</p>
<a class="button primary" id="open-desktop" href="{desktop_url}" rel="noreferrer">Open in Flow-Like</a>
<a class="button secondary" href="{web_url}" rel="noreferrer">Continue in browser</a>
<p class="hint" id="handoff-status" role="status" aria-live="polite">Use the desktop app, or join from your browser.</p>
</div>
</div>
</article>"#
    );
    document(
        &format!("You’re invited to {}", page.name),
        &format!("Join {} on Flow-Like. {}", page.name, page.description),
        &head,
        &body,
        true,
    )
}

pub(super) fn render_unavailable(temporary: bool) -> String {
    let (title, description, hint) = if temporary {
        (
            "Invitation unavailable",
            "We couldn’t check this invitation right now.",
            "Please try opening the link again in a moment.",
        )
    } else {
        (
            "Invalid invitation",
            "This link is invalid or no longer available.",
            "Ask the person who sent it for a new invitation.",
        )
    };
    let body = format!(
        r#"<article class="unavailable" aria-labelledby="invitation-title">
<span class="eyebrow">Flow-Like invitation</span>
<h1 id="invitation-title">{title}</h1>
<p class="description">{description}</p>
<p class="hint">{hint}</p>
</article>"#
    );
    document(
        title,
        description,
        "<meta name=\"twitter:card\" content=\"summary\">",
        &body,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untrusted_metadata_is_text_in_both_head_and_body() {
        let page = InvitationPage {
            name: "<script>alert('name')</script>".into(),
            description: "\" onload=\"alert('description')".into(),
            image: Some("https://images.example/cover?x=\"/><script>evil()</script>".into()),
            icon: None,
            desktop_url: "flow-like://join?appId=a&token=t".into(),
            web_url: "https://app.example/join?appId=a&token=t&web=1".into(),
            share_url: "https://api.example/join?appId=a&token=t".into(),
        };
        let html = render_invitation(&page);
        assert!(!html.contains("<script>alert"));
        assert!(!html.contains("<script>evil"));
        assert!(!html.contains("\" onload=\""));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("appId=a&amp;token=t&amp;web=1"));
        assert_eq!(html.matches("<script>").count(), 1);
        assert!(html.contains(&format!("<script>{SCRIPT}</script>")));
    }

    #[test]
    fn unavailable_pages_have_no_images_links_or_launch_script() {
        for temporary in [true, false] {
            let html = render_unavailable(temporary);
            assert!(!html.contains("og:image"));
            assert!(!html.contains("<img"));
            assert!(!html.contains("href="));
            assert!(!html.contains("<script>"));
            assert!(!html.contains("flow-like://"));
        }
    }
}
