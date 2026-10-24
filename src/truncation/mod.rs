//! Payload limits from `@posthog/mcp`. Works on the event's internal JSON
//! shape, because that is what the TypeScript SDK measures.

mod budget;
mod fields;
mod largest_strings;
mod normalize;

use serde_json::{Map, Value};

use crate::support::js_text::{js_len, js_prefix};

pub(crate) const MAX_STRING_LENGTH: usize = 32_768;
pub(crate) const TRUNCATION_SUFFIX: &str = "...";

const MAX_DEPTH: usize = 10;
const REDUCIBLE_FIELDS: [&str; 4] = ["parameters", "response", "identifyActorData", "error"];

pub(crate) fn truncate_event(mut event: Map<String, Value>) -> Map<String, Value> {
    fields::apply_field_limits(&mut event);
    for field in REDUCIBLE_FIELDS {
        if let Some(value) = event.get_mut(field) {
            *value = normalize::normalize(value, MAX_DEPTH);
        }
    }
    budget::fit_to_budget(event)
}

pub(crate) fn truncate_string(text: &str, max_length: usize) -> String {
    if js_len(text) <= max_length {
        return text.to_string();
    }
    format!("{}{TRUNCATION_SUFFIX}", js_prefix(text, max_length))
}

#[cfg(test)]
mod tests;
