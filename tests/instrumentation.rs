//! End to end over an in-memory transport: a real `rmcp` client talking to an
//! instrumented server, with events recorded instead of sent.

use std::sync::{Arc, Mutex};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolRequestParams, CallToolResult, ContentBlock, JsonObject};
use rmcp::schemars::{self, JsonSchema};
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use rmcp_posthog::{CaptureEvent, Sink};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<CaptureEvent>>>);

impl Sink for Recorder {
    fn capture(&self, event: CaptureEvent) {
        self.0.lock().unwrap().push(event);
    }
}

impl Recorder {
    fn events(&self) -> Vec<CaptureEvent> {
        self.0.lock().unwrap().clone()
    }
}

#[derive(Deserialize, JsonSchema)]
struct EchoArgs {
    text: String,
}

struct Echo;

#[tool_router]
impl Echo {
    /// Returns its arguments, so the test can see what the tool received.
    #[tool(description = "Echo the text back")]
    async fn echo(&self, Parameters(EchoArgs { text }): Parameters<EchoArgs>) -> CallToolResult {
        CallToolResult::success(vec![ContentBlock::text(text)])
    }

    #[tool(description = "Always fails")]
    async fn fail(&self) -> CallToolResult {
        CallToolResult::error(vec![ContentBlock::text("TypeError: nothing works")])
    }
}

#[tool_handler]
impl ServerHandler for Echo {}

fn arguments(value: Value) -> JsonObject {
    match value {
        Value::Object(object) => object,
        _ => unreachable!("arguments are objects"),
    }
}

fn conversation_id(result: &CallToolResult) -> String {
    let texts = serde_json::to_value(&result.content).unwrap();
    texts
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|block| serde_json::from_str::<Value>(block["text"].as_str()?).ok())
        .find_map(|parsed| parsed["conversation_id"].as_str().map(str::to_string))
        .expect("the result carries a conversation id")
}

fn property<'a>(event: &'a CaptureEvent, key: &str) -> &'a Value {
    event.properties.get(key).unwrap_or(&Value::Null)
}

#[tokio::test]
async fn captures_a_session_the_way_the_typescript_sdk_does() {
    let recorder = Recorder::default();
    let (server_side, client_side) = tokio::io::duplex(64 * 1024);
    let server = rmcp_posthog::instrument(Echo, recorder.clone());
    tokio::spawn(async move { server.serve(server_side).await.unwrap().waiting().await });
    let client = ().serve(client_side).await.unwrap();

    let listing = client.list_tools(None).await.unwrap();
    let echo_tool = listing
        .tools
        .iter()
        .find(|tool| tool.name == "echo")
        .unwrap();
    let required = echo_tool.input_schema["required"].clone();
    assert_eq!(required, json!(["text", "context", "llm_model"]));
    assert!(echo_tool.input_schema["properties"]["conversation_id"].is_object());

    let first = client
        .call_tool(
            CallToolRequestParams::new("echo").with_arguments(arguments(json!({
                "text": "hello",
                "context": "Greeting the user to check the echo tool works.",
                "llm_model": "claude-opus-5-5",
            }))),
        )
        .await
        .unwrap();
    let handle = conversation_id(&first);
    let second = client
        .call_tool(
            CallToolRequestParams::new("fail").with_arguments(arguments(json!({
                "context": "Checking the failure path for a user at jane@example.com.",
                "conversation_id": handle,
            }))),
        )
        .await
        .unwrap();
    assert_eq!(second.is_error, Some(true));
    client.cancel().await.unwrap();

    let events = recorder.events();
    let names: Vec<&str> = events.iter().map(|event| event.event.as_str()).collect();
    assert_eq!(
        names,
        [
            "$mcp_initialize",
            "$mcp_tools_list",
            "$mcp_tool_call",
            "$mcp_tool_call",
            "$exception"
        ]
    );

    let echo_call = &events[2];
    assert_eq!(property(echo_call, "$mcp_tool_name"), "echo");
    assert_eq!(
        property(echo_call, "$mcp_intent_source"),
        "context_parameter"
    );
    assert_eq!(property(echo_call, "$mcp_llm_model"), "claude-opus-5-5");
    assert_eq!(property(echo_call, "$mcp_input_keys"), &json!(["text"]));
    let captured_arguments =
        &property(echo_call, "$mcp_parameters")["request"]["params"]["arguments"];
    assert_eq!(
        captured_arguments,
        &json!({ "text": "hello" }),
        "analytics arguments are stripped"
    );
    assert_eq!(
        first.content.len(),
        2,
        "the tool's text plus the conversation handle"
    );

    let failed_call = &events[3];
    assert_eq!(
        property(failed_call, "$session_id"),
        property(echo_call, "$session_id")
    );
    assert_eq!(property(failed_call, "$mcp_is_error"), true);
    assert_eq!(property(failed_call, "$mcp_error_type"), "TypeError");
    assert_eq!(property(failed_call, "$mcp_error_message"), "nothing works");
    let intent = property(failed_call, "$mcp_intent").as_str().unwrap();
    assert!(
        intent.contains("[redacted]") && !intent.contains("jane@"),
        "{intent}"
    );

    let exception = &events[4];
    assert_eq!(exception.distinct_id, failed_call.distinct_id);
    assert_eq!(
        property(exception, "$exception_list")[0]["type"],
        "TypeError"
    );
}

#[tokio::test]
async fn before_send_can_drop_events() {
    let recorder = Recorder::default();
    let options = rmcp_posthog::Options {
        before_send: Some(Arc::new(|event: CaptureEvent| {
            (event.event != "$mcp_tools_list").then_some(event)
        })),
        ..Default::default()
    };
    let (server_side, client_side) = tokio::io::duplex(64 * 1024);
    let server = rmcp_posthog::instrument_with(Echo, recorder.clone(), options);
    tokio::spawn(async move { server.serve(server_side).await.unwrap().waiting().await });
    let client = ().serve(client_side).await.unwrap();
    client.list_tools(None).await.unwrap();
    client.cancel().await.unwrap();

    let names: Vec<String> = recorder
        .events()
        .into_iter()
        .map(|event| event.event)
        .collect();
    assert_eq!(names, ["$mcp_initialize"]);
}
