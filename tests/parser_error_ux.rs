//! Parse-error UX — errors must be errors, never panics.
//!
//! `Token::human` renders the offending token inside every parse-error
//! message (via `token_or_end`). String tokens longer than 24 bytes get
//! truncated with an ellipsis so a pasted document doesn't flood the
//! error. The old truncation byte-sliced (`&s[..24]`) without a
//! char-boundary check, so any parse error whose nearby token was a
//! long string literal containing multibyte UTF-8 straddling byte 24
//! panicked the handler instead of returning the error. These tests
//! pin the contract: malformed statements return `Err`, whatever the
//! literal's encoding.

use gigi::parser::parse;

/// A string literal where a bundle name is expected forces the parser
/// to render the token in the error message. Both literals are >24
/// bytes with a multibyte char straddling byte 24:
///   - `"x" + "я"×20` — 41 bytes; after the 1-byte prefix every char
///     boundary is odd, so byte 24 lands mid-char.
///   - `"x" + "🦀"×6` — 25 bytes; the last crab spans bytes 21..25,
///     so byte 24 lands mid-char.
/// Under the old byte-slice truncation both panicked; the contract is
/// a clean parse error with the truncated literal and ellipsis marker.
#[test]
fn long_multibyte_string_literal_in_error_path_returns_err_not_panic() {
    let cyrillic = format!("x{}", "я".repeat(20));
    let crabs = format!("x{}", "🦀".repeat(6));
    for lit in [cyrillic, crabs] {
        let stmt = format!("COVER '{lit}' ALL;");
        let err = parse(&stmt).expect_err(
            "a string literal where a bundle name is expected must be a \
             parse error, not an accepted statement",
        );
        assert!(
            err.contains("string"),
            "error must render the offending token kind: {err}"
        );
        assert!(
            err.contains('…'),
            "long literals must render truncated with the ellipsis \
             marker: {err}"
        );
    }
}

/// Behavior-preservation fence: ASCII literals keep truncating at
/// exactly 24 chars + ellipsis, byte-for-byte what the old code
/// produced. The multibyte fix must not shift the ASCII rendering.
#[test]
fn long_ascii_string_literal_truncates_at_24_chars() {
    let lit = "a".repeat(40);
    let stmt = format!("COVER '{lit}' ALL;");
    let err = parse(&stmt).expect_err("string literal is not a bundle name");
    assert!(
        err.contains(&format!("string '{}…'", "a".repeat(24))),
        "ASCII truncation must stay at exactly 24 chars: {err}"
    );
}

/// Short multibyte literals (≤24 bytes) render whole — no truncation,
/// no ellipsis, no panic.
#[test]
fn short_multibyte_string_literal_renders_whole() {
    let stmt = "COVER 'привет' ALL;";
    let err = parse(stmt).expect_err("string literal is not a bundle name");
    assert!(
        err.contains("string 'привет'"),
        "short literals render untruncated: {err}"
    );
}
