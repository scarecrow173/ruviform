---
id: "IMPL-034QTOFbpeDKVZDoNfrXP5"
type: "impl"
status: "completed"
title: "Issue #263 design system showcase theme matrix a11y regression cross-consumer gates"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/263"
    - "https://github.com/scarecrow173/ruviform/issues/257"
  changes:
    added:
      - type: "file"
        path: "ui/showcase/package.json"
      - type: "file"
        path: "ui/showcase/package-lock.json"
      - type: "file"
        path: "ui/showcase/tsconfig.json"
      - type: "file"
        path: "ui/showcase/tsconfig.build.json"
      - type: "file"
        path: "ui/showcase/README.md"
      - type: "file"
        path: "ui/showcase/src/index.ts"
      - type: "file"
        path: "ui/showcase/src/themes.ts"
      - type: "file"
        path: "ui/showcase/src/demo-data.ts"
      - type: "file"
        path: "ui/showcase/src/demo-gateway.ts"
      - type: "file"
        path: "ui/showcase/src/mount.ts"
      - type: "file"
        path: "ui/showcase/src/consumers.ts"
      - type: "file"
        path: "ui/showcase/examples/showcase.html"
      - type: "file"
        path: "ui/showcase/test/env.ts"
      - type: "file"
        path: "ui/showcase/test/helpers.ts"
      - type: "file"
        path: "ui/showcase/test/theme-matrix.test.ts"
      - type: "file"
        path: "ui/showcase/test/a11y.test.ts"
      - type: "file"
        path: "ui/showcase/test/showcase.test.ts"
      - type: "file"
        path: "ui/showcase/test/consumers.test.ts"
      - type: "file"
        path: "ui/showcase/test/deps.test.ts"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implementation Record — Issue #263: Design System Showcase・Visual/A11y Regression・Cross-Consumer Gates

Status: delivered (headless-verified; proportionate slice — see "Not verified here")
Depends on: Issue #258 (`ui/core`, merge `9ff4de9`), Issue #259 (`ui/components`, `84e6972`/`e3d1471`), Issue #260 (`ui/viz`, `b22221a`/`094c4cf`), Issue #261 (`ui/binding`, `6675b6f`/`33a3699`)
Design source: `docs/designs/issue-257-…` §12 (Reference Skin / Theme / VizStyle) and §13 (framework-neutral Web UI/native hybrid)

## Summary

New package `ui/showcase` (`@ruviform/ui-showcase`, private) — the
reusable design-system showcase and headless verification fixture for
the delivered `ui/components` + `ui/viz` + `ui/binding` surfaces: a
standalone vanilla-TypeScript/Web-Components page, a five-column theme
matrix, deterministic ruviform-owned demo data and generic
`BindingPort` gateways, two cross-consumer fixture shapes, and 71
headless tests covering theme parity, a11y/state, DOM-snapshot,
receipt lifecycle, bounded queries, and the no-framework dependency
audit.

## Context

