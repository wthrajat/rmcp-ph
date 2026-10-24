//! A tool that declares `context`, `llm_model` or `conversation_id` itself owns
//! it; analytics only owns the ones it can add.

use serde_json::{Map, Value};

use super::output_schema::can_declare_instructions;
use crate::contract::arguments;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ArgumentFlags {
    pub context: bool,
    pub conversation_id: bool,
    pub llm_model: bool,
}

impl ArgumentFlags {
    pub(crate) const fn all(value: bool) -> Self {
        Self {
            context: value,
            conversation_id: value,
            llm_model: value,
        }
    }

    pub(crate) fn and(self, other: Self) -> Self {
        Self {
            context: self.context && other.context,
            conversation_id: self.conversation_id && other.conversation_id,
            llm_model: self.llm_model && other.llm_model,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ownership {
    pub arguments: ArgumentFlags,
    pub output_instructions: bool,
}

pub(crate) fn ownership(input_schema: Option<&Value>, output_schema: Option<&Value>) -> Ownership {
    let schema = input_schema.and_then(Value::as_object);
    Ownership {
        arguments: ArgumentFlags {
            context: can_add_argument(schema, arguments::CONTEXT),
            conversation_id: can_add_argument(schema, arguments::CONVERSATION_ID),
            llm_model: can_add_argument(schema, arguments::LLM_MODEL),
        },
        output_instructions: can_declare_instructions(output_schema),
    }
}

pub(crate) fn can_add_argument(schema: Option<&Map<String, Value>>, name: &str) -> bool {
    !declares(schema, name) && !schema.is_some_and(is_composed)
}

pub(crate) fn declares(schema: Option<&Map<String, Value>>, name: &str) -> bool {
    let properties = schema
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object);
    properties.is_some_and(|properties| properties.contains_key(name))
}

pub(crate) fn is_composed(schema: &Map<String, Value>) -> bool {
    ["$ref", "oneOf", "allOf", "anyOf"]
        .iter()
        .any(|keyword| is_truthy(schema.get(*keyword)))
}

fn is_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Number(number)) => number.as_f64().is_some_and(|n| n != 0.0),
        Some(_) => true,
    }
}

pub(crate) fn strip_owned_arguments(arguments: &mut Map<String, Value>, owned: ArgumentFlags) {
    let owned_names = [
        (owned.context, arguments::CONTEXT),
        (owned.conversation_id, arguments::CONVERSATION_ID),
        (owned.llm_model, arguments::LLM_MODEL),
    ];
    for (is_owned, name) in owned_names {
        if is_owned {
            arguments.remove(name);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn tools_that_declare_an_argument_own_it() {
        let schema = json!({ "type": "object", "properties": { "context": {} } });
        let owned = ownership(Some(&schema), None);
        assert!(!owned.arguments.context);
        assert!(owned.arguments.llm_model);
    }

    #[test]
    fn composed_schemas_are_left_alone() {
        let schema = json!({ "oneOf": [{ "type": "object" }] });
        assert_eq!(
            ownership(Some(&schema), None).arguments,
            ArgumentFlags::default()
        );
    }
}
