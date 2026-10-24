use serde_json::{Map, Value};

use crate::contract::arguments;

pub(super) fn from_arguments(sent: Option<&Map<String, Value>>, can_read: bool) -> Option<String> {
    if !can_read {
        return None;
    }
    let context = sent?.get(arguments::CONTEXT)?.as_str()?;
    (!context.trim().is_empty()).then(|| context.to_string())
}
