//! Turns remote modpack descriptions into HTML that is safe to render.
//!
//! Descriptions are written by pack authors, so they are untrusted: scripts,
//! frames, styles, event handlers and non-HTTPS URLs are removed here, before
//! the HTML ever reaches the webview.

use std::collections::HashSet;

use ammonia::{Builder, UrlRelative};
use pulldown_cmark::{Options, Parser, html};

/// Renders Modrinth's Markdown body (which may embed raw HTML) and sanitizes it.
pub(crate) fn markdown_to_safe_html(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let mut rendered = String::new();
    html::push_html(&mut rendered, Parser::new_ext(markdown, options));
    sanitize_html(&rendered)
}

/// Keeps ammonia's default allowlist of formatting tags, restricted to HTTPS URLs.
pub(crate) fn sanitize_html(raw: &str) -> String {
    Builder::default()
        .url_schemes(HashSet::from(["https"]))
        .url_relative(UrlRelative::Deny)
        .link_rel(Some("noopener noreferrer"))
        .clean(raw)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown_formatting() {
        let html = markdown_to_safe_html("# Title\n\n**bold** and [site](https://example.com)");
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains(r#"href="https://example.com""#));
        assert!(html.contains(r#"rel="noopener noreferrer""#));
    }

    #[test]
    fn keeps_https_images_embedded_in_markdown() {
        let html = markdown_to_safe_html("![shot](https://cdn.example.com/a.png)");
        assert!(html.contains(r#"src="https://cdn.example.com/a.png""#));
    }

    #[test]
    fn removes_scripts_frames_styles_and_handlers() {
        let html = sanitize_html(
            r#"<p onclick="steal()" style="color:red">hi</p><script>alert(1)</script><iframe src="https://example.com"></iframe><style>p{}</style>"#,
        );
        assert_eq!(html, "<p>hi</p>");
    }

    #[test]
    fn removes_raw_html_scripts_inside_markdown() {
        let html = markdown_to_safe_html("text\n\n<script>alert(1)</script>\n");
        assert!(!html.contains("script"));
        assert!(!html.contains("alert"));
    }

    #[test]
    fn drops_urls_that_are_not_absolute_https() {
        let html = sanitize_html(
            r#"<a href="javascript:alert(1)">a</a><a href="http://example.com">b</a><a href="/relative">c</a><img src="data:image/png;base64,AAAA">"#,
        );
        assert!(!html.contains("javascript"));
        assert!(!html.contains("http://"));
        assert!(!html.contains("/relative"));
        assert!(!html.contains("data:"));
    }
}
