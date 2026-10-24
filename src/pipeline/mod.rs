//! sanitize -> truncate -> fan out -> before_send -> sink.

pub(crate) mod exceptions;
pub(crate) mod parameters;
mod sanitize;
mod sink;

use std::panic::{AssertUnwindSafe, catch_unwind};

use serde_json::Value;

use crate::event::{CaptureEvent, McpEvent, build_capture_events};
use crate::options::Options;
use crate::support::ids::new_uuid_v7;
use crate::truncation::truncate_event;

pub use sink::Sink;

pub(crate) struct Pipeline {
    sink: Box<dyn Sink>,
    options: Options,
}

impl Pipeline {
    pub(crate) fn new(sink: impl Sink, options: Options) -> Self {
        Self {
            sink: Box::new(sink),
            options,
        }
    }

    pub(crate) fn options(&self) -> &Options {
        &self.options
    }

    /// Analytics must never break a tool, so a failure here drops the event.
    pub(crate) fn publish(&self, event: McpEvent) {
        let outcome = catch_unwind(AssertUnwindSafe(|| self.process(event)));
        if outcome.is_err() {
            tracing::warn!("PostHog MCP analytics failed to process an event; dropped it");
        }
    }

    fn process(&self, mut event: McpEvent) {
        sanitize::sanitize_event(&mut event);
        let Some(mut event) = truncate(event) else {
            tracing::warn!("PostHog MCP analytics failed to truncate an event; dropped it");
            return;
        };
        event.id = format!("evt_{}", new_uuid_v7());
        let exception_autocapture = self.options.enable_exception_autocapture;
        for capture in build_capture_events(&event, exception_autocapture) {
            if let Some(capture) = self.before_send(capture) {
                self.sink.capture(capture);
            }
        }
    }

    fn before_send(&self, capture: CaptureEvent) -> Option<CaptureEvent> {
        match &self.options.before_send {
            Some(before_send) => before_send(capture),
            None => Some(capture),
        }
    }
}

fn truncate(event: McpEvent) -> Option<McpEvent> {
    let Value::Object(shape) = serde_json::to_value(&event).ok()? else {
        return None;
    };
    serde_json::from_value(Value::Object(truncate_event(shape))).ok()
}
