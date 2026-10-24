//! Large base64 payloads, which are replaced whole instead of scanned.

use std::sync::LazyLock;

use regex::Regex;

use super::patterns::JS_SPACE;
use crate::support::js_text::js_len;

const SIZE_GATE: usize = 10_240;

static BASE64: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9+/\n\r]+=*$").expect("base64 pattern"));
static BASE64URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_-]+={0,2}$").expect("base64url pattern"));
static DATA_URL_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"^data:[^,{JS_SPACE}]*;base64,")).expect("data url prefix pattern")
});
static DATA_URL_PAYLOAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9+/_-]+={0,2}$").expect("data url payload pattern"));

pub(super) fn is_binary_blob(text: &str) -> bool {
    if js_len(text) < SIZE_GATE {
        return false;
    }
    let is_base64url = text.contains(['-', '_']) && BASE64URL.is_match(text);
    BASE64.is_match(text) || is_base64_data_url(text) || is_base64url
}

fn is_base64_data_url(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    let Some(prefix) = DATA_URL_PREFIX.find(&lowered) else {
        return false;
    };
    let Some(payload) = decode_uri_component(&text[prefix.end()..]) else {
        return false;
    };
    let payload: String = payload
        .chars()
        .filter(|c| *c != '\r' && *c != '\n')
        .collect();
    DATA_URL_PAYLOAD.is_match(&payload)
}

fn decode_uri_component(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }
        let hex = text.get(index + 1..index + 3)?;
        if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        decoded.push(u8::from_str_radix(hex, 16).ok()?);
        index += 3;
    }
    String::from_utf8(decoded).ok()
}
