# @ruviform/ui-components

Framework-neutral Web Components for Ruviform (Issue #259): Reference
Skin controls, a typed design-token/theme system, variants, and
slots/parts — built on the `@ruviform/ui-core` headless contract
(Issue #258).

**Plain TypeScript + DOM APIs only.** No UI framework, runtime, compiler,
or renderer dependency is allowed in this package — not for consumers,
not transitively. `happy-dom` appears in `devDependencies` *only* to
provide a DOM for `node --test`; nothing under `src/` imports it.

## Relationship to ui/core

```
ui/core       headless contract: ValueDescriptor/ValueMapper,
              InteractionControl (Begin→Preview*→Commit|Cancel),
              event types. No DOM.
ui/components presentation: Custom Elements, Shadow DOM, Reference
              Skin CSS, pointer/keyboard/IME wiring, tokens/themes.
              Imports the contract exclusively through src/headless.ts.
```

Presentation code never re-implements value mapping, snapping, or
gesture semantics — every control delegates to `InteractionControl`.
Custom skins subclass the exported classes (or `RvfControlElement`)
and inherit the same machinery; forking gesture/value logic is a
contract violation.

## Elements

| Tag | Control | Role | Pointer gesture | Notes |
| --- | --- | --- | --- | --- |
| `<rvf-knob>` | rotary knob | `slider` | relative vertical drag | SVG arc + indicator |
| `<rvf-fader>` | fader/slider | `slider` | absolute track | `orientation="vertical"` (default) / `"horizontal"` |
| `<rvf-seekbar>` | seek position | `slider` | absolute track | `buffered` display-only property |
| `<rvf-toggle>` | binary switch | `switch` | click/Space/Enter | fixed `{0,1}` descriptor; `checked` property |
| `<rvf-numeric-input>` | text entry | `spinbutton` | type, arrows, steps | IME-safe; Enter/blur commit, Escape cancels |
| `<rvf-range-selector>` | two-edge range | `slider` ×2 | absolute per thumb | edges `-lo`/`-hi` in `controlId`; cannot cross |
| `<rvf-transport>` | transport buttons | `button` | click | **intent + display only — no transport model** |
| `<rvf-theme>` | theme scope | — | — | sets `--rvf-*` vars on itself |

### Attributes shared by value controls

`control-id` (stable id for event `meta.controlId`; falls back to
`id`, then tag name), `value` (controlled committed value — same
deferral semantics as the property), `label`, `disabled`, `readonly`,
`invalid`, `pending`, `unsupported`, `variant` (`compact`),
`hide-readout`. Boolean attributes follow HTML presence semantics.

### Typed properties (never JSON-in-attribute)

| Property | Type | Purpose |
| --- | --- | --- |
| `descriptor` | `ValueDescriptor` | range/step/mapping/default/unit — the serializable contract |
| `ports` | `ValuePorts` | local formatter/parser/custom mapping callbacks |
| `value` | `number` | **controlled** committed value (plain domain) |
| `tokens` | `Record<TokenKey, TokenInput>` | instance-level token overrides |
| `target`, `originView` | `string` | opaque ids carried in event meta |
| `checked` | `boolean` | toggle binary alias for `value` |
| `valueLow`/`valueHigh`/`range` | `number` | range-selector edges |
| `buffered` | `number` | seekbar display fraction `[0,1]` |
| `capabilities`, `state` | `TransportCapabilities`, `string` | transport display model |
| `theme`, `tokens`, `base`, `density` | theme inputs | `<rvf-theme>` scope |

`value` (and `checked`/`valueLow`/`valueHigh`) are **controlled**:
writes are the consumer's authoritative state and emit *no* intent
events. A write while a gesture is active, while the control is
unavailable (`disabled`/`readonly`/`unsupported`), or before connect is
**staged** and lands when the control can accept it — it is never
dropped silently, and it survives controller rebuilds triggered by
`descriptor`/`ports`/`control-id` changes. Contract violations
(non-finite, out-of-range) still surface as `rejected` events and a
transient `data-rejected` attribute.

Set `descriptor`/`ports` before or at mount: a `value` written under the
default descriptor and then superseded by a descriptor swap is reset by
design — the old value may be meaningless in the new range.

## Events

| Type | detail | When |
| --- | --- | --- |
| `rvf-gesture` | `InteractionEvent` verbatim | every headless event: `begin`, `preview`, `commit`, `cancel`, `unavailable`, `rejected`, `noop` |
| `rvf-transport` | `{ action, controlId, sequence, source }` | transport button activation (`play`/`pause`/`resume`/`stop`) |

Both are `bubbles: true, composed: true` — they cross the shadow
boundary and reach `document` listeners. These are **UI intents**, not
command receipts: the consumer's binding layer (#261) owns target
resolution and authorization.

`<rvf-transport>` contains no transport model, clock, RT behavior, undo,
or DAW control — `state` is consumer-declared display state,
`capabilities` marks possible actions, intents are fire-and-forget.
`<rvf-range-selector>` is the one composite: two headless controls
(`<id>-lo` / `<id>-hi`) sharing a track; the no-crossing constraint is
applied at the input-dispatch boundary so pointer, keyboard, wheel,
and programmatic writes all observe it — not a second gesture engine.

## Tokens & themes

`src/tokens.ts` is the public role list (`TOKEN_SPECS`) — three layers:

- **foundation** (`font.*`, `space.unit`, `radius`, `focus.*`, `motion.*`)
- **semantic** (`surface*`, `text*`, `border`, `interactive*`, `focus.ring`,
  `status.*`, `state.disabled`)
- **role** (`knob.*`, `fader.*`, `toggle.*`, `seekbar.*`, `range.*`,
  `numeric.*`, `transport.*`, `control.*`)

Resolution order (low → high):

```
reference defaults → theme base (light|dark) → density scaling
→ scope theme overrides → instance `tokens` → semantic state CSS
```

- `resolveTheme(input)` → `{ vars, tokens, diagnostics }`; a token that
  fails validation/ref-resolution is **omitted** so the stylesheet's
  `var(--rvf-*, <literal fallback>)` applies — controls always render.
- `applyTheme(scope, input)` sets every resolved `--rvf-*` on the
  scope's inline style — a complete set, so themes neither leak out nor
  leak in (scope isolation).
- `<rvf-theme base="dark" density="compact">` wraps any subtree; the
  `theme`/`tokens` properties take the same serializable inputs.
- `el.tokens = {...}` on **any** element (controls, `<rvf-transport>`,
  `<rvf-range-selector>`) applies instance overrides (highest
  precedence) and reports `tokenDiagnostics`. A `{ ref: "token" }`
  override emits `var(--rvf-*, <fallback>)` — it tracks the ambient
  scope theme and still renders with no theme applied.
- Unknown keys, type mismatches, non-finite numbers, unknown/circular
  `ref`s are reported as `TokenDiagnostic`s — never silently accepted.
- Token data is serializable; render callbacks live only in `ports`
  (never cross a theme boundary).

## Slots & parts

Every control exposes a `label` slot (a plain `<span part="label">`
fallback renders the `label` attribute) and semantic parts —
`root`, `track`, `fill`, `thumb`, `thumb-lo`/`thumb-hi`, `arc`,
`arc-track`, `indicator`, `dial`, `field`, `input`, `spin-up`,
`spin-down`, `buffered`, `readout`, `play`/`pause`/`resume`/`stop`.
Private DOM nesting and class names are **not** a public contract.

## Accessibility

- Roles: `slider` (value controls), `switch` (toggle), `spinbutton`
  (inner `<input>`), `group` (range host / transport), `button`
  (transport actions).
- `aria-valuemin/max/now/valuetext` on every value control;
  `aria-orientation` follows `orientation`; `aria-checked` on toggle;
  `aria-label` from `label` (an external `aria-labelledby` wins).
- Keyboard: arrows ±step (Shift = fine), PageUp/Down ±10, Home/End
  extremes, Escape cancels; toggle adds Space/Enter; numeric-input adds
  Enter-commit/Escape-cancel/step arrows on the field.
- Pointer: capture-based drag, `lostpointercapture`/cancel → gesture
  cancel; wheel bursts coalesce into one gesture (200ms idle commit);
  double-click resets to the descriptor default.
- IME: `compositionstart..end` suspends numeric dispatches — no
  half-token previews or commits.
- States are distinct and perceivable: `disabled`, `readonly`,
  `unsupported` (hatched edge), `pending` (aria-busy + outline),
  `invalid`/`data-rejected` (error ring), `data-previewing` (active
  gesture — local preview, not committed).
- `prefers-reduced-motion` collapses transitions; focus uses
  `:focus-visible` + `focus.ring`/`focus.width` tokens.

## Lifecycle

Connect renders the shadow tree once; `variant`/`orientation`/
`hide-readout` rebuild it — an in-flight gesture is terminated first
(its pointer tracking dies with the old nodes) and listeners re-bind.
Disconnect teardown is idempotent and runs on **every** disconnect:
timers cleared, wheel bursts suspended, and an in-flight gesture
terminated via `detach` (cancel) so no gesture strands. Reconnect does
not duplicate subscriptions and keeps the committed value. Gesture ids
are minted per physical gesture — a terminated id is never reused, so
`already-terminated` rejections only mean a real contract violation.

## Custom skins

Subclass a concrete control (or `RvfControlElement`), override
`template()`/`stylesheet()`/`updateVisual()`, and register under a new
tag — gesture, event, a11y, and token machinery is inherited. Dispatch
inputs through `el.dispatchInput(input)`, never `control.dispatch`
directly — direct controller dispatches bypass `rvf-gesture` forwarding,
the staged-write flush, and sequence bookkeeping. Event `meta.sequence`
is continuous per element: a controller rebuild (descriptor/ports/id
change) does not restart the stream a consumer dedupes on.

## Usage

```ts
import { registerRvfElements } from "@ruviform/ui-components";

registerRvfElements(); // explicit mount boundary; idempotent

const fader = document.querySelector("rvf-fader")!;
fader.descriptor = { min: -60, max: 6, unit: "dB", defaultValue: -12 };
fader.value = -20;                    // controlled — no events
fader.addEventListener("rvf-gesture", (e) => {
  if (e.detail.type === "commit") model.setTarget(fader.controlId, e.detail.value);
});
```

Build for embedding: `npm run build` emits `dist/components/src/**.js`
(+ `.d.ts`) with `.js` specifiers — import the ESM files directly in a
WebView (`type="module"`), or bundle downstream. Sources also run
as-is under Node 26 type stripping (`--experimental-strip-types` era).
See `examples/showcase.html` for thin composition (ChannelStrip /
TransportBar / ParameterControl) and a two-theme consumer.

## Packaging / embed

| Surface | Contract |
| --- | --- |
| `exports["."]` | `./src/index.ts` (consumers transpile or use `dist/`) |
| `dist/` | `tsc` output — `.js` + `.d.ts`, import specifiers rewritten |
| registration | `registerRvfElements()` — no import side effects |
| WebView | plain ESM `<script type="module">`; no CSP beyond script-src needs |

## Verification boundary

**Verified headless** (this repo, `npm test`, Node 26 + happy-dom):
token resolution/diagnostics, theme isolation, control semantics/ARIA,
pointer/keyboard/IME gesture contract, controlled-value semantics,
state attributes, lifecycle/remount, skin subclassing, transport
intents.

**NOT verified here** (manual, tracked as follow-up):
real WebView2 (Windows), WKWebView (macOS), WebKitGTK (Linux) rendering
and pointer-capture behavior; high-DPI visual output; screen-reader
output; `prefers-reduced-motion`/contrast media behavior; touch input
on hardware. happy-dom lacks `matchMedia` and `hasPointerCapture` —
media queries and capture-query paths get real-browser verification
later (Issue #261 windowed work).
