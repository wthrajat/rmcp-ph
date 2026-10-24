//! Replays goldens produced by `@posthog/core` itself (see
//! `conformance/goldens.mjs`), so the port is checked against the
//! implementation it copies rather than against a reading of it.

use serde::Deserialize;
use serde_json::{Value, json};

use super::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Golden {
    input: String,
    captured: String,
    free_text: String,
}

fn goldens() -> Vec<Golden> {
    let fixture = include_str!("../../tests/fixtures/redaction-goldens.json");
    serde_json::from_str(fixture).expect("golden fixture parses")
}

fn preview(text: &str) -> String {
    text.chars().take(120).collect()
}

#[test]
fn captured_values_match_posthog_core() {
    let mismatches: Vec<String> = goldens()
        .iter()
        .filter_map(|golden| {
            let actual = sanitize_captured_value(&Value::String(golden.input.clone()));
            (actual != Value::String(golden.captured.clone())).then(|| {
                format!(
                    "input:    {}\nexpected: {}\nactual:   {}",
                    preview(&golden.input),
                    preview(&golden.captured),
                    preview(actual.as_str().unwrap_or_default())
                )
            })
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "{} mismatches:\n\n{}",
        mismatches.len(),
        mismatches.join("\n\n")
    );
}

#[test]
fn free_text_matches_posthog_core() {
    let mismatches: Vec<String> = goldens()
        .iter()
        .filter_map(|golden| {
            let actual = sanitize_free_text(&golden.input);
            (actual != golden.free_text).then(|| {
                format!(
                    "input:    {}\nexpected: {}\nactual:   {}",
                    preview(&golden.input),
                    preview(&golden.free_text),
                    preview(&actual)
                )
            })
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "{} mismatches:\n\n{}",
        mismatches.len(),
        mismatches.join("\n\n")
    );
}

#[test]
fn credential_named_keys_are_redacted_whole() {
    let value = json!({ "api_key": { "nested": 1 }, "Authorization": "Bearer x", "region": "eu" });
    let sanitized = sanitize_captured_value(&value);
    assert_eq!(
        sanitized,
        json!({ "api_key": "[redacted]", "Authorization": "[redacted]", "region": "eu" })
    );
}
