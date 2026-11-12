---
id: "IMPL-KOaepeNi5SFj7gy0GtgA38"
type: "impl"
status: "completed"
title: "Issue #259 Framework-neutral Web Components / Reference Skin / Theme・Slot 実装記録"
created: "2026-11-03"
updated: "2026-11-03"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/259"
  changes:
    added:
      - type: "directory"
        path: "ui/components/"
    modified: []
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
  related:
    - "docs/designs/issue-257-driver-neutral-ui-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-257-driver-neutral-ui-contract.md"
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

# Implementation Record — Issue #259: Framework-neutral Web Components / Reference Skin / Theme・Slot

Status: delivered (headless-verified)
Depends on: Issue #258 (`ui/core` headless contract, landed via PR #267 / merge `9ff4de9`)
Design source: `docs/designs/issue-257-…` §12 (Reference Skin/Theme) and §13 (framework-neutral Web UI/Native Hybrid)

## What was built

New package `ui/components` (`@ruviform/ui-components`, private) — plain
TypeScript Custom Elements on top of `ui/core`. No UI framework,
runtime, compiler, or renderer dependency; `happy-dom@21.0.4` is a
test-only devDependency providing the DOM for `node --test`.

### Module map

| Path | Role |
| --- | --- |
| `src/headless.ts` | sole import boundary into `../../core/src/index.ts`; presentation never re-implements value/gesture semantics |
| `src/element.ts` | `RvfElement`: shadow render lifecycle, cleanup list, property upgrade, presence-boolean attributes |
| `src/gesture-input.ts` | DOM→`ControlInput` wiring (pointer capture, wheel-burst timer, dblclick reset, shared keyboard map) |
| `src/control-base.ts` | `RvfControlElement`: one `InteractionControl` per element, controlled `value`, `rvf-gesture` emission, ARIA/state sync, token overrides |
| `src/tokens.ts` | `TOKEN_SPECS` (the public role list), resolver + diagnostics, light/dark reference values, density scaling, `applyTheme`/`clearTheme`, `tv()` fallback vars |
| `src/theme-provider.ts` | `<rvf-theme>` scoped theme element (isolation via complete inline var set) |
| `src/events.ts` | namespaced CustomEvent contract (`rvf-gesture`, `rvf-transport`), `bubbles+composed` |
| `src/styles.ts` | shared Reference Skin fragments (base/reset, state cues, label/readout) |
| `src/controls/*.ts` | `knob`, `fader` (h/v), `seekbar`, `toggle`, `numeric-input`, `transport`, `range-selector` |
| `test/*.test.ts` + `test/env.ts` | 65 headless tests over happy-dom |
| `examples/showcase.html` | thin composition demo (two consumer themes, no logic fork) |

### Decisions worth recording

- **Controlled value**: `value`/`checked`/`valueLow`/`valueHigh` writes
  never emit intent events (consumer's authoritative echo is suppressed
  via the `atomic` meta flag); writes during an active gesture are
  deferred to gesture end. Violations still surface as `rejected` +
  `data-rejected` flash.
- **Rebuild semantics**: `descriptor` swap resets to the new default
  (old committed may be meaningless in the new range); `ports`/identity
  swaps preserve committed value (dock-transfer). An in-flight gesture
  is always terminated with a forwarded `detach` — no stranded gesture.
- **Toggle** pins `{min:0,max:1,step:1}` — the binary range is part of
  the control's identity; a consumer-supplied descriptor is ignored.
- **RangeSelector** is the one composite: two `InteractionControl`s
  (`<id>-lo`/`<id>-hi`) sharing a track; edge-crossing is prevented by a
  presentation-level pointer clamp, not a forked gesture engine.
- **NumericInput** uses a real `<input>` (`role="spinbutton"`): typing
  opens the `edit` gesture, parseable keystrokes emit `preview`,
  Enter/blur commit, Escape cancels, invalid text commits `NaN` →
  `rejected` (gesture survives so the user can fix it; blur cancels +
  reverts). `compositionstart..end` suspends all dispatches (IME-safe).
- **Transport** is intent+display only: `state`/`capabilities` are
  consumer-declared; `rvf-transport` carries `{action, controlId,
  sequence, source}`. No transport model, clock, RT behavior, or undo.
- **Tokens**: three layers (foundation/semantic/role); resolution order
  `reference defaults → base (light|dark) → density → scope overrides →
  instance tokens → semantic state CSS`. Failed tokens are *omitted* so
  stylesheet literal fallbacks apply; all violations surface as
  `TokenDiagnostic` (`unknown-token`, `type-mismatch`, `non-finite`,
  `unknown-ref`, `circular-ref`). Scope isolation is structural — the
  provider sets *every* `--rvf-*` var on its own inline style, so
  nothing leaks in either direction.
- **Node 26 note**: type stripping requires literal `.ts` specifiers in
  source; `tsconfig.build.json` uses `rewriteRelativeImportExtensions`
  so `dist/` emits `.js` specifiers (verified).

