//! Names and values shared with PostHog's official MCP analytics SDKs.

pub(crate) mod arguments;
pub(crate) mod events;
pub(crate) mod properties;

pub(crate) const ANALYTICS_SOURCE: &str = "posthog_mcp_analytics";
pub(crate) const LIB_NAME: &str = "posthog-rs-mcp";
pub(crate) const LIB_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const REDACTED: &str = "[redacted]";
pub(crate) const UNKNOWN_TOOL_NAME: &str = "Unknown Tool Name";
pub(crate) const SESSION_INACTIVITY_MINUTES: u64 = 30;
