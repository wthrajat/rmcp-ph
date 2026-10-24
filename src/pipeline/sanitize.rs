//! Redaction applied to a whole event before it is truncated and sent.

use serde_json::{Value, json};

use crate::event::McpEvent;
use crate::privacy::{sanitize_captured_value, sanitize_free_text, sanitize_text};
use crate::support::js_text::js_template;

const NOT_SUPPORTED: &str = "not supported by PostHog MCP analytics";

pub(super) fn sanitize_event(event: &mut McpEvent) {
    if let Some(response) = event.response.as_mut().filter(|value| !value.is_null()) {
        *response = sanitize_response(response);
    }
    if let Some(parameters) = event.parameters.as_mut().filter(|value| !value.is_null()) {
        *parameters = sanitize_captured_value(parameters);
    }
    if let Some(name) = event.resource_name.as_mut() {
        *name = sanitize_text(name);
    }
    if let Some(intent) = event.user_intent.as_mut() {
        *intent = sanitize_free_text(intent);
    }
    if let Some(model) = event.llm_model.as_mut() {
        *model = sanitize_text(model);
    }
    if let Some(error) = event.error.as_mut() {
        sanitize_exception_messages(error);
    }
}

fn sanitize_exception_messages(error: &mut Value) {
    let Some(Value::Array(exceptions)) = error.get_mut("$exception_list") else {
        return;
    };
    for exception in exceptions.iter_mut().filter_map(Value::as_object_mut) {
        let message = exception.get("value").cloned().unwrap_or(Value::Null);
        exception.insert("value".to_string(), sanitize_captured_value(&message));
    }
}

fn sanitize_response(response: &Value) -> Value {
    let Some(Value::Array(content)) = response.get("content") else {
        return sanitize_captured_value(response);
    };
    let mut without_content = response.clone();
    without_content["content"] = json!([]);
    let mut sanitized = sanitize_captured_value(&without_content);
    sanitized["content"] = content.iter().map(sanitize_content_block).collect();
    sanitized
}

fn sanitize_content_block(block: &Value) -> Value {
    let Value::Object(fields) = block else {
        return sanitize_captured_value(block);
    };
    match fields.get("type").and_then(Value::as_str) {
        Some("text" | "resource_link") => sanitize_captured_value(block),
        Some("image") => text_stub(format!("[image content redacted - {NOT_SUPPORTED}]")),
        Some("audio") => text_stub(format!("[audio content redacted - {NOT_SUPPORTED}]")),
        Some("resource") if holds_blob(block) => text_stub(format!(
            "[binary resource content redacted - {NOT_SUPPORTED}]"
        )),
        Some("resource") => sanitize_captured_value(block),
        _ => {
            let block_type = fields.get("type").map(sanitize_captured_value);
            let rendered = js_template(block_type.as_ref());
            text_stub(format!(
                "[unsupported content type \"{rendered}\" redacted - {NOT_SUPPORTED}]"
            ))
        }
    }
}

fn holds_blob(block: &Value) -> bool {
    let resource = block.get("resource").and_then(Value::as_object);
    resource.is_some_and(|resource| resource.contains_key("blob"))
}

fn text_stub(text: String) -> Value {
    json!({ "type": "text", "text": text })
}