Issue #263 asks for showcase + visual/a11y regression + cross-consumer
gates on top of the merged #258–#261 UI packages, with the explicit
constraint that no Synvelis/AudioLens product types enter public
Ruviform interfaces and that real WebView/GPU/pixel/multi-window
evidence remains honestly unverified (the #243 manual gates own it).
This record covers the headless-verifiable core; deferred/unverified
areas are enumerated below.

## Implementation

### Changes

| Path | Role |
| --- | --- |
| `ui/showcase/package.json` + `tsconfig*.json` | private package, `type: module`, `exports "." → ./src/index.ts`; strict tsconfig matching sibling packages (`erasableSyntaxOnly`, `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`, `verbatimModuleSyntax`, `allowImportingTsExtensions`) |
| `ui/showcase/src/themes.ts` | `SHOWCASE_THEMES` — `reference-light`, `reference-dark`, `daw-exemplar`, `analysis-exemplar`, `custom-skin` (compact + `{ref}` indirection); plain serializable `ThemeInput`s |
| `ui/showcase/src/demo-data.ts` | deterministic stereo f64 waveform (220/330 Hz decaying sines + NaN dropout 0.9–1.1 s on ch0), `DemoMeterSource`/`DemoSpectrumSource` wrappers over `ManualSnapshotSource`, deterministic frames, `pumpDemoOnce` |
| `ui/showcase/src/demo-gateway.ts` | `DemoControlGateway` (target store, scripted `accept`/`apply`/`reject`/`unsupported` receipts, projections) + `DemoAnalysisBackend` (parked bounded queries, `resolveQuery`/`rejectQuery`/`cancelQuery`) — generic `BindingPort` fakes, no product types |
| `ui/showcase/src/mount.ts` | `mountShowcase(host, opts)` → `ShowcaseHandle` — per-theme `<rvf-theme>` scope + `ViewLink` (`showcase.<theme>`), same control/viz row per column, deferred cells for Spectrogram/TimeSeries, per-column gateway targets and demo sources |
| `ui/showcase/src/consumers.ts` | `mountDawFixture` (Consumer A: fader→`track.vol`, transport, automation knob→`send.0.level` on `mixer.strip.1`) and `mountAnalysisFixture` (Consumer B: waveform/spectrum/meter + `QueryBinding` metadata pulls on `analysis.view`) |
| `ui/showcase/examples/showcase.html` | standalone static ESM page mounting the same `ShowcaseHandle`, pumping demo frames and applying receipts |
| `ui/showcase/test/*` | 71 headless tests (below) |
| `ui/showcase/README.md` | public spec: theme matrix, token/parts/variants map, per-component usage, consumer fixtures, dependency graph, unverified boundaries |
| `mise.toml` | `check-ui-showcase` / `test-ui-showcase` / `build-ui-showcase` (same `npm ci --no-audit --no-fund` → script pattern) |
| `.github/workflows/ci.yml` | `ui-verify` job gained the three showcase steps after the binding steps |
| `AGENTS.md` | verification task table + `ui-verify` job description updated |

### Decisions During Implementation

- **Receipt routing by `intent.controlId`, not `commandId`**: each
  `CommandBinding` mints its own `cmd-N` sequence, so command ids
  collide across bindings on a shared gateway. `gateway.deliver` is
  wired once per fixture — `(receivedRecord, receipt)` — and resolves
  the owning binding by `intent.controlId` for EVERY receipt; all
  gateway receipt methods take the `DemoReceivedCommand` record, so no
  public path can bypass the routing or land on a colliding `cmd-N`.
- **Per-column `ViewLink`s**: every theme column is `showcase.<theme>`
  with its own gateway targets (`demo.<theme>.<control>`) and demo
  sources, so columns exercise view isolation rather than sharing a
  store.
- **Spectrogram/TimeSeries as honest placeholders**: #260 deferred
  them (native-parity #262 lane); the matrix renders
  `data-deferred` cells, never fake elements or empty canvases.
- **`push` is fixture-only**: `MeterDataSource`/`SpectrumDataSource`
  expose `snapshot`/`subscribe`; the `DemoMeterSource`/
  `DemoSpectrumSource` intersection types add `push` locally instead of
  widening the public port.
- **DOM-level snapshot = semantic fingerprint**: `domFingerprint`
  captures tag + `part`/`role`/`name`/`slot`/`tabindex`/`type`/
  `aria-*`/`data-*` only — classes, inline styles, and private nesting
  are internals, not the public contract.
- **Product exemplars are verification samples**: `daw-exemplar` /
  `analysis-exemplar` themes exercise the token surface the way a
  consumer skin would; they are not the products' real skins and hold
  no product types.

### Review-cycle fixes (found during implementation testing)

- `CommandIntent.kind` (not `.type`) in fixture/test filtering.
- Disabled/readOnly controls emit local `unavailable` events that must
  never reach the gateway — asserted as the contract, not `[]`.
- Waveform's first bounded query is deferred until a real layout
  (`plot.w` LOD budget); the theme-swap test captures its baseline
  after the deferred query settles, then asserts a repaint adds none.
- `domFingerprint` skips `<style>`/`<script>` and includes `name`/
  `slot` attrs.
- `import` specifier scan strips block comments first (prose quotes
  like `apart from "a new truth arrived"` faked a bare specifier).
- Meter/spectrum autopump intervals keep `node --test` alive if a test
  fails before `unmount()` — mounts now take `autopump: false`, so no
  pump loop ever starts in the headless suite.

### Independent review pass (review-protocol: cooperative + adversarial)

Two independent reviewers found and the fixes below were applied —
all CONFIRMED P0/P1 plus the cheap P2 gate/coverage fixes:

- **P1 — `rvf-range-selector` was silently unwired**: the element emits
  per-edge control ids (`<id>-lo`/`-hi`), which the single
  `<id>` binding's `event()` filter discarded, so no range command ever
  reached the gateway. Fixed with two edge bindings + two targets
  (`demo.<theme>.range-lo`/`-hi`) and a dedicated `bindRangeElement`
  reflect path (`valueLow`/`valueHigh` + shared flags); regression test
  asserts a `range-lo` commit lands and reflects.
- **P1 — `examples/showcase.html` receipt drain raced the microtask
  flush**: the document listener spliced `received` synchronously at
  bubble time, before the binding's `queueMicrotask` flush had run —
  commits applied one gesture late, the final gesture never committed,
  and its command timed out flagging `invalid` ~5 s later. Fixed by
  mounting with a synchronous `flushWith`; commits apply, other kinds
  are acked "accepted".
- **P1 — `examples/showcase.html` styled classes the mount never
  emits** (`rvf-showcase-*` vs the real `data-showcase`/`.sc-col`/
  `.sc-row-*`/`.sc-deferred` DOM) and `theme.label` rendered nowhere.
  CSS rewritten to the emitted structure; each column now renders a
  `.sc-col-title` heading.
- **P1 — receipt API could bypass or misroute**: public
  `accept(commandId)`/`reject(commandId)`/`unsupported(commandId)` and
  the just-in-time `deliver` rewiring left a mutable slot where a
  receipt could be dropped or delivered onto another binding's
  colliding `cmd-N`. Redesigned: `deliver(rec, receipt)` is wired once
  and every receipt method takes the `DemoReceivedCommand`;
  `lastCommandId()` removed; `addTarget` now throws on duplicate
  declaration (shared-gateway cross-mount bleed fails loud).
- **P1 — dependency-gate holes**: `peerDependencies`/
  `optionalDependencies` are now asserted empty (not just
  blocklist-checked); `UI_PACKAGES` is discovered from the filesystem
  so a future `ui/<new>` package enters the audit; the specifier
  scanner strips line comments too, matches `require("x")`, and scans
  all JS/TS source extensions; `FORBIDDEN` covers `lit-html`/
  `lit-element` and friends.
- **P2 — vacuous assertions strengthened**: initial-projection test now
  proves the `observed` binding state and a projection-driven value
  change (not just the element default); theme-parity streams assert
  non-empty before comparison; "every viz paints" asserts per-element
  `stats.framesDrawn`/`lastOpCount`; a11y coverage extended to
  numeric-input/range/transport per theme; `maxBytes` default
  (16 MiB) + `over-budget` + `stale` query outcomes are now tested;
  `DemoAnalysisBackend.currentRevision` is wired into `resolveQuery`
  (the documented "bump to make replies stale" now works); parked-query
  abort listeners are removed on settle; duplicate `opts.themes` ids
  throw; demo `heldPeak` can no longer sit below `peak`.
- **P3 (reported, not fixed)**: per-theme pointer-path gestures
  (component-internal; `dispatchInput` is the #258 public contract
  path); transport `state` attr not driven by binding state
  (element-side display state); `viewIntents` arrays wired but only
  partially exercised; `bindTransportElement` unbind asymmetry in
  `ui/binding` (sibling package, non-causal); scanner's exotic
  string-literal comment edge; `opts.gateway` full-duplex sharing with
  differing theme ids still possible by design.

## Related Experiments

### Adopted

- `domFingerprint` over full-HTML snapshots — a structural baseline
  that survives skin internals while still catching part/role/ARIA
  regressions.
- `DemoAnalysisBackend` parked-query model — deterministic
  resolve/reject/cancel is what stale/reordered/cancelled reply
  scenarios need.

### Rejected

- Receipt routing by `commandId` — collides across `CommandBinding`
  instances (`cmd-1` minted per binding); replaced by `controlId`
  routing.
- Framework-specific or product-branded fixture types — contract
  forbids them; shapes-only fixtures cover the gate.

## Validation

- `npm --prefix ui/showcase run typecheck` — green (strict flags incl.
  `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`,
  `erasableSyntaxOnly`, `verbatimModuleSyntax`)
- `npm --prefix ui/showcase test` — 71 tests green (`node --test`,
  happy-dom + fake 2D context):
  - `theme-matrix.test.ts` — identical gesture/command event streams
    across all 5 theme columns; distinct token resolution; zero theme
    diagnostics; `tokens` override/`{ref}`/variant/`label` slot
    exchange; range-edge commands reach the gateway and receipts
    reflect per edge; runtime `theme` swap repaints vars, gestures
    unchanged
  - `a11y.test.ts` — per-theme ARIA surface across the control set
    (sliders/spinbutton/range thumbs/transport/switch), keyboard atomic
    gestures, disabled/readOnly `unavailable`-never-gatewayed,
    pending/invalid, `prefers-reduced-motion` rule presence
  - `showcase.test.ts` — full matrix mounts; every viz draws ≥1 frame
    with ops (per-element stats); theme swap does not re-query the
    waveform source; observed-state + projection-driven value
    reflection; `domFingerprint` baselines for fader/knob/transport
  - `consumers.test.ts` — Consumer A apply/`observed` round-trip,
    rejected receipt → `invalid` + last-observed truth, non-droppable
    transport action, dock-transfer cancel (old-generation); Consumer B
    bounded gap queries, verbatim `sequence`, metadata query
    resolve/cancel/stale/over-budget outcomes with `maxBytes` bounds
    (16 MiB default asserted)
  - `deps.test.ts` — manifests (all dep classes asserted empty) +
    import-specifier + sibling-DAG audit over filesystem-discovered
    packages
- `npm --prefix ui/showcase run build` — green (`dist/` ESM for the
  static page)
- `mise run check-ui-showcase` / `test-ui-showcase` /
  `build-ui-showcase` — green (same `npm ci` → script pattern)
- Existing UI lanes non-regression: `check/test-ui-headless`,
  `check/test-ui-components`, `check/test-ui-viz`,
  `check/test-ui-binding` — all green
- Workspace `mise run check` / `lint` / `test` — green (115 cargo test
  suites, 0 failures)
- `actionlint .github/workflows/ci.yml` — clean

## Consumer impact determination

- **Synvelis (Consumer A shape)**: `impacted` — no API change required.
  The fader/transport/automation fixture exercises `bindControlElement`
  / `bindTransportElement` / `ViewLink` / `CommandBinding` exactly as a
  DAW consumer would; gesture→command→receipt→projection round-trip
  verified headlessly (see `consumers.test.ts`).
- **AudioLens (Consumer B shape)**: `impacted` — no API change
  required. Bounded `WaveformDataSource` queries, snapshot `push`
  semantics, and `QueryBinding` metadata pulls with `maxBytes` bounds
  and dock-cancel verified headlessly.
- Both fixtures use *only* the public `ui/*` API surface; no product
  types entered any public interface.

## Dependency audit

`ui/*/package.json` (core/components/viz/binding/showcase): zero
runtime `dependencies`/`peerDependencies`/`optionalDependencies`;
`devDependencies` ⊆ `{typescript, @types/node, happy-dom}`; no
React/Solid/Vue/Svelte/Preact/Lit/Angular/jQuery anywhere. All
`ui/*/src` imports are source-relative; sibling edges form the DAG
`core → components → viz`, `core → binding`, `components+viz+binding
→ showcase`. Executable gate: `test/deps.test.ts`.

## Risks

- Headless DOM assertions cannot detect real-pixel regressions; the
  DOM fingerprint and op-count checks bound what can be asserted.
- `examples/showcase.html` is hand-inspected headlessly only — its
  real-browser/WebView load path is part of the unverified set below.

## Not verified here (follow-up / manual — Issue #243 gates)

- Real **Windows WebView2**, **macOS WKWebView**, **Linux WebKitGTK**
  rendering — no real WebView was loaded.
- **Real-pixel visual parity** — fake 2D context records ops, not
  pixels; pixel-perfect raster evidence is out of scope headlessly.
- **Real GPU/WebGL2 raster output** — WebGL2 path not exercised beyond
  fake-context selection/fallback.
- **Real multi-window/dock behavior** — `ViewLink` generation/cancel
  semantics verified headlessly; real window managers untested.
- **Screen-reader output** — ARIA/roles asserted structurally only.
- **Spectrogram / TimeSeries** — deferred (Issue #262 lane); shown as
  honest placeholders.

## Follow-ups

- Issue #262 native-parity lane owns real WebView/native-window
  evidence and the deferred Spectrogram/TimeSeries renderers.
- Issue #243 manual gates remain the sign-off for real-machine
  WebView/GPU/pixel/multiwindow verification.
