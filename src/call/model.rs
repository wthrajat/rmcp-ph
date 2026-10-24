//! Which model made the call: client metadata first, the agent's own
//! `llm_model` argument second.

use rmcp::service::{RequestContext, RoleServer};
use serde_json::{Map, Value};

use crate::contract::arguments;

const CODEX_TURN_METADATA: &str = "x-codex-turn-metadata";

pub(super) struct Model {
    pub model: String,
    pub source: &'static str,
}

pub(super) fn resolve(
    sent: Option<&Map<String, Value>>,
    context: &RequestContext<RoleServer>,
    can_read_argument: bool,
) -> Option<Model> {
    let from_client = serde_json::to_value(&context.meta).ok().and_then(|meta| {
        meta.get(CODEX_TURN_METADATA)?
            .get("model")
            .and_then(known_model)
    });
    if let Some(model) = from_client {
        return Some(Model {
            model,
            source: "client_metadata",
        });
    }
    let self_reported = sent?.get(arguments::LLM_MODEL).and_then(known_model)?;
    can_read_argument.then_some(Model {
        model: self_reported,
        source: "self_reported",
    })
}

fn known_model(model: &Value) -> Option<String> {
    let model = model.as_str()?.trim();
    let unknown = model.is_empty() || model.eq_ignore_ascii_case("unknown");
    (!unknown).then(|| model.to_string())
}
