use rmcp::model::{ListToolsResult, PaginatedRequestParams, ServerResult, Tool};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData as McpError, ServerHandler};
use serde_json::Value;

use super::Instrumented;
use super::wire::{is_modern, request_json, wire_shape};
use crate::event::{EventType, McpEvent};
use crate::pipeline::exceptions;
use crate::pipeline::parameters::captured_parameters;
use crate::tools::injection::inject_tool_listing;

const NO_TOOLS_MESSAGE: &str = "No tools were sent to MCP client.";

impl<H: ServerHandler> Instrumented<H> {
    pub(super) async fn traced_list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let mut event = McpEvent::new(EventType::ToolsList, self.shared.touch_session());
        event.parameters = Some(captured_parameters(&request_json(
            &context,
            "tools/list",
            request.as_ref(),
        )));
        self.stamp_connection(&mut event, &context);
        let modern = is_modern(&context);

        let result = match self.inner.list_tools(request, context).await {
            Ok(result) => result,
            Err(error) => {
                event.fail(exceptions::from_mcp_error(&error));
                self.publish(event);
                return Err(error);
            }
        };

        let tools = listed_tools(&result);
        if tools.is_empty() {
            tracing::warn!("No tools in the listing; register them before instrument()");
            event.fail(exceptions::from_message(NO_TOOLS_MESSAGE));
            self.publish(event);
            return Ok(result);
        }

        self.shared.tools.remember(&tools);
        let Some(injected) = self.with_analytics_arguments(&result, tools) else {
            return Ok(result);
        };
        event.listed_tool_names = Some(
            injected
                .tools
                .iter()
                .map(|tool| tool.name.to_string())
                .collect(),
        );
        event.response = listing_envelope(&injected, modern);
        event.succeed();
        self.publish(event);
        Ok(injected)
    }

    pub(super) fn injected_tool(&self, tool: Tool) -> Tool {
        let Ok(descriptor) = serde_json::to_value(&tool) else {
            return tool;
        };
        let injected = inject_tool_listing(vec![descriptor], self.shared.options());
        let injected = injected
            .into_iter()
            .next()
            .and_then(|tool| serde_json::from_value(tool).ok());
        injected.unwrap_or(tool)
    }

    fn with_analytics_arguments(
        &self,
        result: &ListToolsResult,
        tools: Vec<Value>,
    ) -> Option<ListToolsResult> {
        let mut listing = serde_json::to_value(result).ok()?;
        listing["tools"] = Value::Array(inject_tool_listing(tools, self.shared.options()));
        match serde_json::from_value(listing) {
            Ok(injected) => Some(injected),
            Err(error) => {
                tracing::warn!(
                    "PostHog MCP analytics could not inject analytics arguments: {error}"
                );
                None
            }
        }
    }
}

fn listed_tools(result: &ListToolsResult) -> Vec<Value> {
    let listing = serde_json::to_value(result).unwrap_or(Value::Null);
    listing
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn listing_envelope(result: &ListToolsResult, modern: bool) -> Option<Value> {
    let Some(Value::Object(mut envelope)) =
        wire_shape(ServerResult::ListToolsResult(result.clone()), modern)
    else {
        return None;
    };
    envelope.remove("tools");
    (!envelope.is_empty()).then_some(Value::Object(envelope))
}
