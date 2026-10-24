//! `$exception_list` in the shape `@posthog/core` builds for the same inputs.

use std::sync::LazyLock;

use regex::Regex;
use rmcp::ErrorData as McpError;
use serde_json::{Value, json};

static TYPED_MESSAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"^(?:uncaught (?:exception: )?)?",
        r"(?:((?:eval|internal|range|reference|syntax|type|uri|)error): )?(.*)$"
    ))
    .expect("typed message pattern")
});

const LINE_TERMINATORS: [char; 4] = ['\n', '\r', '\u{2028}', '\u{2029}'];

pub(crate) fn from_tool_result(result: &Value) -> Value {
    from_message(&tool_result_message(result))
}

pub(crate) fn from_message(message: &str) -> Value {
    let (error_type, value) = split_error_type(message);
    exception_properties(&error_type, &value, true)
}

pub(crate) fn from_mcp_error(error: &McpError) -> Value {
    let message = format!("MCP error {}: {}", error.code.0, error.message);
    exception_properties("McpError", &message, false)
}

fn exception_properties(error_type: &str, value: &str, synthetic: bool) -> Value {
    json!({
        "$exception_list": [{
            "type": error_type,
            "value": value,
            "mechanism": {
                "type": "generic",
                "handled": true,
                "synthetic": synthetic,
                "exception_id": 0,
            },
        }],
        "$exception_level": "error",
    })
}

fn tool_result_message(result: &Value) -> String {
    let blocks = result
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    let texts: Vec<&str> = blocks
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect();
    let message = texts.join(" ").trim().to_string();
    if message.is_empty() {
        "Unknown error".to_string()
    } else {
        message
    }
}

/// `TypeError: x` becomes type `TypeError`. JavaScript's `.` stops at line
/// terminators, so multi-line messages never match and stay plain errors.
fn split_error_type(message: &str) -> (String, String) {
    let plain = || ("Error".to_string(), message.to_string());
    if message.contains(LINE_TERMINATORS) {
        return plain();
    }
    let lowered = message.to_ascii_lowercase();
    let Some(captures) = TYPED_MESSAGE.captures(&lowered) else {
        return plain();
    };
    let error_type = captures
        .get(1)
        .map_or("Error", |found| &message[found.range()]);
    let value = captures.get(2).map_or("", |found| &message[found.range()]);
    (error_type.to_string(), value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_messages_are_generic_errors() {
        let properties = from_message("Note 42 not found");
        let exception = &properties["$exception_list"][0];
        assert_eq!(exception["type"], "Error");
        assert_eq!(exception["value"], "Note 42 not found");
        assert_eq!(exception["mechanism"]["synthetic"], true);
    }

    #[test]
    fn typed_prefixes_become_the_type() {
        let cases = [
            ("TypeError: x is undefined", "TypeError", "x is undefined"),
            (
                "Uncaught exception: RangeError: too big",
                "RangeError",
                "too big",
            ),
            ("line one\nline two", "Error", "line one\nline two"),
        ];
        for (message, error_type, value) in cases {
            assert_eq!(split_error_type(message), (error_type.into(), value.into()));
        }
    }
}
