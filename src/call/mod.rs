//! What a `tools/call` decides before the tool runs.

mod argument_access;
mod conversation;
mod delivery;
mod intent;
mod model;

use rmcp::model::{CallToolRequestParams, Tool};
use rmcp::service::{RequestContext, RoleServer};
use serde_json::{Map, Value};

use crate::contract::UNKNOWN_TOOL_NAME;
use crate::event::{EventType, McpEvent};
use crate::pipeline::parameters::captured_parameters;
use crate::server::state::Shared;
use crate::server::wire::request_json;
use crate::support::ids::derive_session_id;
use crate::tools::cache::ToolFacts;
use crate::tools::input_keys::input_key_properties;
use crate::tools::ownership::strip_owned_arguments;

pub(crate) use conversation::Conversation;
pub(crate) use delivery::{Delivered, deliver_conversation};

pub(crate) struct ToolCall {
    pub downstream: CallToolRequestParams,
    pub event: McpEvent,
    pub conversation: Conversation,
    pub mirror_instructions: bool,
}

pub(crate) fn prepare_tool_call(
    shared: &Shared,
    request: &CallToolRequestParams,
    context: &RequestContext<RoleServer>,
    live_tool: Option<&Tool>,
) -> ToolCall {
    let options = shared.options();
    let listed = shared.tools.get(&request.name);
    let live = live_tool
        .and_then(|tool| serde_json::to_value(tool).ok())
        .map(|tool| ToolFacts::from_descriptor(&tool));
    let access = argument_access::resolve(options, live.as_ref().or(listed.as_ref()));
    let mirror_instructions = options.enable_conversation_id
        && listed
            .as_ref()
            .is_some_and(|tool| tool.ownership.output_instructions);

    let sent = request.arguments.as_ref();
    let conversation = conversation::resolve(access.read.conversation_id, sent);
    let mut downstream = request.clone();
    if let Some(arguments) = downstream.arguments.as_mut() {
        strip_owned_arguments(arguments, access.strip);
    }

    let session_id = match &conversation.id {
        Some(conversation_id) => derive_session_id(conversation_id),
        None => shared.touch_session(),
    };
    let mut event = McpEvent::new(EventType::ToolsCall, session_id);
    event.conversation_id = conversation.id.clone();
    event.resource_name = Some(tool_name_or_unknown(&request.name));
    event.parameters = Some(captured_parameters(&request_json(
        context,
        "tools/call",
        Some(&downstream),
    )));

    let advertised = listed.as_ref().or(live.as_ref());
    event.tool_category = advertised.and_then(|tool| tool.category.clone());
    event.tool_description = advertised.and_then(|tool| tool.description.clone());

    let input_schema = live
        .as_ref()
        .or(listed.as_ref())
        .and_then(|tool| tool.input_schema.as_ref());
    let sent_object = Value::Object(sent.cloned().unwrap_or_else(Map::new));
    event.add_properties(input_key_properties(&sent_object, input_schema));

    if let Some(intent) = intent::from_arguments(sent, access.read.context) {
        event.user_intent = Some(intent);
        event.user_intent_source = Some("context_parameter".to_string());
    }
    let model = options
        .capture_model
        .then(|| model::resolve(sent, context, access.read.llm_model));
    if let Some(found) = model.flatten() {
        event.llm_model = Some(found.model);
        event.llm_model_source = Some(found.source.to_string());
    }

    ToolCall {
        downstream,
        event,
        conversation,
        mirror_instructions,
    }
}

fn tool_name_or_unknown(name: &str) -> String {
    if name.is_empty() {
        UNKNOWN_TOOL_NAME.to_string()
    } else {
        name.to_string()
    }
}
