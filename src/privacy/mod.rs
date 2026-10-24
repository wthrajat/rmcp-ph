//! Redaction run on every captured value, ported from `@posthog/core`. Order
//! matters: binary gate, PostHog tokens, PII (free text only), then URLs.

mod binary;
mod keys;
mod patterns;
mod pii;
mod tokens;
mod urls;

use serde_json::{Map, Value};

use crate::contract::REDACTED;
use crate::support::js_text::{byte_offset_of_units, js_len};
use crate::truncation::{MAX_STRING_LENGTH, TRUNCATION_SUFFIX};

const BINARY_REDACTED: &str = "[binary data redacted - not supported by PostHog MCP analytics]";

pub(crate) fn sanitize_captured_value(value: &Value) -> Value {
    sanitize_value_with(value, &sanitize_string)
}

pub(crate) fn sanitize_text(text: &str) -> String {
    sanitize_string(text)
}

pub(crate) fn sanitize_free_text(text: &str) -> String {
    if binary::is_binary_blob(text) {
        return BINARY_REDACTED.to_string();
    }
    redact_scanned_head(text, |head| {
        urls::redact_urls(&pii::redact_pii(&tokens::redact_tokens(head)))
    })
}

fn sanitize_string(text: &str) -> String {
    if binary::is_binary_blob(text) {
        return BINARY_REDACTED.to_string();
    }
    redact_scanned_head(text, |head| urls::redact_urls(&tokens::redact_tokens(head)))
}

fn sanitize_value_with(value: &Value, sanitize: &dyn Fn(&str) -> String) -> Value {
    match value {
        Value::String(text) => Value::String(sanitize(text)),
        Value::Array(items) => items
            .iter()
            .map(|item| sanitize_value_with(item, sanitize))
            .collect(),
        Value::Object(object) => Value::Object(sanitize_object(object, sanitize)),
        other => other.clone(),
    }
}

fn sanitize_object(
    object: &Map<String, Value>,
    sanitize: &dyn Fn(&str) -> String,
) -> Map<String, Value> {
    let sanitize_entry = |(key, nested): (&String, &Value)| {
        let sanitized = if keys::is_credential_key(key) {
            Value::from(REDACTED)
        } else {
            sanitize_value_with(nested, sanitize)
        };
        (key.clone(), sanitized)
    };
    object.iter().map(sanitize_entry).collect()
}

/// Redacts only the part of a huge string that truncation can keep.
fn redact_scanned_head(text: &str, redact: impl Fn(&str) -> String) -> String {
    let scanned_units = 4 * MAX_STRING_LENGTH;
    if js_len(text) > scanned_units {
        let head = redact(&text[..head_end(text, scanned_units)]);
        if js_len(&head) >= 2 * MAX_STRING_LENGTH {
            return head + TRUNCATION_SUFFIX;
        }
    }
    redact(text)
}

fn head_end(text: &str, scanned_units: usize) -> usize {
    let mut end = byte_offset_of_units(text, scanned_units);
    while let Some(last) = text[..end].chars().next_back() {
        if patterns::ends_url(last) {
            break;
        }
        end -= last.len_utf8();
    }
    end
}

#[cfg(test)]
mod tests;
