//! A small notes server over stdio, instrumented in one line.
//!
//! ```sh
//! POSTHOG_PROJECT_TOKEN=phc_... cargo run --example notes_server
//! ```
//!
//! `conformance/reference-server.mjs` is its TypeScript twin; keep the two in
//! step.

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::schemars::{self, JsonSchema};
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use serde::Deserialize;
use serde_json::json;

struct Note {
    id: i64,
    title: &'static str,
    body: &'static str,
}

const NOTES: [Note; 3] = [
    Note {
        id: 1,
        title: "Ship the Rust crate",
        body: "Publish once the conformance run is clean.",
    },
    Note {
        id: 2,
        title: "Record the demo",
        body: "Ninety seconds, one failing call.",
    },
    Note {
        id: 3,
        title: "Write the Rust README",
        body: "Lead with the one-line wrap.",
    },
];

#[derive(Deserialize, JsonSchema)]
struct SearchArgs {
    query: String,
}

#[derive(Deserialize, JsonSchema)]
struct NoteArgs {
    id: i64,
}

struct Notes;

#[tool_router]
impl Notes {
    #[tool(description = "Search notes by a word in their title")]
    async fn search_notes(
        &self,
        Parameters(SearchArgs { query }): Parameters<SearchArgs>,
    ) -> CallToolResult {
        let needle = query.to_lowercase();
        let hits: Vec<String> = NOTES
            .iter()
            .filter(|note| note.title.to_lowercase().contains(&needle))
            .map(|note| format!("#{} {}", note.id, note.title))
            .collect();
        let text = if hits.is_empty() {
            "No notes match".to_string()
        } else {
            hits.join("\n")
        };
        CallToolResult::success(vec![ContentBlock::text(text)])
    }

    #[tool(description = "Read one note by its id")]
    async fn get_note(&self, Parameters(NoteArgs { id }): Parameters<NoteArgs>) -> CallToolResult {
        match NOTES.iter().find(|note| note.id == id) {
            Some(note) => {
                let text = format!("#{} {}\n{}", note.id, note.title, note.body);
                CallToolResult::success(vec![ContentBlock::text(text)])
            }
            None => CallToolResult::error(vec![ContentBlock::text(format!("Note {id} not found"))]),
        }
    }

    #[tool(description = "Read the notes service settings")]
    async fn get_settings(&self) -> CallToolResult {
        CallToolResult::structured(json!({
            "region": "eu",
            "api_key": "sk_live_not_a_real_key",
            "webhook": "https://ops:hunter2@hooks.example.com/notes?token=abc123&team=core",
            "posthog_token": "phc_aaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        }))
    }
}

#[tool_handler]
impl ServerHandler for Notes {
    fn get_info(&self) -> ServerConfig {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_tool_list_changed()
            .build();
        ServerConfig::new(capabilities).with_server_info(Implementation::new("notes", "0.1.0"))
    }
}

async fn posthog_client() -> posthog_rs::Client {
    let token = std::env::var("POSTHOG_PROJECT_TOKEN").unwrap_or_default();
    let mut options = posthog_rs::ClientOptionsBuilder::default();
    // A server's IP says nothing about its users; posthog-node skips the lookup too.
    options.api_key(token).disable_geoip(true);
    if let Ok(host) = std::env::var("POSTHOG_HOST") {
        options.host(host);
    }
    let options = options.build().expect("valid PostHog options");
    posthog_rs::client(options).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let posthog = Arc::new(posthog_client().await);

    let server = rmcp_posthog::instrument(Notes, posthog.clone());

    let running = server.serve(rmcp::transport::stdio()).await?;
    running.waiting().await?;
    posthog.shutdown().await;
    Ok(())
}
