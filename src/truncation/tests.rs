use serde_json::{Map, Value, json};

use super::budget::MAX_EVENT_BYTES;
use super::fields::keep_outer_frames;
use super::normalize::normalize;
use super::{MAX_DEPTH, truncate_event};
use crate::support::js_text::json_byte_size;

#[test]
fn normalize_collapses_past_depth_and_breadth() {
    let deep = json!({"a": {"b": [1, 2]}});
    assert_eq!(normalize(&deep, 1), json!({"a": "[Object]"}));
    assert_eq!(normalize(&deep, 2), json!({"a": {"b": "[Array]"}}));

    let wide: Vec<Value> = (0..101).map(Value::from).collect();
    let normalized = normalize(&Value::Array(wide), MAX_DEPTH);
    assert_eq!(normalized[100], json!("[MaxProperties ~]"));
}

#[test]
fn stack_frames_keep_both_ends() {
    let mut frames = Value::Array((0..60).map(Value::from).collect());
    keep_outer_frames(&mut frames);
    assert_eq!(frames.as_array().unwrap().len(), 50);
    assert_eq!(frames[24], json!(24));
    assert_eq!(frames[25], json!(35));
}

#[test]
fn oversized_events_fit_the_budget() {
    let huge = "x".repeat(300_000);
    let mut event = Map::new();
    event.insert(
        "parameters".into(),
        json!({"request": {"params": {"blob": huge}}}),
    );
    let truncated = truncate_event(event);
    assert!(json_byte_size(&truncated) <= MAX_EVENT_BYTES);
}
