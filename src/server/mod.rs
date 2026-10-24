//! The `rmcp` integration: a `ServerHandler` that wraps yours.

mod call_tool;
mod connection;
mod handler;
mod initialize;
mod list_tools;
pub(crate) mod session;
pub(crate) mod state;
pub(crate) mod wire;

use std::sync::Arc;

use rmcp::ServerHandler;

use crate::event::McpEvent;
use crate::options::Options;
use crate::pipeline::Sink;
use state::Shared;

/// Wraps an `rmcp` server so its handshakes, tool listings and tool calls are
/// captured to PostHog.
///
/// ```no_run
/// # use rmcp::ServerHandler;
/// # async fn example<H: ServerHandler>(server: H) {
/// let posthog = posthog_rs::client("phc_project_token").await;
/// let server = rmcp_posthog::instrument(server, posthog);
/// # }
/// ```
pub fn instrument<H: ServerHandler>(handler: H, sink: impl Sink) -> Instrumented<H> {
    instrument_with(handler, sink, Options::default())
}

pub fn instrument_with<H: ServerHandler>(
    handler: H,
    sink: impl Sink,
    options: Options,
) -> Instrumented<H> {
    Instrumented {
        inner: handler,
        shared: Arc::new(Shared::new(sink, options)),
    }
}

pub struct Instrumented<H> {
    inner: H,
    shared: Arc<Shared>,
}

impl<H> Instrumented<H> {
    pub fn inner(&self) -> &H {
        &self.inner
    }

    fn publish(&self, mut event: McpEvent) {
        event.finish();
        self.shared.pipeline.publish(event);
    }
}
