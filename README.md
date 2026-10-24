# rmcp-posthog

posthog mcp analytics for rust mcp servers built on [rmcp](https://github.com/modelcontextprotocol/rust-sdk). unofficial, experimental.

posthog ships mcp analytics sdks for typescript, python, go and ruby. this is the rust one ([posthog-rs#258](https://github.com/PostHog/posthog-rs/issues/258)).

## use

```rust
let server = rmcp_posthog::instrument(my_server, posthog_client);
```

every tool call lands in posthog with its intent, model, latency, errors and session. secrets and pii are redacted first.

## try

```sh
POSTHOG_PROJECT_TOKEN=phc_... cargo run --example github_scout
```

## proof

a port of `@posthog/mcp` 0.23.0. the same agent session runs through the official typescript sdk and this crate, and every event is compared field by field:

```sh
cd conformance && npm install && npm run conformance
```

## license

apache-2.0. ports code from posthog-js and agentcat, see [notice](NOTICE).
