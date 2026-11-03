# @ruviform/ui-showcase

Design system showcase + verification fixtures for the Ruviform UI
packages (Issue #263): a standalone vanilla-TypeScript/Web-Components
showcase page, the headless theme matrix, deterministic demo data and
gateway fakes, and the cross-consumer / dependency-audit gates — all
over the *public* surfaces of `ui/components` (Issue #259), `ui/viz`
(Issue #260), and `ui/binding` (Issue #261).

**Plain TypeScript + DOM APIs only.** No UI framework, runtime,
bundler, or renderer dependency — same package discipline as the rest
of `ui/*` (see [Dependency graph](#dependency-graph-audit)).
`happy-dom` appears in `devDependencies` *only* to provide a DOM for
`node --test`; nothing under `src/` imports it. Nothing here carries a
Synvelis/AudioLens product type — the consumer fixtures are *shapes*
exercising the public API.

## What this package is (and is not)

- **Is:** reusable demo/verification harness — the same
  `mountShowcase()` object the standalone page mounts and the headless
  tests drive. Evidence for theme reuse, gesture/command parity across
  themes, a11y/state behavior, and cross-consumer compatibility.
- **Is not:** a product, a theme pack, or a pixel-reference renderer.
  Real WebView2/WKWebView/WebKitGTK, real GPU/pixel output, real
  multi-window/dock behavior, and screen-reader output are **not
  verified here** — those remain on the #243 manual gates.

## Layout

| Path | Role |
| --- | --- |
| `src/themes.ts` | `SHOWCASE_THEMES` — the matrix columns: `reference-light`, `reference-dark`, `daw-exemplar`, `analysis-exemplar`, `custom-skin` (compact + `{ref}` token indirection). All plain serializable `ThemeInput`s |
| `src/demo-data.ts` | Deterministic ruviform-owned demo data: stereo f64 waveform with a synthetic NaN dropout (0.9–1.1 s on ch0), meter/spectrum snapshot sources + frames — no randomness, no product-domain state |
| `src/demo-gateway.ts` | `DemoControlGateway` (parameter store + scripted receipts + projections) and `DemoAnalysisBackend` (parked bounded queries) — generic `BindingPort` fakes |
| `src/mount.ts` | `mountShowcase(host)` → `ShowcaseHandle`: one `<rvf-theme>` scope per column, the same control row + viz row per theme, deferred cells for Spectrogram/TimeSeries, per-column `ViewLink` + gateway targets |
| `src/consumers.ts` | `mountDawFixture` (Consumer A shape: fader/transport/automation on `mixer.strip.1`) and `mountAnalysisFixture` (Consumer B shape: waveform/spectrum/metadata query on `analysis.view`) |
| `examples/showcase.html` | Standalone static page — mounts the same handle, pumps demo frames, applies receipts |
| `test/*.test.ts` | Headless evidence (see [Verification](#verification)) |

## The theme matrix

`mountShowcase` builds one column per `SHOWCASE_THEMES` entry — the
same row of controls and viz under each `<rvf-theme>` scope, so the
*same* scripted gesture/command runs identically in every column
(themes are data, never an API fork). Columns:

- **Reference Light** (`{ base: "light" }`) — the `#259` reference skin
- **Reference Dark** (`{ base: "dark" }`) — `REFERENCE_DARK_TOKENS`
- **DAW-shape exemplar** — dark base + warm accent role/viz tokens;
  exercises the token surface the way a DAW-style consumer skin would
- **Analysis-shape exemplar** — same token slots, different values;
  proves skin exchange is data-only
- **Custom skin** — light base + `density: "compact"` + `{ ref }` token
  indirection (skin-assembly path)

Each column is its own `ViewLink` view (`showcase.<themeId>`) with its
own demo sources and gateway targets — per-column isolation rather
than a shared store.

## Token / parts / variants quick reference

The full token catalog and per-element parts/slots live in the package
READMEs; this table is the map for interpreting the matrix.

| Surface | Where documented | Showcase use |
| --- | --- | --- |
| Design tokens (`--rvf-*` roles, `viz.*`, `{ref}` indirection, `ThemeInput` precedence: scope → instance → `appearance`) | `ui/components/README.md` "Tokens & themes"; `ui/viz/README.md` "VizStyle" | Column themes + the per-instance `tokens` override + `{ref}` indirection in `custom-skin` |
| Parts (`part="root|track|fill|thumb|readout|label|…"`), `label` slot | `ui/components/README.md` "Slots & parts" | The `domFingerprint` DOM snapshot asserts semantic part baselines for fader/knob/transport |
| Variants (`variant="compact"`, `orientation`, `hide-readout`) | `ui/components/README.md` "Attributes" | Variant exchange test + compact custom-skin column |
| Events (`rvf-gesture`, `rvf-transport`; `rvf-viewport`/`rvf-select`/`rvf-backend` wired in the analysis fixture) | `ui/components/README.md` "Events"; `ui/binding/README.md` | The cross-theme identical-event-stream test |
| Data ports (`WaveformDataSource`, `MeterDataSource`, `SpectrumDataSource`, `BindingPort` command/query/subscribe) | `ui/viz/README.md` "Data ports"; `ui/binding/README.md` | `demo-data.ts` + `demo-gateway.ts` implement them |

## Per-component usage as exercised

| Element | Showcase fixture | Exercised surface |
| --- | --- | --- |
| `<rvf-knob>` `knob` | `Gain` descriptor `0–100 %` | pointer gesture → gateway command → receipt |
| `<rvf-fader>` `fader` | `Vol` descriptor `-60–6 dB` | same + readOnly/disabled/pending states |
| `<rvf-seekbar>` `seekbar` | `Pos` `0–1` | same slider path |
| `<rvf-numeric-input>` `numeric` | `dB` spinbutton | same typed-input path |
| `<rvf-toggle>` `toggle` | `Mute` | `checked`/set gesture, `role=switch` |
| `<rvf-range-selector>` `range` | `Sel` `0–1` | per-edge keyboard (`thumb-lo`) gesture |
| `<rvf-transport>` `transport` | `state=stopped` | `rvf-transport` action intents → non-droppable commands |
| `<rvf-waveform>` | bounded stereo source w/ NaN gap | bounded `buckets` queries, gap buckets, channel labels |
| `<rvf-meter>` | deterministic snapshot frames | `sequence` verbatim, snapshot pump |
| `<rvf-spectrum>` | deterministic frames w/ periodic NaN bins | log freq + dB window mapping |
| `<rvf-theme>` | matrix columns | scope resolution, base swap, diagnostics |
| *(deferred)* `spectrogram`, `timeseries` | honest placeholder cells | rendered as deferred — never faked |

## Cross-consumer gates

`src/consumers.ts` mounts two *shapes* of consumer through nothing but
the public API:

- **Consumer A — DAW-shape** (`mountDawFixture`): fader → `track.vol`,
  transport → `transport`, automation knob → `send.0.level`; one
  `ViewLink` (`mixer.strip.1`) so dock moves exercise generation/cancel
  semantics. Emulates the Synvelis fader/transport/automation-binding
  surface.
- **Consumer B — analysis-shape** (`mountAnalysisFixture`): bounded
  `MemoryWaveformSource` queries, snapshot `push`, `QueryBinding`
  metadata pulls with `maxBytes` bounds and dock-cancel. Emulates the
  AudioLens waveform/spectrum/metadata surface.

Receipt routing is by `intent.controlId` — `CommandBinding` mints
`cmd-N` ids that collide across bindings, so a bare `commandId` can
never route. `gateway.deliver` is wired once per fixture to resolve
each receipt's owning binding from its command record; every receipt
path (`gateway.apply/accept/reject/unsupported` — all taking the
received record) routes through it.

## Dependency graph (audit)

```
ui/core       headless contract — no DOM, no deps
ui/components → ui/core            (presentation, DOM lib)
ui/viz        → ui/core, ui/components (viz tokens/parts)
ui/binding    → ui/core            (ports/adapters; DOM-free src surface)
ui/showcase   → ui/core, ui/components, ui/viz, ui/binding
```

Sibling imports are **source-relative** (`../../<pkg>/src/...`) — no
registry dependencies exist in any `ui/*/package.json`. Runtime
`dependencies`/`peerDependencies`/`optionalDependencies` are empty in
all five manifests; `devDependencies` ⊆ `{typescript, @types/node,
happy-dom}`. `test/deps.test.ts` is the executable gate: it fails on
any framework dep (React/Solid/Vue/Svelte/Preact/Lit/Angular/jQuery/…)
or any bare (non-relative) import specifier in `ui/*/src`.

## Running it

```sh
mise run check-ui-showcase   # npm ci + tsc --noEmit
mise run test-ui-showcase    # npm ci + node --test (71 tests)
mise run build-ui-showcase   # emit dist/ for the static page
```

`examples/showcase.html` is a static ES-module page: serve `ui/` as a
root and load the compiled `dist/` tree (after `build-ui-showcase` —
the page's `<script type="module">` specifier must then point at
`dist/showcase/src/index.js`), or serve the `.ts` sources under a
transpiling dev server.

## Verification

- `theme-matrix.test.ts` — identical gesture/command event streams in
  every theme column (API-fork detector); distinct resolved token sets;
  zero theme diagnostics; token override/`{ref}`/variant/slot exchange;
  range-edge commands reach the gateway and receipts reflect
  `valueLow`/`valueHigh`; runtime base swap repaints vars without
  changing events
- `a11y.test.ts` — per-theme ARIA surface across the control set
  (sliders, spinbutton, two-edge range thumbs, transport buttons,
  switch): `role`/`aria-value*`/`aria-disabled`/`aria-readonly`/
  `aria-busy`/`aria-invalid`/`aria-checked`, keyboard atomic
  begin+commit, disabled/readOnly `unavailable` notices that never
  reach the gateway, pending/invalid attrs, `prefers-reduced-motion`
  rule presence in shadow CSS
- `showcase.test.ts` — full matrix mounts under the Reference Skin
  alone; every viz draws ≥1 frame with ops (per-element stats, not a
  shared-context aggregate); theme swap does not re-query the waveform
  source; binding-observed + projection-driven controlled-value
  reflection; `domFingerprint` semantic-surface baseline for
  fader/knob/transport
- `consumers.test.ts` — Consumer A: gesture→command→apply→`observed`
  round-trip, rejected receipt → `invalid` + last-observed truth,
  non-droppable transport actions, dock-transfer cancel under the old
  generation. Consumer B: bounded gap queries, verbatim `sequence`,
  metadata query resolve/cancel/stale/over-budget outcomes with
  `maxBytes` bounds (16 MiB default asserted)
- `deps.test.ts` — the framework-free dependency audit above

## Not verified here

Real Windows WebView2 / macOS WKWebView / Linux WebKitGTK, real-pixel
visual parity, real GPU/WebGL raster output, real multi-window/dock
behavior, and real screen-reader output — all left to the #243 manual
gates. Spectrogram and TimeSeries are deferred (Issue #262 lane) and
rendered as honest placeholders.
