//! `$mcp_input_keys`: the argument names a call sent, never their values. Only
//! names the tool's own schema declares are shown; the rest become one
//! `[redacted]` entry.

use std::collections::HashSet;

use serde_json::{Map, Value};

use crate::contract::{REDACTED, arguments, properties};
use crate::support::js_text::{js_compare, js_len};

const MAX_INPUT_KEYS: usize = 20;
const MAX_KEY_LENGTH: usize = 64;

pub(crate) fn input_key_properties(
    sent: &Value,
    input_schema: Option<&Value>,
) -> Map<String, Value> {
    let mut result = Map::new();
    let Value::Object(sent) = sent else {
        return result;
    };
    let declared_names = declared_names(input_schema);
    let reportable = sent.keys().filter(|name| {
        declared_names.contains(name.as_str()) || !arguments::ALL.contains(&name.as_str())
    });
    let (mut shown, hidden): (Vec<&String>, Vec<&String>) = reportable
        .partition(|name| declared_names.contains(name.as_str()) && js_len(name) <= MAX_KEY_LENGTH);
    shown.sort_by(|left, right| js_compare(left, right));
    let mut keys: Vec<String> = shown.into_iter().take(MAX_INPUT_KEYS).cloned().collect();
    if !hidden.is_empty() && keys.len() < MAX_INPUT_KEYS {
        keys.push(REDACTED.to_string());
    }
    result.insert(properties::INPUT_KEYS.to_string(), keys.into());
    result
}

fn declared_names(input_schema: Option<&Value>) -> HashSet<&str> {
    let declared = input_schema
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object);
    declared
        .map(|names| names.keys().map(String::as_str).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn undeclared_names_collapse_into_one_redacted_entry() {
        let schema = json!({ "properties": { "query": {}, "limit": {} } });
        let sent =
            json!({ "query": "x", "limit": 1, "secret_name": 1, "other": 2, "context": "why" });
        let result = input_key_properties(&sent, Some(&schema));
        assert_eq!(
            result[properties::INPUT_KEYS],
            json!(["limit", "query", "[redacted]"])
        );
    }

    #[test]
    fn analytics_arguments_alone_leave_an_empty_list() {
        let sent = json!({ "context": "why", "llm_model": "m" });
        let result = input_key_properties(&sent, None);
        assert_eq!(result[properties::INPUT_KEYS], json!([]));
    }
}
