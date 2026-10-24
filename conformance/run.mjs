// Replays one agent session against the official TypeScript SDK and against
// rmcp-posthog, captures what each sends to PostHog, and diffs the two.
//
//   npm install && npm run conformance
//
// Only values that differ on every run are normalized: event uuids, timestamps,
// durations, generated ids (compared by shape and by which events share them),
// and the identity the transport SDK stamps on every event (`$lib`, `$os`, ...).

import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

import { startCaptureServer } from './capture-server.mjs'
import { runScenario } from './scenario.mjs'

const ROOT = fileURLToPath(new URL('..', import.meta.url))
const TOKEN = 'phc_conformance_test_token_123'
const FLUSH_GRACE_MS = 1500

// Identity the transport SDK stamps on everything it sends. `$geoip_disable` is
// deliberately compared: it decides whether PostHog geolocates the server.
const TRANSPORT_PROPERTIES = new Set(['$lib', '$lib_version', '$is_server', '$os', '$os_version'])
const TRANSPORT_PROPERTY_PREFIXES = ['$lib_version__']
const GENERATED_SESSION = /^ses_[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/
const DERIVED_SESSION = /^ses_[0-9a-f]{32}$/
const UUID_V7 = /[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}/g

const SERVERS = {
  typescript: { command: 'node', args: [fileURLToPath(new URL('./reference-server.mjs', import.meta.url))] },
  rust: { command: `${ROOT}target/debug/examples/notes_server`, args: [] },
}

async function capture(server) {
  const capture = await startCaptureServer()
  const env = { POSTHOG_PROJECT_TOKEN: TOKEN, POSTHOG_HOST: capture.url }
  const { results } = await runScenario(server.command, server.args, env)
  await new Promise((resolve) => setTimeout(resolve, FLUSH_GRACE_MS))
  const events = capture.takeEvents()
  await capture.close()
  return { events, results }
}

/** Replaces each distinct generated id with a label, so sharing is compared. */
function labeller() {
  const labels = new Map()
  const counts = new Map()
  return (prefix, value) => {
    const key = `${prefix}:${value}`
    if (!labels.has(key)) {
      const count = (counts.get(prefix) ?? 0) + 1
      counts.set(prefix, count)
      labels.set(key, `<${prefix}${count}>`)
    }
    return labels.get(key)
  }
}

/** posthog-rs keeps properties in a hash map, so visit them in a fixed order. */
function sortedEntries(object) {
  return Object.entries(object ?? {}).sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0))
}

function sessionShape(value) {
  if (GENERATED_SESSION.test(value)) return 'generated'
  if (DERIVED_SESSION.test(value)) return 'derived'
  return 'invalid'
}

function normalizeStrings(value, label) {
  if (typeof value === 'string') return value.replace(UUID_V7, (uuid) => label('uuid', uuid))
  if (Array.isArray(value)) return value.map((item) => normalizeStrings(item, label))
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, normalizeStrings(item, label)]))
  }
  return value
}

function isTransportProperty(key) {
  return TRANSPORT_PROPERTIES.has(key) || TRANSPORT_PROPERTY_PREFIXES.some((prefix) => key.startsWith(prefix))
}

function normalizeEvent(event, label) {
  const properties = {}
  for (const [key, value] of sortedEntries(event.properties)) {
    if (isTransportProperty(key)) continue
    if (key === '$session_id') {
      properties[key] = `${sessionShape(value)}:${label('session', value)}`
    } else if (key === '$mcp_duration_ms') {
      properties[key] = Number.isInteger(value) && value >= 0 ? '<duration>' : value
    } else {
      properties[key] = normalizeStrings(value, label)
    }
  }
  const distinctId = event.distinct_id ?? event.properties?.distinct_id
  return {
    event: event.event,
    distinct_id: `${sessionShape(distinctId)}:${label('session', distinctId)}`,
    properties,
  }
}

function normalizeRun(run) {
  const label = labeller()
  return {
    events: run.events.map((event) => normalizeEvent(event, label)),
    results: run.results.map((result) => normalizeStrings(result, label)),
  }
}

function diff(expected, actual, path = '') {
  if (JSON.stringify(expected) === JSON.stringify(actual)) return []
  const bothObjects = expected && actual && typeof expected === 'object' && typeof actual === 'object'
  if (!bothObjects || Array.isArray(expected) !== Array.isArray(actual)) {
    return [{ path: path || '(root)', typescript: expected, rust: actual }]
  }
  const keys = new Set([...Object.keys(expected), ...Object.keys(actual)])
  return [...keys].flatMap((key) => diff(expected[key], actual[key], `${path}${Array.isArray(expected) ? `[${key}]` : `.${key}`}`))
}

function report(typescript, rust) {
  const differences = [
    ...diff(typescript.events.map((e) => e.event), rust.events.map((e) => e.event), 'event names'),
    ...typescript.events.flatMap((event, index) => diff(event, rust.events[index], `events[${index}] ${event.event}`)),
    ...diff(typescript.results, rust.results, 'tool results'),
  ]
  console.log(`typescript @posthog/mcp: ${typescript.events.length} events`)
  console.log(`rust rmcp-posthog:       ${rust.events.length} events`)
  if (differences.length === 0) {
    console.log('\nPASS: every event matches field for field.')
    return true
  }
  console.log(`\nFAIL: ${differences.length} differences\n`)
  for (const { path, typescript: expected, rust: actual } of differences) {
    console.log(`${path}\n  typescript: ${JSON.stringify(expected)}\n  rust:       ${JSON.stringify(actual)}`)
  }
  return false
}

execFileSync('cargo', ['build', '--quiet', '--example', 'notes_server'], { cwd: ROOT, stdio: 'inherit' })
const typescript = normalizeRun(await capture(SERVERS.typescript))
const rust = normalizeRun(await capture(SERVERS.rust))
if (process.argv.includes('--dump')) {
  console.log(JSON.stringify({ typescript, rust }, null, 2))
}
process.exitCode = report(typescript, rust) ? 0 : 1
