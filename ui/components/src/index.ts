/**
 * `@ruviform/ui-components` — framework-neutral Web Components
 * (Issue #259).
 *
 * Public surface:
 * - the seven Reference-Skin value controls plus the transport and the
 *   `<rvf-theme>` scope provider,
 * - {@link registerRvfElements} — the explicit, idempotent mount
 *   boundary; importing this module has NO registration side effects,
 * - the DOM event contract (`rvf-gesture`, `rvf-transport`),
 * - the design-token contract (`resolveTheme`, `applyTheme`, the token
 *   schema and diagnostics),
 * - the presentation base classes for custom skins.
 *
 * Framework-neutral by construction: plain TypeScript + DOM APIs only —
 * no framework runtime, no compile step requirement beyond TypeScript.
 */

// The headless contract is part of the package's public API: consumers
// name `InteractionEvent`, `ValueDescriptor`, `ControlInput`, etc. when
// handling `rvf-gesture` events and assigning typed properties.
export * from "./headless.ts";

export { RvfElement } from "./element.ts";
export {
  RvfControlElement,
  CONTROL_BASE_ATTRIBUTES,
  type ControlGestureMode,
} from "./control-base.ts";
export {
  attachGestureInput,
  dispatchValueKey,
  WHEEL_IDLE_MS,
  type GestureBinding,
  type GestureEndpoint,
  type GestureGeometry,
} from "./gesture-input.ts";
export {
  RVF_GESTURE_EVENT,
  RVF_TRANSPORT_EVENT,
  emitGestureEvent,
  emitTransportEvent,
  withSequenceOffset,
  type RvfGestureEvent,
  type RvfTransportEvent,
  type TransportAction,
  type TransportIntent,
} from "./events.ts";
export {
  TOKEN_SPECS,
  REFERENCE_DARK_TOKENS,
  REFERENCE_FALLBACK,
  COMPACT_FACTOR,
  applyTheme,
  clearTheme,
  resolveTheme,
  tokenVarName,
  tokenCssText,
  tv,
  type ResolvedTheme,
  type ThemeBase,
  type ThemeDensity,
  type ThemeInput,
  type TokenDiagnostic,
  type TokenDiagnosticKind,
  type TokenInput,
  type TokenKey,
  type TokenKind,
  type TokenRef,
  type TokenSpec,
  type TokenValue,
} from "./tokens.ts";

export { RvfTheme } from "./theme-provider.ts";
export { RvfKnob } from "./controls/knob.ts";
export { RvfFader } from "./controls/fader.ts";
export { RvfSeekbar } from "./controls/seekbar.ts";
export { RvfToggle } from "./controls/toggle.ts";
export { RvfNumericInput } from "./controls/numeric-input.ts";
export {
  RvfTransport,
  type TransportCapabilities,
} from "./controls/transport.ts";
export { RvfRangeSelector } from "./controls/range-selector.ts";

import { RvfTheme } from "./theme-provider.ts";
import { RvfKnob } from "./controls/knob.ts";
import { RvfFader } from "./controls/fader.ts";
import { RvfSeekbar } from "./controls/seekbar.ts";
import { RvfToggle } from "./controls/toggle.ts";
import { RvfNumericInput } from "./controls/numeric-input.ts";
import { RvfTransport } from "./controls/transport.ts";
import { RvfRangeSelector } from "./controls/range-selector.ts";

/** Tag-name → constructor registry (single source for registration). */
export const RVF_ELEMENTS = {
  "rvf-theme": RvfTheme,
  "rvf-knob": RvfKnob,
  "rvf-fader": RvfFader,
  "rvf-seekbar": RvfSeekbar,
  "rvf-toggle": RvfToggle,
  "rvf-numeric-input": RvfNumericInput,
  "rvf-transport": RvfTransport,
  "rvf-range-selector": RvfRangeSelector,
} as const;

/**
 * Define every Ruviform element on `customElements`. Idempotent — safe
 * to call on every page/mount; already-registered tags are skipped so
 * custom skins registered under the same name are never clobbered.
 */
export function registerRvfElements(
  registry: CustomElementRegistry = customElements,
): void {
  for (const [tag, ctor] of Object.entries(RVF_ELEMENTS)) {
    if (registry.get(tag) === undefined) {
      registry.define(tag, ctor);
    }
  }
}
