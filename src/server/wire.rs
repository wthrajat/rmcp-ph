//! Requests and results as they look on the wire, which is what gets captured.

use rmcp::model::{ProtocolVersion, ServerResult};
use rmcp::service::{RequestContext, RoleServer};
use serde::Serialize;
use serde_json::{Map, Value};

pub(crate) fn is_modern(context: &RequestContext<RoleServer>) -> bool {
    let modern = ProtocolVersion::V_2026_07_28;
    context
        .protocol_version()
        .is_some_and(|version| version.as_str() >= modern.as_str())
}

/// `{ id, jsonrpc, method, params }`, with the `_meta` rmcp moved into the
/// context put back where the client sent it.
pub(crate) fn request_json(
    context: &RequestContext<RoleServer>,
    method: &str,
    params: Option<&impl Serialize>,
) -> Map<String, Value> {
    let mut request = Map::new();
    if let Ok(id) = serde_json::to_value(&context.id) {
        request.insert("id".to_string(), id);
    }
    request.insert("jsonrpc".to_string(), "2.0".into());
    request.insert("method".to_string(), method.into());
    let params = params.and_then(|params| serde_json::to_value(params).ok());
    if let Some(params) = with_request_meta(params, context) {
        request.insert("params".to_string(), params);
    }
    request
}

fn with_request_meta(params: Option<Value>, context: &RequestContext<RoleServer>) -> Option<Value> {
    let meta = serde_json::to_value(&context.meta)
        .ok()
        .filter(|meta| meta.as_object().is_some_and(|m| !m.is_empty()));
    let Some(meta) = meta else {
        return params;
    };
    let mut params = match params {
        Some(Value::Object(params)) => params,
        Some(other) => return Some(other),
        None => Map::new(),
    };
    params.entry("_meta").or_insert(meta);
    Some(Value::Object(params))
}

/// rmcp's last step before sending: legacy peers lose `resultType`, 2026-07-28
/// peers gain caching hints.
pub(crate) fn wire_shape(mut result: ServerResult, modern: bool) -> Option<Value> {
    if modern {
        result.fill_missing_cache_hints();
    } else {
        result.strip_result_type_for_legacy_peer();
    }
    serde_json::to_value(&result).ok()
}