## Review-cycle fixes (two-reviewer pass)

- **Gesture-id minting**: pointer/wheel/edit gestures use unique ids
  (`ptr-<id>-<seq>`, `wheel-<seq>`, `edit-<seq>`) — the contract rejects
  a terminated id as `already-terminated`, so reusing ids broke every
  second drag/wheel burst/edit on the same control.
- **Stranding closure**: multi-pointer takeover is ignored at
  `pointerdown`; a rejected terminal commit auto-cancels (pointer is
  gone); `refresh()` terminates in-flight gestures before re-rendering;
  disconnect teardown is idempotent and runs on every disconnect
  (cleanup list is no longer consumed — `suspend()` clears wheel
  timers while `detach()` removes listeners for surface replacement).
- **Controlled-write staging**: `value` writes (property *and*
  attribute) stage through `pendingExternal_` — they defer while a
  gesture is active, while unavailable, or before connect; they survive
  controller rebuilds and land when the control can accept them.
  `syncAvailability` always runs the event pipeline so re-enabling
  flushes staged writes.
- **Sequence continuity**: `meta.sequence` is rebased at the element so
  a controller rebuild (descriptor/ports/`control-id` change) does not
  restart the stream consumers dedupe on. Idle rebuilds no longer emit
  a spurious `noop`.
- **RangeSelector invariants**: the no-crossing clamp moved to the
  input-dispatch boundary (`preprocess`) — pointer, keyboard, wheel,
  and programmatic edge writes all observe it; thumbs use grab-offset
  drags (press never re-centers the edge); `control-id`/`originView`
  work like other controls; `tokens`/availability accessors added.
- **A11y**: fader always reports `aria-orientation` (default is
  vertical); ARIA mirrored pre-render onto the host is stripped once
  the inner target exists; `pointerdown` focuses the a11y target since
  `preventDefault` suppresses browser focus.
- **IME/text**: keydowns during composition are swallowed
  (`composing_`/`isComposing`); `editing_`/`composing_` resync when the
  edit gesture dies externally; the field restores `user-select:text`.
- **Parse/format symmetry** (`ui/core`): the default parser strips a
  trailing unit suffix (`"45 %"` round-trips) and rejects empty text —
  `Number("") === 0` would silently commit zero on a cleared field.
- **Tokens**: instance `{ref}` overrides emit
  `var(--rvf-<target>, <fallback>)` (track ambient scope *and* render
  unthemed); `applyTheme` clears stale vars so a now-failing token
  can't mask the stylesheet fallback; invalid overrides under a dark
  base fall back to the themed value; negative `length`/`duration`
  tokens are rejected as `type-mismatch`.
- **Surface parity**: `tokens`/`tokenDiagnostics` moved to `RvfElement`
  (transport + range-selector get instance overrides); the headless
  contract types are re-exported from `index.ts` so consumers can name
  `InteractionEvent`/`ValueDescriptor`/`ControlInput` without deep
  imports; `dispatchInput` is the supported dispatch door for skins.

## Verification

Ran and green:

- `npm --prefix ui/components run typecheck` (tsc --noEmit, strict +
  `exactOptionalPropertyTypes` + `erasableSyntaxOnly`)
- `npm --prefix ui/components test` — 94 tests (node --test, happy-dom)
- `npm --prefix ui/components run build` — dist emit + specifier rewrite
- `mise run check-ui-headless`, `mise run test-ui-headless` — #258 lane
  still green
- `mise run fmt-check`, `mise run check`, `mise run lint`, `mise run test`
  — workspace lanes (see report)

CI: `ui-verify` job gained `check-ui-components`, `test-ui-components`,
`build-ui-components` steps (#258 pattern: `mise install node@26.12.1`,
`npm ci` from committed lockfile, explicit `node --test` glob).

## Not verified here (follow-up / manual)

- Real WebView2 / WKWebView / WebKitGTK rendering, pointer capture,
  IME, and high-DPI output — deferred to Issue #261 windowed work.
- `matchMedia` (`prefers-reduced-motion`, contrast) — happy-dom lacks
  it; CSS declares the behavior but a real browser run is pending.
- Screen-reader announcement behavior — ARIA is asserted structurally
  only.
- React/other-framework consumer fixtures — Issue #263 scope.

## Boundary notes

- No DAW/analysis/track/playback/project/command state in any control —
  ChannelStrip/TransportBar/ParameterControl exist only as thin
  composition in `examples/showcase.html`.
- `ui/core` gained exactly one behavior fix: the default `parse` strips
  a trailing unit suffix and rejects empty text (the DOM boundary stays
  one-way through `src/headless.ts`; no new surface was added to the
  contract).
- Ordering limitation: a `value` written under the *default* descriptor
  and later superseded by a descriptor swap is reset — consumers set
  `descriptor`/`ports` before or at mount.
