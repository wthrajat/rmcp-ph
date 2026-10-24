//! Adds `context`, `llm_model` and `conversation_id` to each listed tool, in
//! the order `@posthog/mcp` adds them.

use serde_json::{Map, Value, json};

use super::output_schema::declare_instructions;
use super::ownership::{can_add_argument, declares};
use crate::contract::arguments;
use crate::options::Options;

pub(crate) fn inject_tool_listing(tools: Vec<Value>, options: &Options) -> Vec<Value> {
    tools
        .into_iter()
        .map(|tool| inject_tool(tool, options))
        .collect()
}

fn inject_tool(mut tool: Value, options: &Options) -> Value {
    let Some(descriptor) = tool.as_object_mut() else {
        return tool;
    };
    if options.context {
        let description = custom_or(&options.context_description, arguments::CONTEXT_DESCRIPTION);
        add_argument(descriptor, arguments::CONTEXT, description, true);
    }
    if options.capture_model {
        let description = custom_or(&options.model_description, arguments::LLM_MODEL_DESCRIPTION);
        add_argument(descriptor, arguments::LLM_MODEL, description, true);
    }
    if options.enable_conversation_id {
        let description = arguments::CONVERSATION_ID_DESCRIPTION;
        add_argument(descriptor, arguments::CONVERSATION_ID, description, false);
        declare_instructions(descriptor);
    }
    tool
}

fn custom_or<'a>(custom: &'a Option<String>, default: &'a str) -> &'a str {
    custom
        .as_deref()
        .filter(|text| !text.is_empty())
        .unwrap_or(default)
}

fn add_argument(tool: &mut Map<String, Value>, name: &str, description: &str, required: bool) {
    let schema = tool.get("inputSchema").and_then(Value::as_object);
    if !can_add_argument(schema, name) {
        warn_skipped(tool, schema, name);
        return;
    }
    let empty_schema = || json!({ "type": "object", "properties": {}, "required": [] });
    let Some(schema) = tool
        .entry("inputSchema")
        .or_insert_with(empty_schema)
        .as_object_mut()
    else {
        return;
    };
    if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
        schema.remove("additionalProperties");
    }
    let properties = schema.entry("properties").or_insert_with(|| json!({}));
    if let Some(properties) = properties.as_object_mut() {
        properties.insert(
            name.to_string(),
            json!({ "type": "string", "description": description }),
        );
    }
    if required {
        add_required(schema, name);
    }
}

fn add_required(schema: &mut Map<String, Value>, name: &str) {
    match schema.get_mut("required") {
        Some(Value::Array(required)) if !required.iter().any(|entry| entry == name) => {
            required.push(name.into());
        }
        Some(Value::Array(_)) => {}
        _ => {
            schema.insert("required".to_string(), json!([name]));
        }
    }
}

fn warn_skipped(tool: &Map<String, Value>, schema: Option<&Map<String, Value>>, name: &str) {
    let tool_name = tool
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let reason = match declares(schema, name) {
        true => format!("already has '{name}'"),
        false => "has a composed schema (oneOf/allOf/anyOf/$ref)".to_string(),
    };
    tracing::warn!("Tool \"{tool_name}\" {reason}; skipping {name} injection");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(schema: Value) -> Map<String, Value> {
        let Value::Object(tool) = json!({ "name": "t", "inputSchema": schema }) else {
            unreachable!()
        };
        tool
    }

    #[test]
    fn injects_a_required_argument_and_drops_closed_schemas() {
        let mut descriptor = tool(json!({
            "type": "object",
            "properties": { "q": { "type": "string" } },
            "required": ["q"],
            "additionalProperties": false,
        }));
        add_argument(&mut descriptor, arguments::CONTEXT, "why", true);
        let schema = &descriptor["inputSchema"];
        assert_eq!(schema["required"], json!(["q", "context"]));
        assert_eq!(schema["properties"]["context"]["type"], "string");
        assert!(schema.get("additionalProperties").is_none());
    }

    #[test]
    fn declared_arguments_are_not_injected() {
        let mut descriptor = tool(json!({ "type": "object", "properties": { "context": {} } }));
        add_argument(&mut descriptor, arguments::CONTEXT, "why", true);
        assert!(descriptor["inputSchema"].get("required").is_none());
    }
}
