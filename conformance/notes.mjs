// Mirrors examples/notes_server.rs. Keep the two in step, or the conformance
// diff reports a difference in the demo instead of in the SDKs.

export const SERVER_INFO = { name: 'notes', version: '0.1.0' }

export const TOOL_DESCRIPTIONS = {
  search_notes: 'Search notes by a word in their title',
  get_note: 'Read one note by its id',
  get_settings: 'Read the notes service settings',
}

export const NOTES = [
  { id: 1, title: 'Ship the Rust crate', body: 'Publish once the conformance run is clean.' },
  { id: 2, title: 'Record the demo', body: 'Ninety seconds, one failing call.' },
  { id: 3, title: 'Write the Rust README', body: 'Lead with the one-line wrap.' },
]

export function formatNote(note) {
  return `#${note.id} ${note.title}\n${note.body}`
}

export function formatSearch(query) {
  const needle = query.toLowerCase()
  const hits = NOTES.filter((note) => note.title.toLowerCase().includes(needle))
  if (hits.length === 0) return 'No notes match'
  return hits.map((note) => `#${note.id} ${note.title}`).join('\n')
}

export function settings() {
  return {
    region: 'eu',
    api_key: 'sk_live_not_a_real_key',
    webhook: 'https://ops:hunter2@hooks.example.com/notes?token=abc123&team=core',
    posthog_token: 'phc_aaaaaaaaaaaaaaaaaaaaaaaaaaaa',
  }
}
