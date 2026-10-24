use rmcp::ServerHandler;
use rmcp::service::{RequestContext, RoleServer};

use super::Instrumented;
use crate::event::McpEvent;

impl<H: ServerHandler> Instrumented<H> {
    pub(super) fn stamp_connection(
        &self,
        event: &mut McpEvent,
        context: &RequestContext<RoleServer>,
    ) {
        let server = self.inner.get_info().server_info;
        let remembered = self.shared.remembered_client();
        let client = context.client_info();
        event.server_name = Some(server.name);
        event.server_version = Some(server.version);
        event.client_name = client
            .as_ref()
            .map(|client| client.name.clone())
            .or(remembered.name);
        event.client_version = client.map(|client| client.version).or(remembered.version);
        event.protocol_version = remembered.protocol_version;
    }
}
