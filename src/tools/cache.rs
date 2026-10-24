//! What the last listing said about each tool, before injection.

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;

use super::ownership::{Ownership, ownership};

#[derive(Clone, Debug)]
pub(crate) struct ToolFacts {
    pub description: Option<String>,
    pub category: Option<String>,
    pub ownership: Ownership,
    pub input_schema: Option<Value>,
}

impl ToolFacts {
    pub(crate) fn from_descriptor(tool: &Value) -> Self {
        let input_schema = tool.get("inputSchema");
        Self {
            description: text(tool.get("description")),
            category: text(tool.pointer("/_meta/category")),
            ownership: ownership(input_schema, tool.get("outputSchema")),
            input_schema: input_schema.cloned(),
        }
    }
}

fn text(value: Option<&Value>) -> Option<String> {
    let text = value?.as_str()?;
    (!text.is_empty()).then(|| text.to_string())
}

#[derive(Default)]
pub(crate) struct ToolCache {
    tools: Mutex<HashMap<String, ToolFacts>>,
}

impl ToolCache {
    pub(crate) fn remember(&self, tools: &[Value]) {
        let mut cache = self
            .tools
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for tool in tools {
            if let Some(name) = tool.get("name").and_then(Value::as_str) {
                cache.insert(name.to_string(), ToolFacts::from_descriptor(tool));
            }
        }
    }

    pub(crate) fn get(&self, name: &str) -> Option<ToolFacts> {
        let cache = self
            .tools
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.get(name).cloned()
    }
}
