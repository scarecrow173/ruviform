/**
 * `@ruviform/ui-core` — Issue #258 framework-neutral headless UI contract:
 * typed value mapping and the Begin/Preview/Commit/Cancel gesture state
 * machine shared by Knob/Fader/Slider/NumericInput/Seekbar/Range controls.
 *
 * This module has no DOM, UI-framework, WebView, or renderer dependencies;
 * it runs in plain TypeScript under `node --test` as well as in a WebView.
 */

export * from "./value.ts";
export * from "./interaction.ts";
