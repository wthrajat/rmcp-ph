//! Fits an event into 100KB: collapse nesting level by level, then trim the
//! largest strings.

use serde_json::{Map, Value};

use super::largest_strings::trim_largest_strings;
use super::normalize::normalize;
use super::{MAX_DEPTH, REDUCIBLE_FIELDS};
use crate::support::js_text::json_byte_size;

pub(super) const MAX_EVENT_BYTES: usize = 102_400;

pub(super) fn fit_to_budget(event: Map<String, Value>) -> Map<String, Value> {
    if json_byte_size(&event) <= MAX_EVENT_BYTES {
        return event;
    }
    let mut at_depth_one = None;
    for depth in (1..deepest_reducible_field(&event)).rev() {
        let reduced = collapse_reducible_fields(&event, depth);
        if json_byte_size(&reduced) <= MAX_EVENT_BYTES {
            return reduced;
        }
        if depth == 1 {
            at_depth_one = Some(reduced);
        }
    }
    let reduced = at_depth_one.unwrap_or_else(|| collapse_reducible_fields(&event, 1));
    trim_largest_strings(reduced, MAX_EVENT_BYTES)
}

fn deepest_reducible_field(event: &Map<String, Value>) -> usize {
    REDUCIBLE_FIELDS
        .iter()
        .filter_map(|field| event.get(*field))
        .map(|value| nesting_depth(value, MAX_DEPTH))
        .max()
        .unwrap_or(0)
}

fn nesting_depth(value: &Value, limit: usize) -> usize {
    let children: Vec<&Value> = match value {
        Value::Array(items) => items.iter().collect(),
        Value::Object(object) => object.values().collect(),
        _ => return 0,
    };
    if limit == 0 {
        return 0;
    }
    let mut deepest = 0;
    for child in children {
        deepest = deepest.max(nesting_depth(child, limit - 1));
        if deepest == limit - 1 {
            break;
        }
    }
    deepest + 1
}

fn collapse_reducible_fields(event: &Map<String, Value>, depth: usize) -> Map<String, Value> {
    let mut reduced = event.clone();
    for field in REDUCIBLE_FIELDS {
        if let Some(value) = reduced.get_mut(field).filter(|value| !value.is_null()) {
            *value = normalize(value, depth);
        }
    }
    reduced
}
