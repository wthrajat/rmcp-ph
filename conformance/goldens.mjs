// Regenerates tests/fixtures/redaction-goldens.json from @posthog/core, the
// implementation the TypeScript MCP SDK runs. The Rust tests replay it.

import { readFileSync, writeFileSync } from 'node:fs'
import { sanitizeCapturedValue, sanitizeFreeText } from '@posthog/core'

const OPTIONS = { maxStringLength: 32_768, truncationSuffix: '...' }
const cases = JSON.parse(readFileSync(new URL('./redaction-cases.json', import.meta.url), 'utf8'))

const goldens = cases.map((input) => ({
  input,
  captured: sanitizeCapturedValue(input, OPTIONS),
  freeText: sanitizeFreeText(input, OPTIONS),
}))

const target = new URL('../tests/fixtures/redaction-goldens.json', import.meta.url)
writeFileSync(target, JSON.stringify(goldens, null, 1) + '\n')
console.log(`wrote ${goldens.length} goldens`)
