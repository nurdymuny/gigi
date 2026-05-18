# GIGI Sheets

A spreadsheet-style front-end for GIGI fiber-bundle databases. Non-technical
operators read, edit, filter, and visualize bundles without writing GQL.

Visual contract: [`mockups/sheets.html`](../mockups/sheets.html).
Sprint plan: [`GIGI_SHEETS_SPRINT_SPEC.md`](../GIGI_SHEETS_SPRINT_SPEC.md)
+ [`GIGI_SHEETS_SPRINT_SPEC_ADDENDUM_v0.1.md`](../GIGI_SHEETS_SPRINT_SPEC_ADDENDUM_v0.1.md).

## Stack

React 18 · Vite 5 · TypeScript · Vitest · `@gigi-db/client` (workspace).

The base path is `/gigi/sheets/`; dev server runs on port 5177.

## Develop

```bash
npm install
npm run dev          # http://localhost:5177
npm test             # vitest run
npm run typecheck    # tsc --noEmit
```

Sheets imports `@gigi-db/client` directly from the workspace source at
`../sdk/js/src/index.ts` via a Vite path alias — no separate build step.

## Current status

**S0 — Foundation.** Scaffold + first green test. The grid arrives later
in S0; the geometry overlay follows in S2.
