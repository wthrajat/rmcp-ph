use std::sync::Arc;

use serde_json::Value;

use crate::contract::{LIB_NAME, LIB_VERSION};
use crate::event::CaptureEvent;
use crate::support::ids::new_uuid_v7;

/// Receives every event the instrumentation emits. Implemented for
/// [`posthog_rs::Client`]; implement it to send events anywhere else.
pub trait Sink: Send + Sync + 'static {
    fn capture(&self, event: CaptureEvent);
}

impl<S: Sink + ?Sized> Sink for Arc<S> {
    fn capture(&self, event: CaptureEvent) {
        (**self).capture(event);
    }
}

impl Sink for posthog_rs::Client {
    fn capture(&self, event: CaptureEvent) {
        match to_posthog_event(event) {
            Ok(event) => posthog_rs::Client::capture(self, event),
            Err(error) => tracing::warn!("PostHog MCP analytics dropped an event: {error}"),
        }
    }
}

fn to_posthog_event(event: CaptureEvent) -> Result<posthog_rs::Event, posthog_rs::Error> {
    let mut posthog_event = posthog_rs::Event::new(event.event, event.distinct_id);
    for (key, value) in event.properties.into_iter().chain(lib_properties()) {
        posthog_event.insert_prop(key, value)?;
    }
    posthog_event.set_timestamp(event.timestamp)?;
    if let Ok(uuid) = new_uuid_v7().parse() {
        posthog_event.set_uuid(uuid);
    }
    Ok(posthog_event)
}

fn lib_properties() -> Vec<(String, Value)> {
    let mut properties = vec![
        ("$lib".to_string(), Value::from(LIB_NAME)),
        ("$lib_version".to_string(), Value::from(LIB_VERSION)),
    ];
    let parts: Vec<u64> = LIB_VERSION
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect();
    if let [major, minor, patch] = parts[..] {
        properties.push(("$lib_version__major".to_string(), major.into()));
        properties.push(("$lib_version__minor".to_string(), minor.into()));
        properties.push(("$lib_version__patch".to_string(), patch.into()));
    }
    properties
}
