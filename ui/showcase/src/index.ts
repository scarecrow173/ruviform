/**
 * `@ruviform/ui-showcase` — design system showcase + verification
 * fixtures (Issue #263).
 *
 * Layers:
 * - **Theme matrix**: {@link SHOWCASE_THEMES} — Reference Light/Dark +
 *   DAW-shape and analysis-shape verification exemplars + a
 *   compact custom skin. All plain serializable `ThemeInput`s.
 * - **Demo data**: `makeDemo*` factories — deterministic ruviform-owned
 *   synthetic sources/frames behind the viz data ports.
 * - **Demo gateways**: {@link DemoControlGateway} /
 *   {@link DemoAnalysisBackend} — generic `BindingPort` fakes (no
 *   consumer product types).
 * - **Mounts**: {@link mountShowcase} (the full matrix),
 *   {@link mountDawFixture} (Consumer A — fader/transport/automation),
 *   {@link mountAnalysisFixture} (Consumer B — waveform/spectrum/
 *   metadata queries).
 *
 * Plain TypeScript + DOM APIs only — no UI framework, runtime, or
 * renderer dependency (happy-dom is a test-only devDependency).
 */

export * from "./themes.ts";
export * from "./demo-data.ts";
export * from "./demo-gateway.ts";
export * from "./mount.ts";
export * from "./consumers.ts";
