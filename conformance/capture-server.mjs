// A stand-in for PostHog's ingestion endpoint. Both SDKs send their real HTTP
// batches here, so the comparison covers serialization, not just event building.

import { createServer } from 'node:http'
import { gunzipSync } from 'node:zlib'

const GZIP_MAGIC = [0x1f, 0x8b]

function isGzip(body) {
  return body[0] === GZIP_MAGIC[0] && body[1] === GZIP_MAGIC[1]
}

function decodeBody(body) {
  const raw = isGzip(body) ? gunzipSync(body) : body
  return JSON.parse(raw.toString('utf8'))
}

function eventsFromPayload(payload) {
  if (Array.isArray(payload)) return payload
  if (Array.isArray(payload.batch)) return payload.batch
  return [payload]
}

export async function startCaptureServer() {
  const events = []
  const server = createServer((request, response) => {
    const chunks = []
    request.on('data', (chunk) => chunks.push(chunk))
    request.on('end', () => {
      try {
        events.push(...eventsFromPayload(decodeBody(Buffer.concat(chunks))))
        response.writeHead(200, { 'content-type': 'application/json' })
        response.end('{"status":1}')
      } catch (error) {
        response.writeHead(400)
        response.end(String(error))
      }
    })
  })
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
  const { port } = server.address()
  return {
    url: `http://127.0.0.1:${port}`,
    takeEvents: () => events.splice(0, events.length),
    close: () => new Promise((resolve) => server.close(resolve)),
  }
}
