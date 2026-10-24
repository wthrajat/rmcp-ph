//! Hands the conversation handle back to the agent: in `structuredContent` for
//! tools that declared it, and as a text block on the call that minted it.

use rmcp::model::CallToolResult;
use serde_json::{Value, json};

use super::Conversation;
use crate::contract::arguments;

pub(crate) struct Delivered {
    pub result: CallToolResult,
    pub original: Value,
    pub handed_over: bool,
}

pub(crate) fn deliver_conversation(
    result: CallToolResult,
    conversation: &Conversation,
    mirror_instructions: bool,
) -> Delivered {
    let original = serde_json::to_value(&result).unwrap_or(Value::Null);
    let Some(conversation_id) = conversation.id.as_deref() else {
        return Delivered {
            result,
            original,
            handed_over: false,
        };
    };
    let mut updated = original.clone();
    let mirrored =
        mirror_instructions && mirror_into_structured_content(&mut updated, conversation_id);
    let prompted = conversation.minted && add_prompt_back(&mut updated, conversation_id);
    match serde_json::from_value(updated) {
        Ok(result) => Delivered {
            result,
            original,
            handed_over: mirrored || prompted,
        },
        Err(error) => {
            tracing::warn!("PostHog MCP analytics could not add the conversation handle: {error}");
            Delivered {
                result,
                original,
                handed_over: false,
            }
        }
    }
}

fn add_prompt_back(result: &mut Value, conversation_id: &str) -> bool {
    let Some(Value::Array(content)) = result.get_mut("content") else {
        return false;
    };
    let text = json!({ "conversation_id": conversation_id }).to_string();
    content.push(json!({ "type": "text", "text": text }));
    true
}

fn mirror_into_structured_content(result: &mut Value, conversation_id: &str) -> bool {
    let Some(Value::Object(structured)) = result.get_mut("structuredContent") else {
        return false;
    };
    if structured.contains_key(arguments::INSTRUCTIONS_KEY) {
        return false;
    }
    let instructions = json!({ "conversation_id": conversation_id });
    structured.insert(arguments::INSTRUCTIONS_KEY.to_string(), instructions);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_back_rides_content() {
        let mut result = json!({ "content": [{ "type": "text", "text": "ok" }] });
        assert!(add_prompt_back(&mut result, "abc"));
        assert_eq!(
            result["content"][1]["text"],
            "{\"conversation_id\":\"abc\"}"
        );
    }
}
