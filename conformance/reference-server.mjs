// The TypeScript twin of examples/notes_server.rs, instrumented with the official
// @posthog/mcp SDK. Same tools, same descriptions, same results.

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'
import { instrument } from '@posthog/mcp'
import { PostHog } from 'posthog-node'
import { z } from 'zod'

import { NOTES, SERVER_INFO, TOOL_DESCRIPTIONS, formatNote, formatSearch, settings } from './notes.mjs'

const posthog = new PostHog(process.env.POSTHOG_PROJECT_TOKEN, {
  host: process.env.POSTHOG_HOST,
  flushAt: 1,
  flushInterval: 0,
})

const server = new McpServer(SERVER_INFO)

// Registered in the order rmcp's tool router lists them (by name), and with
// `isError: false` on success, which rmcp's result constructors always set.
server.registerTool(
  'get_note',
  { description: TOOL_DESCRIPTIONS.get_note, inputSchema: { id: z.number().int() } },
  async ({ id }) => {
    const note = NOTES.find((candidate) => candidate.id === id)
    if (!note) {
      return { content: [{ type: 'text', text: `Note ${id} not found` }], isError: true }
    }
    return { content: [{ type: 'text', text: formatNote(note) }], isError: false }
  }
)

server.registerTool('get_settings', { description: TOOL_DESCRIPTIONS.get_settings }, async () => ({
  content: [{ type: 'text', text: JSON.stringify(settings()) }],
  structuredContent: settings(),
  isError: false,
}))

server.registerTool(
  'search_notes',
  { description: TOOL_DESCRIPTIONS.search_notes, inputSchema: { query: z.string() } },
  async ({ query }) => ({ content: [{ type: 'text', text: formatSearch(query) }], isError: false })
)

instrument(server, posthog)

const transport = new StdioServerTransport()
transport.onclose = async () => {
  await posthog.shutdown()
  process.exit(0)
}
await server.connect(transport)
