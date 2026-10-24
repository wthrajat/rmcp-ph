//! URL patterns with JavaScript's character classes spelled out: Rust's `\s`
//! is Unicode-aware, JavaScript's adds U+FEFF and omits U+0085.

use std::sync::LazyLock;

use regex::Regex;

pub(crate) const JS_SPACE: &str = r"\t\n\x0B\x0C\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}";

const SCHEME: &str = r"[A-Za-z][A-Za-z0-9+.-]{0,63}";

pub(crate) static URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r#"{SCHEME}:[^{JS_SPACE}<>"]+"#)).expect("url pattern"));

pub(crate) static AUTHORITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("{SCHEME}://")).expect("authority pattern"));

pub(crate) static AUTHORITY_AT_START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("^{SCHEME}://")).expect("leading authority pattern"));

pub(crate) fn ends_url(character: char) -> bool {
    matches!(character, '<' | '>' | '"') || is_js_space(character)
}

fn is_js_space(character: char) -> bool {
    matches!(
        character,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}
