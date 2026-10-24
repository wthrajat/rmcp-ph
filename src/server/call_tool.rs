use rmcp::model::{CallToolRequestParams, CallToolResponse, ServerResult};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData as McpError, ServerHandler};

use super::Instrumented;
use super::wire::{is_modern, wire_shape};
use crate::call::{Delivered, ToolCall, deliver_conversation, prepare_tool_call};
use crate::pipeline::exceptions;

impl<H: ServerHandler> Instrumented<H> {
    pub(super) async fn traced_call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let live_tool = self.inner.get_tool(&request.name);
        let call = prepare_tool_call(&self.shared, &request, &context, live_tool.as_ref());
        let ToolCall {
            downstream,
            mut event,
            conversation,
            mirror_instructions,
        } = call;
        self.stamp_connection(&mut event, &context);
        let modern = is_modern(&context);

        let response = match self.inner.call_tool(downstream, context).await {
            Ok(response) => response,
            Err(error) => {
                if conversation.minted {
                    event.conversation_id = None;
                }
                event.fail(exceptions::from_mcp_error(&error));
                self.publish(event);
                return Err(error);
            }
        };

        let CallToolResponse::Complete(result) = response else {
            event.succeed();
            self.publish(event);
            return Ok(response);
        };

        let Delivered {
            result,
            original,
            handed_over,
        } = deliver_conversation(result, &conversation, mirror_instructions);
        if conversation.minted && !handed_over {
            event.conversation_id = None;
        }
        match result.is_error {
            Some(true) => event.fail(exceptions::from_tool_result(&original)),
            _ => event.succeed(),
        }
        event.response = wire_shape(ServerResult::CallToolResult(result.clone()), modern);
        self.publish(event);
        Ok(CallToolResponse::Complete(result))
    }
}
