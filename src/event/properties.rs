use serde_json::{Map, Value};

#[derive(Default)]
pub(super) struct Properties(Map<String, Value>);

impl Properties {
    pub(super) fn set(&mut self, key: &str, value: impl Into<Value>) {
        self.0.insert(key.to_string(), value.into());
    }

    pub(super) fn set_text(&mut self, key: &str, value: Option<&String>) {
        if let Some(text) = value.filter(|text| !text.is_empty()) {
            self.set(key, text.clone());
        }
    }

    pub(super) fn set_value(&mut self, key: &str, value: Option<&Value>) {
        if let Some(value) = value {
            self.set(key, value.clone());
        }
    }

    pub(super) fn extend(&mut self, extra: Option<&Map<String, Value>>) {
        if let Some(extra) = extra {
            self.0.extend(extra.clone());
        }
    }

    pub(super) fn into_map(self) -> Map<String, Value> {
        self.0
    }
}
