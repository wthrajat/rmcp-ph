//! How one internal event fans out into PostHog events.

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use super::properties::Properties;
use super::{EventType, McpEvent};
use crate::contract::{ANALYTICS_SOURCE, events, properties as key};

/// One PostHog event, as `before_send` sees it and as it is sent.
#[derive(Clone, Debug, PartialEq)]
pub struct CaptureEvent {
    pub event: String,
    pub distinct_id: String,
    pub properties: Map<String, Value>,
    pub timestamp: DateTime<Utc>,
}

pub(crate) fn build_capture_events(
    event: &McpEvent,
    exception_autocapture: bool,
) -> Vec<CaptureEvent> {
    let mut batch = vec![main_event(event)];
    let failed = event.is_error == Some(true) && event.error.is_some();
    if failed && exception_autocapture {
        batch.push(exception_event(event));
    }
    batch
}

fn main_event(event: &McpEvent) -> CaptureEvent {
    let mut properties = Properties::default();
    properties.set(key::INTERFACE, "mcp");
    properties.set(key::SOURCE, ANALYTICS_SOURCE);
    add_session(event, &mut properties);
    add_tool_identity(event, &mut properties);
    add_listed_tools(event, &mut properties);
    properties.set_value(key::DURATION_MS, event.duration.map(Value::from).as_ref());
    add_connection(event, &mut properties);
    add_intent_and_model(event, &mut properties);
    add_outcome(event, &mut properties);
    properties.set_value(key::PARAMETERS, event.parameters.as_ref());
    properties.set_value(key::RESPONSE, event.response.as_ref());
    properties.extend(event.properties.as_ref());
    capture(event, event.event_type.posthog_name(), properties)
}

fn exception_event(event: &McpEvent) -> CaptureEvent {
    let mut properties = Properties::default();
    properties.set(key::INTERFACE, "mcp");
    add_session(event, &mut properties);
    properties.extend(event.error.as_ref().and_then(Value::as_object));
    add_tool_identity(event, &mut properties);
    add_connection(event, &mut properties);
    properties.extend(event.properties.as_ref());
    capture(event, events::EXCEPTION, properties)
}

fn capture(event: &McpEvent, name: &str, properties: Properties) -> CaptureEvent {
    CaptureEvent {
        event: name.to_string(),
        distinct_id: distinct_id(event),
        properties: properties.into_map(),
        timestamp: event.timestamp,
    }
}

fn distinct_id(event: &McpEvent) -> String {
    let session_id = event.session_id.clone().filter(|id| !id.is_empty());
    session_id.unwrap_or_else(|| "anonymous".to_string())
}

fn add_session(event: &McpEvent, properties: &mut Properties) {
    properties.set_text(key::SESSION_ID, event.session_id.as_ref());
    properties.set_text(key::CONVERSATION_ID, event.conversation_id.as_ref());
    properties.set(key::PROCESS_PERSON_PROFILE, false);
}

fn add_tool_identity(event: &McpEvent, properties: &mut Properties) {
    let is_tool_call = event.event_type == EventType::ToolsCall;
    properties.set_text(key::RESOURCE_NAME, event.resource_name.as_ref());
    if is_tool_call {
        properties.set_text(key::TOOL_NAME, event.resource_name.as_ref());
        properties.set_text(key::TOOL_DESCRIPTION, event.tool_description.as_ref());
        properties.set_text(key::TOOL_CATEGORY, event.tool_category.as_ref());
    }
}

fn add_listed_tools(event: &McpEvent, properties: &mut Properties) {
    let names = event
        .listed_tool_names
        .as_ref()
        .filter(|names| !names.is_empty());
    if let (Some(names), EventType::ToolsList) = (names, event.event_type) {
        properties.set(key::LISTED_TOOL_NAMES, names.clone());
    }
}

fn add_connection(event: &McpEvent, properties: &mut Properties) {
    properties.set_text(key::SERVER_NAME, event.server_name.as_ref());
    properties.set_text(key::SERVER_VERSION, event.server_version.as_ref());
    properties.set_text(key::CLIENT_NAME, event.client_name.as_ref());
    properties.set_text(key::CLIENT_VERSION, event.client_version.as_ref());
    properties.set_text(key::PROTOCOL_VERSION, event.protocol_version.as_ref());
}

fn add_intent_and_model(event: &McpEvent, properties: &mut Properties) {
    properties.set_text(key::INTENT, event.user_intent.as_ref());
    properties.set_text(key::INTENT_SOURCE, event.user_intent_source.as_ref());
    properties.set_text(key::LLM_MODEL, event.llm_model.as_ref());
    properties.set_text(key::LLM_MODEL_SOURCE, event.llm_model_source.as_ref());
}

fn add_outcome(event: &McpEvent, properties: &mut Properties) {
    let Some(is_error) = event.is_error else {
        return;
    };
    properties.set(key::IS_ERROR, is_error);
    if is_error {
        let first_exception = event
            .error
            .as_ref()
            .and_then(|error| error.pointer("/$exception_list/0"));
        properties.set_value(key::ERROR_TYPE, non_empty_text(first_exception, "type"));
        properties.set_value(key::ERROR_MESSAGE, non_empty_text(first_exception, "value"));
    }
}

fn non_empty_text<'a>(exception: Option<&'a Value>, field: &str) -> Option<&'a Value> {
    let value = exception?.get(field)?;
    value
        .as_str()
        .filter(|text| !text.is_empty())
        .map(|_| value)
}
