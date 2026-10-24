//! JavaScript string semantics. The TypeScript SDK measures and cuts strings in
//! UTF-16 code units, so limits only match if this crate does too.

use std::cmp::Ordering;

use serde::Serialize;
use serde_json::Value;

pub(crate) fn js_len(text: &str) -> usize {
    text.encode_utf16().count()
}

pub(crate) fn byte_offset_of_units(text: &str, units: usize) -> usize {
    let mut seen = 0;
    for (offset, character) in text.char_indices() {
        let width = character.len_utf16();
        if seen + width > units {
            return offset;
        }
        seen += width;
    }
    text.len()
}

pub(crate) fn js_prefix(text: &str, units: usize) -> &str {
    &text[..byte_offset_of_units(text, units)]
}

pub(crate) fn js_compare(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

pub(crate) fn js_template(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(flag)) => flag.to_string(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Array(items)) => items
            .iter()
            .map(array_item_template)
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".to_string(),
    }
}

fn array_item_template(item: &Value) -> String {
    match item {
        Value::Null => String::new(),
        other => js_template(Some(other)),
    }
}

pub(crate) fn json_byte_size(value: &impl Serialize) -> usize {
    serde_json::to_vec(value).map_or(0, |bytes| bytes.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_and_cuts_count_utf16_units() {
        let crab = "a\u{1f980}b";
        assert_eq!(js_len(crab), 4);
        assert_eq!(js_prefix(crab, 1), "a");
        assert_eq!(js_prefix(crab, 2), "a");
        assert_eq!(js_prefix(crab, 3), "a\u{1f980}");
    }
}
