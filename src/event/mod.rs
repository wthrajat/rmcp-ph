//! The internal event. Field names and order mirror `@posthog/mcp`, because
//! truncation measures its byte budget on this shape.

mod capture;
mod js_date;
mod properties;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::contract::events;

pub use capture::CaptureEvent;
pub(crate) use capture::build_capture_events;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum EventType {
    #[serde(rename = "mcp:initialize")]
    Initialize,
    #[serde(rename = "mcp:tools/list")]
    ToolsList,
    #[serde(rename = "mcp:tools/call")]
    ToolsCall,
}

impl EventType {
    pub(crate) fn posthog_name(self) -> &'static str {
        match self {
            EventType::Initialize => events::INITIALIZE,
            EventType::ToolsList => events::TOOLS_LIST,
            EventType::ToolsCall => events::TOOL_CALL,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpEvent {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub event_type: EventType,
    #[serde(with = "js_date")]
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listed_tool_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_intent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_intent_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Map<String, Value>>,
}

impl McpEvent {
    pub(crate) fn new(event_type: EventType, session_id: String) -> Self {
        Self {
            id: String::new(),
            session_id: Some(session_id),
            event_type,
            timestamp: Utc::now(),
            duration: None,
            server_name: None,
            server_version: None,
            client_name: None,
            client_version: None,
            resource_name: None,
            protocol_version: None,
            tool_category: None,
            tool_description: None,
            listed_tool_names: None,
            parameters: None,
            response: None,
            user_intent: None,
            user_intent_source: None,
            llm_model: None,
            llm_model_source: None,
            is_error: None,
            error: None,
            conversation_id: None,
            properties: None,
        }
    }

    pub(crate) fn add_properties(&mut self, extra: Map<String, Value>) {
        self.properties.get_or_insert_with(Map::new).extend(extra);
    }

    pub(crate) fn succeed(&mut self) {
        self.is_error = Some(false);
    }

    pub(crate) fn fail(&mut self, error: Value) {
        self.is_error = Some(true);
        self.error = Some(error);
    }

    pub(crate) fn finish(&mut self) {
        let elapsed = Utc::now() - self.timestamp;
        self.duration = Some(elapsed.num_milliseconds().max(0) as u64);
    }
}
