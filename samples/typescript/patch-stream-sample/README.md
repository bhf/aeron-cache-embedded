# TypeScript Patch Stream Sample

Demonstrates server-side **synthetic PATCH streaming**. With a patch-mode subscriber listening, ordinary
`putItem` writes (never `patchItem`) are turned into patch traffic by the cache service:

- the first put of a key arrives as a full `ADD_ITEM` **baseline**,
- each later overwrite arrives as a `PATCH_ITEM` **delta** carrying only the changed fields,
- an identical (no-op) put is **suppressed** (no event), and
- a non-JSON value falls back to a full `ADD_ITEM`.

An `EmbeddedObjectCache` deep-merges each delta (RFC 7386), so `get` always holds the full,
reconstructed object with untouched (including nested) fields preserved.

## Prerequisites

- Node.js 18+
- A running Aeron Cache backend (defaults to `http://localhost:7070` / `ws://localhost:7071`)

## Running the Sample

```bash
npm install
npm run build
node dist/index.js <baseUrl> <wsUrl>
# e.g.
node dist/index.js http://localhost:7070 ws://localhost:7071
```

The sample loops forever, cycling through a few document states; press `Ctrl+C` to stop.
