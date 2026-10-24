//! Bounds depth, breadth and string length, collapsing what does not fit.

use serde_json::{Map, Value};

use super::{MAX_STRING_LENGTH, truncate_string};

const MAX_BREADTH: usize = 100;
const OVERFLOW_MARKER: &str = "[MaxProperties ~]";

pub(super) fn normalize(value: &Value, remaining_depth: usize) -> Value {
    match value {
        Value::String(text) => Value::String(truncate_string(text, MAX_STRING_LENGTH)),
        Value::Array(_) if remaining_depth == 0 => Value::from("[Array]"),
        Value::Object(_) if remaining_depth == 0 => Value::from("[Object]"),
        Value::Array(items) => normalize_array(items, remaining_depth - 1),
        Value::Object(object) => normalize_object(object, remaining_depth - 1),
        other => other.clone(),
    }
}

fn normalize_array(items: &[Value], remaining_depth: usize) -> Value {
    let mut result: Vec<Value> = items
        .iter()
        .take(MAX_BREADTH)
        .map(|item| normalize(item, remaining_depth))
        .collect();
    if items.len() > MAX_BREADTH {
        result.push(Value::from(OVERFLOW_MARKER));
    }
    Value::Array(result)
}

fn normalize_object(object: &Map<String, Value>, remaining_depth: usize) -> Value {
    let mut result: Map<String, Value> = object
        .iter()
        .take(MAX_BREADTH)
        .map(|(key, nested)| (key.clone(), normalize(nested, remaining_depth)))
        .collect();
    if object.len() > MAX_BREADTH {
        result.insert("...".to_string(), Value::from(OVERFLOW_MARKER));
    }
    Value::Object(result)
}
