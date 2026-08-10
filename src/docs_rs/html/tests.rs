//! Unit tests for the security scrub and the rustdoc chrome stripper.

use std::borrow::Cow;

use super::sanitize::{RE_JS_URLS, RE_ON_HANDLERS, scrub_pattern};
use super::{sanitize_html_fragment, scrub_rustdoc_chrome};

const CLEAN_PAGE: &str = r#"<div class="docblock"><p>Hello <a href="/x">x</a></p></div>"#;

#[test]
fn sanitize_is_identity_on_clean_markup() {
    assert_eq!(sanitize_html_fragment(CLEAN_PAGE), CLEAN_PAGE);
}

#[test]
fn sanitize_still_drops_scripts_handlers_and_js_urls() {
    let dirty = r#"<div onclick="boom()"><script>evil()</script><a href="javascript:evil()">x</a><style>p{}</style></div>"#;
    let out = sanitize_html_fragment(dirty);
    assert!(!out.contains("<script"), "script survived: {out}");
    assert!(!out.contains("<style"), "style survived: {out}");
    assert!(!out.contains("onclick"), "handler survived: {out}");
    assert!(!out.contains("javascript:"), "js url survived: {out}");
}

#[test]
fn chrome_scrub_matches_previous_replace_chain() {
    // Reference implementation: the pre-Cow strip order, byte for byte.
    fn reference(md: &str) -> String {
        let stripped = md.replace('§', "").replace("Copy item path", "");
        let mut out = String::with_capacity(stripped.len());
        let mut prev_space = false;
        for c in stripped.chars() {
            if c == ' ' {
                if prev_space {
                    continue;
                }
                prev_space = true;
                out.push(' ');
            } else {
                prev_space = false;
                out.push(c);
            }
        }
        out
    }
    for case in [
        "plain text with no chrome",
        "§Heading Copy item path\n\nbody  with   spaces",
        "Copy§ item path stays fused",
        "",
        "   ",
    ] {
        assert_eq!(
            scrub_rustdoc_chrome(case),
            reference(case),
            "case: {case:?}"
        );
    }
}

/// Measurement harness for the sanitize hot path (not a correctness gate).
///
/// Run with `cargo test --release -- --ignored --nocapture sanitize_timing`.
#[test]
#[ignore = "measurement only; prints timings instead of asserting"]
fn sanitize_timing() {
    use std::time::Instant;

    let unit = r#"<section class="docblock"><h2>Item</h2><p>Some documentation text with <code>inline</code> and <a href="/rel/path">a link</a>.</p></section>"#;
    for target_mib in [1usize, 4, 10] {
        let target = target_mib * 1024 * 1024;
        let mut body = String::with_capacity(target + unit.len());
        while body.len() < target {
            body.push_str(unit);
        }
        // Isolate the scrub chain: the shared `Html::parse_fragment` cost
        // dwarfs it, so comparing whole-function timings hides the delta.
        let legacy_tail = |html: &str| -> String {
            let mut out = html.to_string();
            out = RE_ON_HANDLERS.replace_all(&out, "").into_owned();
            out = RE_JS_URLS.replace_all(&out, "").into_owned();
            out
        };
        let cow_tail = |html: &str| -> String {
            let out = scrub_pattern(&RE_ON_HANDLERS, Cow::Borrowed(html));
            scrub_pattern(&RE_JS_URLS, out).into_owned()
        };
        let started = Instant::now();
        let before = legacy_tail(&body);
        let legacy_elapsed = started.elapsed();
        let started = Instant::now();
        let after = cow_tail(&body);
        let cow_elapsed = started.elapsed();
        assert_eq!(before, after, "scrub chain diverged from the baseline");
        let started = Instant::now();
        let full = sanitize_html_fragment(&body);
        let full_elapsed = started.elapsed();
        assert_eq!(full, after, "sanitize output diverged from the baseline");
        // Same body with one script tag: forces the DOM parse path.
        let mut dirty = String::with_capacity(body.len() + 32);
        dirty.push_str("<script>void 0;</script>");
        dirty.push_str(&body);
        let started = Instant::now();
        let dirty_out = sanitize_html_fragment(&dirty);
        let dirty_elapsed = started.elapsed();
        assert!(!dirty_out.contains("<script"), "script survived the scrub");
        println!(
            "input={target_mib}MiB out_len={} | scrub legacy={legacy_elapsed:?} cow={cow_elapsed:?} | sanitize clean(prescan skip)={full_elapsed:?} dirty(DOM parse)={dirty_elapsed:?}",
            after.len()
        );
    }
}
