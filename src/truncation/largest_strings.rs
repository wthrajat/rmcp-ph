//! The last resort: shorten the longest strings until the event fits.

use serde_json::{Map, Value};

use super::TRUNCATION_SUFFIX;
use crate::support::js_text::{js_len, js_prefix, json_byte_size};

const MAX_ATTEMPTS: usize = 10;
const MIN_STRING_LENGTH: usize = 100;
const MIN_REDUCTION: i64 = 10;
const SUFFIX_OVERHEAD: i64 = 200;

#[derive(Clone)]
enum Step {
    Key(String),
    Index(usize),
}

struct LongString {
    path: Vec<Step>,
    length: usize,
}

pub(super) fn trim_largest_strings(
    event: Map<String, Value>,
    max_bytes: usize,
) -> Map<String, Value> {
    let mut root = Value::Object(event);
    for _ in 0..MAX_ATTEMPTS {
        let size = json_byte_size(&root);
        if size <= max_bytes {
            break;
        }
        let long_strings = longest_first(&root);
        if long_strings.is_empty() || !shorten(&mut root, &long_strings, size - max_bytes) {
            break;
        }
    }
    match root {
        Value::Object(event) => event,
        _ => unreachable!("the event stays an object"),
    }
}

fn longest_first(root: &Value) -> Vec<LongString> {
    let mut found = Vec::new();
    collect_long_strings(root, &mut Vec::new(), &mut found);
    found.sort_by_key(|long_string| std::cmp::Reverse(long_string.length));
    found
}

fn shorten(root: &mut Value, long_strings: &[LongString], excess: usize) -> bool {
    let mut remaining = excess as i64 + SUFFIX_OVERHEAD;
    let mut shortened = false;
    for LongString { path, length } in long_strings {
        if remaining <= 0 {
            break;
        }
        let reduction = remaining.min(*length as i64 / 2);
        if reduction < MIN_REDUCTION {
            continue;
        }
        if let Some(Value::String(text)) = value_at(root, path) {
            let kept_length = *length - reduction as usize;
            *text = format!("{}{TRUNCATION_SUFFIX}", js_prefix(text, kept_length));
            remaining -= reduction;
            shortened = true;
        }
    }
    shortened
}

fn collect_long_strings(value: &Value, path: &mut Vec<Step>, found: &mut Vec<LongString>) {
    if matches!(path.as_slice(), [Step::Key(key)] if key == "serverBuild") {
        return;
    }
    match value {
        Value::String(text) if js_len(text) > MIN_STRING_LENGTH => found.push(LongString {
            path: path.clone(),
            length: js_len(text),
        }),
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                path.push(Step::Index(index));
                collect_long_strings(item, path, found);
                path.pop();
            }
        }
        Value::Object(object) => {
            for (key, nested) in object {
                path.push(Step::Key(key.clone()));
                collect_long_strings(nested, path, found);
                path.pop();
            }
        }
        _ => {}
    }
}

fn value_at<'a>(root: &'a mut Value, path: &[Step]) -> Option<&'a mut Value> {
    path.iter().try_fold(root, |current, step| match step {
        Step::Key(key) => current.get_mut(key.as_str()),
        Step::Index(index) => current.get_mut(*index),
    })
}
