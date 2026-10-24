use rmcp::model::{InitializeRequestParams, InitializeResult};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData as McpError, ServerHandler};

use super::Instrumented;
use super::session::ClientInfo;
use super::wire::request_json;
use crate::contract::UNKNOWN_TOOL_NAME;
use crate::event::{EventType, McpEvent};
use crate::pipeline::parameters::captured_parameters;

impl<H: ServerHandler> Instrumented<H> {
    pub(super) async fn traced_initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        let mut event = McpEvent::new(EventType::Initialize, self.shared.touch_session());
        event.resource_name = Some(UNKNOWN_TOOL_NAME.to_string());
        event.parameters = Some(captured_parameters(&request_json(
            &context,
            "initialize",
            Some(&request),
        )));
        self.stamp_connection(&mut event, &context);
        event.client_name = Some(request.client_info.name.clone());
        event.client_version = Some(request.client_info.version.clone());
        let requested_version = request.protocol_version.to_string();

        let result = self.inner.initialize(request, context).await?;

        let negotiated_version = result.protocol_version.to_string();
        let protocol_version = match negotiated_version.is_empty() {
            true => requested_version,
            false => negotiated_version,
        };
        event.protocol_version = Some(protocol_version.clone());
        event.response = serde_json::to_value(&result).ok();
        self.shared.remember_client(ClientInfo {
            name: event.client_name.clone(),
            version: event.client_version.clone(),
            protocol_version: Some(protocol_version),
        });
        event.finish();
        self.shared.pipeline.publish(event);
        Ok(result)
    }
}
