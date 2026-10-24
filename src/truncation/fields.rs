use serde_json::{Map, Value};

use super::truncate_string;

const MAX_USER_INTENT_LENGTH: usize = 2048;
const MAX_ERROR_MESSAGE_LENGTH: usize = 2048;
const MAX_RESOURCE_NAME_LENGTH: usize = 256;
const MAX_METADATA_LENGTH: usize = 256;
const MAX_CONTENT_TEXT_LENGTH: usize = 32_768;
const MAX_STACK_FRAMES: usize = 50;

const METADATA_FIELDS: [&str; 8] = [
    "serverName",
    "serverVersion",
    "clientName",
    "clientVersion",
    "clientUserAgent",
    "vendorClient",
    "errorType",
    "llmModel",
];

pub(super) fn apply_field_limits(event: &mut Map<String, Value>) {
    limit_text(event, "userIntent", MAX_USER_INTENT_LENGTH);
    limit_text(event, "resourceName", MAX_RESOURCE_NAME_LENGTH);
    for field in METADATA_FIELDS {
        limit_text(event, field, MAX_METADATA_LENGTH);
    }
    if let Some(error) = event.get_mut("error") {
        limit_exceptions(error);
    }
    if let Some(response) = event.get_mut("response") {
        limit_text_blocks(response);
    }
}

fn limit_text(event: &mut Map<String, Value>, field: &str, max_length: usize) {
    if let Some(Value::String(text)) = event.get_mut(field) {
        *text = truncate_string(text, max_length);
    }
}

fn limit_exceptions(error: &mut Value) {
    let Some(Value::Array(exceptions)) = error.get_mut("$exception_list") else {
        return;
    };
    for exception in exceptions.iter_mut().filter_map(Value::as_object_mut) {
        if let Some(Value::String(message)) = exception.get_mut("value") {
            *message = truncate_string(message, MAX_ERROR_MESSAGE_LENGTH);
        }
        if let Some(frames) = exception
            .get_mut("stacktrace")
            .and_then(|trace| trace.get_mut("frames"))
        {
            keep_outer_frames(frames);
        }
    }
}

pub(super) fn keep_outer_frames(frames: &mut Value) {
    let Some(frames) = frames.as_array_mut() else {
        return;
    };
    if frames.len() <= MAX_STACK_FRAMES {
        return;
    }
    let half = MAX_STACK_FRAMES / 2;
    let tail_start = frames.len() - half;
    frames.drain(half..tail_start);
}

fn limit_text_blocks(response: &mut Value) {
    let Some(Value::Array(blocks)) = response.get_mut("content") else {
        return;
    };
    for block in blocks.iter_mut().filter_map(Value::as_object_mut) {
        let is_text = block.get("type").and_then(Value::as_str) == Some("text");
        if let (true, Some(Value::String(text))) = (is_text, block.get_mut("text")) {
            *text = truncate_string(text, MAX_CONTENT_TEXT_LENGTH);
        }
    }
}
