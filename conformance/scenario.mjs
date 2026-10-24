// One scripted agent session, replayed against each server.

import { Client } from '@modelcontextprotocol/sdk/client/index.js'
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js'

export const CLIENT_INFO = { name: 'conformance-client', version: '1.0.0' }

const MODEL = 'claude-opus-5-5'

export const CALLS = [
  {
    name: 'search_notes',
    arguments: {
      query: 'rust',
      context: 'Finding notes about the Rust crate to plan the next release step.',
      llm_model: MODEL,
    },
  },
  {
    name: 'get_note',
    arguments: { id: 2, context: 'Reading the demo note to check what the video needs.' },
    echoConversationId: true,
  },
  {
    name: 'get_note',
    arguments: {
      id: 42,
      context: 'Opening a note jane@example.com shared from 10.0.0.12 to answer her question.',
      llm_model: 'unknown',
    },
  },
  {
    name: 'get_settings',
    arguments: { context: 'Checking service settings before changing the webhook.', llm_model: MODEL },
    echoConversationId: true,
  },
  {
    name: 'search_notes',
    arguments: { query: 'demo', sneaky_field: 'x', context: 'Looking up the demo note again.', llm_model: MODEL },
    echoConversationId: true,
  },
  { name: 'search_notes', arguments: { query: 'nothing-matches' } },
]

/** The handle a server handed back, read the way an agent would. */
function conversationIdFrom(result) {
  for (const block of result?.content ?? []) {
    try {
      const parsed = JSON.parse(block.text)
      if (typeof parsed?.conversation_id === 'string') return parsed.conversation_id
    } catch {}
  }
  return undefined
}

export async function runScenario(command, args, env) {
  const transport = new StdioClientTransport({ command, args, env: { ...process.env, ...env }, stderr: 'inherit' })
  const client = new Client(CLIENT_INFO)
  await client.connect(transport)
  const listing = await client.listTools()
  const results = []
  let conversationId
  for (const { echoConversationId, ...call } of CALLS) {
    const echoed = echoConversationId && conversationId ? { conversation_id: conversationId } : {}
    const result = await client.callTool({ ...call, arguments: { ...call.arguments, ...echoed } })
    conversationId = conversationIdFrom(result) ?? conversationId
    results.push(result)
  }
  await client.close()
  return { listing, results }
}
