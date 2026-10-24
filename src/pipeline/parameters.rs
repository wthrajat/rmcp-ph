//! `$mcp_parameters`: the JSON-RPC request, sanitized, without `context`.

use serde_json::{Map, Value, json};

use crate::contract::arguments;
use crate::privacy::sanitize_captured_value;

pub(crate) fn captured_parameters(request: &Map<String, Value>) -> Value {
    let mut captured = Map::new();
    for key in ["id", "jsonrpc", "method"] {
        if let Some(value) = request.get(key) {
            captured.insert(key.to_string(), sanitize_captured_value(value));
        }
    }
    if let Some(params) = request.get("params") {
        captured.insert("params".to_string(), captured_params(params));
    }
    json!({ "request": captured })
}

fn captured_params(params: &Value) -> Value {
    let Value::Object(params) = params else {
        return sanitize_captured_value(params);
    };
    let capture_entry = |(key, value): (&String, &Value)| {
        let captured = match key.as_str() {
            "arguments" => captured_arguments(value),
            _ => sanitize_captured_value(value),
        };
        (key.clone(), captured)
    };
    Value::Object(params.iter().map(capture_entry).collect())
}

fn captured_arguments(arguments: &Value) -> Value {
    let Value::Object(arguments) = arguments else {
        return sanitize_captured_value(arguments);
    };
    let captured = arguments
        .iter()
        .filter(|(key, _)| key.as_str() != arguments::CONTEXT)
        .map(|(key, value)| (key.clone(), sanitize_captured_value(value)))
        .collect();
    Value::Object(captured)
}
