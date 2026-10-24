//! PostHog MCP analytics for [`rmcp`] servers. Experimental and unofficial: a
//! port of PostHog's `@posthog/mcp` 0.23.0 that sends the same `$mcp_*` events.
//!
//! ```no_run
//! # use rmcp::{ServerHandler, ServiceExt};
//! # async fn example<H: ServerHandler>(server: H) -> Result<(), Box<dyn std::error::Error>> {
//! let posthog = std::sync::Arc::new(posthog_rs::client("phc_project_token").await);
//! let server = rmcp_posthog::instrument(server, posthog.clone());
//! let running = server.serve(rmcp::transport::stdio()).await?;
//! running.waiting().await?;
//! posthog.shutdown().await;
//! # Ok(())
//! # }
//! ```

mod call;
mod contract;
mod event;
mod options;
mod pipeline;
mod privacy;
mod server;
mod support;
mod tools;
mod truncation;

pub use event::CaptureEvent;
pub use options::{BeforeSend, Options};
pub use pipeline::Sink;
pub use server::{Instrumented, instrument, instrument_with};
