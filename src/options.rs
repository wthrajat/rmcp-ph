//! Configuration. Defaults match `@posthog/mcp` 0.23.0.

use std::sync::Arc;

use crate::event::CaptureEvent;

/// Inspect, change, or drop (`None`) each event right before it is sent.
pub type BeforeSend = Arc<dyn Fn(CaptureEvent) -> Option<CaptureEvent> + Send + Sync>;

#[derive(Clone)]
pub struct Options {
    /// Inject a required `context` argument into every tool so the agent says
    /// why it is calling it. Captured as `$mcp_intent`. On by default.
    pub context: bool,
    /// Replaces the default description of the injected `context` argument.
    pub context_description: Option<String>,
    /// Inject an `llm_model` argument and capture it as `$mcp_llm_model`.
    /// On by default.
    pub capture_model: bool,
    /// Replaces the default description of the injected `llm_model` argument.
    pub model_description: Option<String>,
    /// Hand the agent a `conversation_id` to echo on later calls, so every call
    /// in one conversation shares a session. On by default.
    pub enable_conversation_id: bool,
    /// Send a `$exception` event alongside each failed call. On by default.
    pub enable_exception_autocapture: bool,
    pub before_send: Option<BeforeSend>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            context: true,
            context_description: None,
            capture_model: true,
            model_description: None,
            enable_conversation_id: true,
            enable_exception_autocapture: true,
            before_send: None,
        }
    }
}

impl std::fmt::Debug for Options {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Options")
            .field("context", &self.context)
            .field("capture_model", &self.capture_model)
            .field("enable_conversation_id", &self.enable_conversation_id)
            .field(
                "enable_exception_autocapture",
                &self.enable_exception_autocapture,
            )
            .field("before_send", &self.before_send.is_some())
            .finish_non_exhaustive()
    }
}
