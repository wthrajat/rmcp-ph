//! The agent echoes the handle the server minted; every call carrying it
//! shares one session.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map, Value};

use crate::contract::arguments;
use crate::support::ids::new_uuid_v7;

static MINTED_SHAPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-7[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$",
    )
    .expect("conversation id pattern")
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Conversation {
    pub id: Option<String>,
    pub minted: bool,
}

/// Agents invent ids like `conv-1`; only a uuidv7 we could have minted is reused.
pub(super) fn resolve(enabled: bool, sent: Option<&Map<String, Value>>) -> Conversation {
    if !enabled {
        return Conversation {
            id: None,
            minted: false,
        };
    }
    let echoed = sent
        .and_then(|sent| sent.get(arguments::CONVERSATION_ID))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| MINTED_SHAPE.is_match(id));
    match echoed {
        Some(id) => Conversation {
            id: Some(id.to_lowercase()),
            minted: false,
        },
        None => Conversation {
            id: Some(new_uuid_v7()),
            minted: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn sent(conversation_id: &str) -> Map<String, Value> {
        let Value::Object(sent) = json!({ "conversation_id": conversation_id }) else {
            unreachable!()
        };
        sent
    }

    #[test]
    fn echoed_handles_are_reused_and_lowercased() {
        let conversation = resolve(true, Some(&sent("01A16F22-413C-7661-A1A1-829BBF0CAEDF")));
        assert_eq!(
            conversation.id.as_deref(),
            Some("01a16f22-413c-7661-a1a1-829bbf0caedf")
        );
        assert!(!conversation.minted);
    }

    #[test]
    fn invented_handles_are_replaced() {
        let conversation = resolve(true, Some(&sent("conv-1")));
        assert!(conversation.minted);
        assert_ne!(conversation.id.as_deref(), Some("conv-1"));
    }
}
