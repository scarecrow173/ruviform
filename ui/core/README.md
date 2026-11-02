# @ruviform/ui-core

Issue #258: framework-neutral headless UI contract — value mapping and the
`Begin → Preview* → Commit | Cancel` gesture state machine shared by
Knob / Fader / Slider / NumericInput / Seekbar / Range controls
(設計正本: Issue #257).

This package is plain TypeScript with **no runtime dependencies** — no DOM,
WebView, React, Solid, Vue, or Svelte. It runs under `node --test` directly
via erasable TypeScript (mise-pinned Node 26) and inside any WebView or
framework layer.

## Responsibilities

- `src/value.ts` — `ValueDescriptor` + `ValueMapper`: finite validation,
  `[min,max]` plain ⇄ `[0,1]` normalized mapping (`linear` / `log` /
  `custom`), range policy (`clamp` / `reject`), declared step domain
  (`plain` / `normalized`) with snap, fine steps, degenerate and `constant`
  ranges, defaults, unit/format/parse, explicit `−∞ dB` display, and
  `verifyMapping` defect probing.
- `src/interaction.ts` — `InteractionControl` state machine,
  `ControlInput` / `InteractionEvent` contract with per-control monotonic
  `sequence` and adapter-assigned `inputSeq` (duplicate / out-of-order /
  stale-gesture detection), atomic inputs, keyed lifecycle termination
  (`focus-lost`, `detach`), explicit `unavailable` / `rejected` / `noop`
  reasons, coalescible previews, and `ParamValueAdapter<T>` — the explicit
  conversion port consumers implement for plugin parameter types.
- `test/` — deterministic headless tests only; no DOM or renderer.

## Boundary

DOM event listeners, Web Components lifecycle, and presentation adapters
belong to Issue #259+. Product-specific command / RT / project / analysis
state belongs to Synvelis / AudioLens; this package only carries opaque
consumer-owned IDs (`controlId`, `gestureId`, `target`, `originView`).

## Verification

```sh
mise run check-ui-headless   # npm ci + tsc --noEmit (no DOM lib)
mise run test-ui-headless    # npm ci + node --test
```
