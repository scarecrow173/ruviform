/**
 * Ruviform design-token contract (Issue #257 §12.3, Issue #259).
 *
 * Token taxonomy has three public layers:
 * - `foundation`: raw visual material — type scale, spacing unit, radius,
 *   focus stroke, motion durations. Carries no product meaning.
 * - `semantic`: purpose roles — surface/text/interactive/focus/status.
 *   Light and dark themes reference the same roles.
 * - `role`: control-scoped override points (`knob.arc`, `fader.thumb`, …).
 *   The reference implementation consumes exactly these keys; a role
 *   token's default may reference a semantic token (`{ ref: "interactive" }`)
 *   so a single semantic recolor reaches every control.
 *
 * Resolution order (low → high precedence):
 *   reference defaults → theme base (light|dark) → density scaling →
 *   scope-local product theme overrides → declared instance overrides →
 *   semantic state rules (applied last, in CSS).
 *
 * Tokens are serializable data only — render callbacks live in separate
 * slot/property ports and never cross a theme boundary. Unknown keys,
 * type mismatches, non-finite numbers, and circular `{ ref }` chains are
 * reported as {@link TokenDiagnostic}s; offending entries are skipped and
 * the reference fallback applies instead of failing silently.
 */

/** Value kinds a token may carry. */
export type TokenKind = "color" | "length" | "duration" | "number" | "string";

/** Serializable token value. `number` is px for `length`, ms for `duration`. */
export type TokenValue = string | number;

/** Alias to another token key (same resolution layer). */
export interface TokenRef {
  readonly ref: string;
}

export type TokenInput = TokenValue | TokenRef;

export type TokenLayer = "foundation" | "semantic" | "role";

export interface TokenSpec {
  readonly kind: TokenKind;
  readonly layer: TokenLayer;
  /** CSS custom-property name the token resolves to. */
  readonly cssVar: string;
  /** Reference (light) default; may alias another token via `{ ref }`. */
  readonly default: TokenInput;
  /** `density: "compact"` scales numeric defaults by {@link COMPACT_FACTOR}. */
  readonly densityScalable?: boolean;
  readonly doc: string;
}

const F = "foundation" as const;
const S = "semantic" as const;
const R = "role" as const;

/**
 * The public token schema: name → spec. This record IS the public role
 * list — consumers may override any key here and must not invent
 * undocumented `rvf-*` semantics outside it.
 */
export const TOKEN_SPECS = {
  // ── foundation ───────────────────────────────────────────────
  "font.family": {
    kind: "string", layer: F, cssVar: "--rvf-font-family",
    default:
      "system-ui, 'Segoe UI', 'Hiragino Sans', 'Noto Sans', sans-serif",
    doc: "UI text font stack (system fonts only — no bundled font assets).",
  },
  "font.size": {
    kind: "length", layer: F, cssVar: "--rvf-font-size", default: 12,
    densityScalable: true, doc: "Base label/readout font size (px).",
  },
  "space.unit": {
    kind: "length", layer: F, cssVar: "--rvf-space-unit", default: 4,
    densityScalable: true, doc: "Spacing quantum (px) for padding/gaps.",
  },
  radius: {
    kind: "length", layer: F, cssVar: "--rvf-radius", default: 3,
    doc: "Corner radius (px) for tracks/thumbs/fields.",
  },
  "focus.width": {
    kind: "length", layer: F, cssVar: "--rvf-focus-width", default: 2,
    doc: "Focus ring stroke width (px).",
  },
  "motion.fast": {
    kind: "duration", layer: F, cssVar: "--rvf-motion-fast", default: 80,
    doc: "Fast state transition (ms); suppressed under reduced-motion.",
  },
  "motion.normal": {
    kind: "duration", layer: F, cssVar: "--rvf-motion-normal", default: 160,
    doc: "Normal state transition (ms); suppressed under reduced-motion.",
  },

  // ── semantic ─────────────────────────────────────────────────
  surface: {
    kind: "color", layer: S, cssVar: "--rvf-surface", default: "#f4f5f7",
    doc: "Lowest background surface.",
  },
  "surface.raised": {
    kind: "color", layer: S, cssVar: "--rvf-surface-raised",
    default: "#ffffff", doc: "Raised control surface (thumbs, fields, buttons).",
  },
  text: {
    kind: "color", layer: S, cssVar: "--rvf-text", default: "#1c1f24",
    doc: "Primary text/foreground.",
  },
  "text.dim": {
    kind: "color", layer: S, cssVar: "--rvf-text-dim", default: "#59616e",
    doc: "Secondary text (labels, readouts, units).",
  },
  border: {
    kind: "color", layer: S, cssVar: "--rvf-border", default: "#c6cbd3",
    doc: "Neutral control border/track edge.",
  },
  interactive: {
    kind: "color", layer: S, cssVar: "--rvf-interactive",
    default: "#2f6fd0", doc: "Primary interactive accent (fills, active arc).",
  },
  "interactive.hover": {
    kind: "color", layer: S, cssVar: "--rvf-interactive-hover",
    default: "#2a62b8", doc: "Interactive accent under hover.",
  },
  "interactive.active": {
    kind: "color", layer: S, cssVar: "--rvf-interactive-active",
    default: "#1f4f99", doc: "Interactive accent while pressed/dragging.",
  },
  "focus.ring": {
    kind: "color", layer: S, cssVar: "--rvf-focus-ring",
    default: "#2f6fd0", doc: "Focus-visible ring color.",
  },
  "status.error": {
    kind: "color", layer: S, cssVar: "--rvf-status-error",
    default: "#c63b3b", doc: "Invalid/error/rejected state.",
  },
  "status.warning": {
    kind: "color", layer: S, cssVar: "--rvf-status-warning",
    default: "#a8781c", doc: "Warning/stale state.",
  },
  "status.pending": {
    kind: "color", layer: S, cssVar: "--rvf-status-pending",
    default: "#5f74c9", doc: "Pending confirmation state.",
  },
  "state.disabled": {
    kind: "number", layer: S, cssVar: "--rvf-state-disabled",
    default: 0.45,
    doc: "Opacity factor applied to unavailable controls (plus shape cues).",
  },

  // ── control roles ────────────────────────────────────────────
  "control.label": {
    kind: "color", layer: R, cssVar: "--rvf-control-label",
    default: { ref: "text.dim" }, doc: "Label text color (all controls).",
  },
  "control.readout": {
    kind: "color", layer: R, cssVar: "--rvf-control-readout",
    default: { ref: "text" }, doc: "Value readout text color.",
  },
  "knob.body": {
    kind: "color", layer: R, cssVar: "--rvf-knob-body",
    default: { ref: "surface.raised" }, doc: "Knob disc body fill.",
  },
  "knob.arc": {
    kind: "color", layer: R, cssVar: "--rvf-knob-arc",
    default: { ref: "interactive" }, doc: "Knob value arc stroke.",
  },
  "knob.arc.track": {
    kind: "color", layer: R, cssVar: "--rvf-knob-arc-track",
    default: "#d5d9df", doc: "Knob arc background track stroke.",
  },
  "knob.indicator": {
    kind: "color", layer: R, cssVar: "--rvf-knob-indicator",
    default: { ref: "text" }, doc: "Knob position indicator line.",
  },
  "knob.size": {
    kind: "length", layer: R, cssVar: "--rvf-knob-size", default: 40,
    densityScalable: true, doc: "Knob diameter (px).",
  },
  "fader.track": {
    kind: "color", layer: R, cssVar: "--rvf-fader-track",
    default: "#d5d9df", doc: "Fader groove color.",
  },
  "fader.fill": {
    kind: "color", layer: R, cssVar: "--rvf-fader-fill",
    default: { ref: "interactive" }, doc: "Fader filled portion color.",
  },
  "fader.thumb": {
    kind: "color", layer: R, cssVar: "--rvf-fader-thumb",
    default: { ref: "surface.raised" }, doc: "Fader thumb fill.",
  },
  "fader.thumb.border": {
    kind: "color", layer: R, cssVar: "--rvf-fader-thumb-border",
    default: { ref: "border" }, doc: "Fader thumb edge color.",
  },
  "fader.length": {
    kind: "length", layer: R, cssVar: "--rvf-fader-length", default: 160,
    densityScalable: true, doc: "Fader track length (px, along orientation).",
  },
  "fader.thickness": {
    kind: "length", layer: R, cssVar: "--rvf-fader-thickness", default: 24,
    densityScalable: true, doc: "Fader cross-axis size incl. thumb (px).",
  },
  "toggle.track": {
    kind: "color", layer: R, cssVar: "--rvf-toggle-track",
    default: "#c2c8d1", doc: "Toggle track (off) color.",
  },
  "toggle.track.on": {
    kind: "color", layer: R, cssVar: "--rvf-toggle-track-on",
    default: { ref: "interactive" }, doc: "Toggle track (on) color.",
  },
  "toggle.thumb": {
    kind: "color", layer: R, cssVar: "--rvf-toggle-thumb",
    default: "#ffffff", doc: "Toggle thumb color.",
  },
  "toggle.width": {
    kind: "length", layer: R, cssVar: "--rvf-toggle-width", default: 36,
    densityScalable: true, doc: "Toggle track width (px).",
  },
  "toggle.height": {
    kind: "length", layer: R, cssVar: "--rvf-toggle-height", default: 20,
    densityScalable: true, doc: "Toggle track height (px).",
  },
  "seekbar.track": {
    kind: "color", layer: R, cssVar: "--rvf-seekbar-track",
    default: "#d5d9df", doc: "Seekbar unfilled track color.",
  },
  "seekbar.fill": {
    kind: "color", layer: R, cssVar: "--rvf-seekbar-fill",
    default: { ref: "interactive" }, doc: "Seekbar played-position fill.",
  },
  "seekbar.buffered": {
    kind: "color", layer: R, cssVar: "--rvf-seekbar-buffered",
    default: "#b7bfca", doc: "Seekbar buffered-ahead indicator color.",
  },
  "seekbar.height": {
    kind: "length", layer: R, cssVar: "--rvf-seekbar-height", default: 18,
    densityScalable: true, doc: "Seekbar track height (px).",
  },
  "range.track": {
    kind: "color", layer: R, cssVar: "--rvf-range-track",
    default: "#d5d9df", doc: "Range selector unfilled track color.",
  },
  "range.fill": {
    kind: "color", layer: R, cssVar: "--rvf-range-fill",
    default: { ref: "interactive" }, doc: "Range selector selected band color.",
  },
  "range.thumb": {
    kind: "color", layer: R, cssVar: "--rvf-range-thumb",
    default: { ref: "surface.raised" }, doc: "Range selector thumb fill.",
  },
  "range.thumb.border": {
    kind: "color", layer: R, cssVar: "--rvf-range-thumb-border",
    default: { ref: "border" }, doc: "Range selector thumb edge color.",
  },
  "range.height": {
    kind: "length", layer: R, cssVar: "--rvf-range-height", default: 20,
    densityScalable: true, doc: "Range selector track height (px).",
  },
  "numeric.bg": {
    kind: "color", layer: R, cssVar: "--rvf-numeric-bg",
    default: { ref: "surface.raised" }, doc: "Numeric input field background.",
  },
  "numeric.border": {
    kind: "color", layer: R, cssVar: "--rvf-numeric-border",
    default: { ref: "border" }, doc: "Numeric input field border.",
  },
  "numeric.height": {
    kind: "length", layer: R, cssVar: "--rvf-numeric-height", default: 24,
    densityScalable: true, doc: "Numeric input field height (px).",
  },
  "transport.bg": {
    kind: "color", layer: R, cssVar: "--rvf-transport-bg",
    default: { ref: "surface.raised" }, doc: "Transport button face color.",
  },
  "transport.fg": {
    kind: "color", layer: R, cssVar: "--rvf-transport-fg",
    default: { ref: "text" }, doc: "Transport button glyph color.",
  },
  "transport.active": {
    kind: "color", layer: R, cssVar: "--rvf-transport-active",
    default: { ref: "interactive" }, doc: "Lit transport button (state display).",
  },
  "transport.size": {
    kind: "length", layer: R, cssVar: "--rvf-transport-size", default: 28,
    densityScalable: true, doc: "Transport button size (px, square).",
  },

  // ── viz roles (Issue #260 — Canvas/WebGL paint roles, §12.5) ─────
  // These roles are consumed by the `@ruviform/ui-viz` renderers, not
  // by the DOM control skin. Resolving them here keeps CSS surfaces and
  // GPU paint on one resolved value set.
  "viz.background": {
    kind: "color", layer: R, cssVar: "--rvf-viz-background",
    default: { ref: "surface" }, doc: "Viz plot-area background.",
  },
  "viz.grid": {
    kind: "color", layer: R, cssVar: "--rvf-viz-grid",
    default: "#e2e6ec", doc: "Viz grid line color.",
  },
  "viz.grid.width": {
    kind: "length", layer: R, cssVar: "--rvf-viz-grid-width", default: 1,
    doc: "Viz grid line width (px).",
  },
  "viz.axis": {
    kind: "color", layer: R, cssVar: "--rvf-viz-axis",
    default: { ref: "border" }, doc: "Viz axis rule color.",
  },
  "viz.tick": {
    kind: "color", layer: R, cssVar: "--rvf-viz-tick",
    default: { ref: "text.dim" }, doc: "Viz tick/legend label color.",
  },
  "viz.series.0": {
    kind: "color", layer: R, cssVar: "--rvf-viz-series-0",
    default: { ref: "interactive" }, doc: "Series palette slot 0 (wraps).",
  },
  "viz.series.1": {
    kind: "color", layer: R, cssVar: "--rvf-viz-series-1",
    default: "#2ea37c", doc: "Series palette slot 1.",
  },
  "viz.series.2": {
    kind: "color", layer: R, cssVar: "--rvf-viz-series-2",
    default: "#d9822b", doc: "Series palette slot 2.",
  },
  "viz.series.3": {
    kind: "color", layer: R, cssVar: "--rvf-viz-series-3",
    default: "#b050b8", doc: "Series palette slot 3.",
  },
  "viz.series.4": {
    kind: "color", layer: R, cssVar: "--rvf-viz-series-4",
    default: "#c64f4f", doc: "Series palette slot 4.",
  },
  "viz.series.5": {
    kind: "color", layer: R, cssVar: "--rvf-viz-series-5",
    default: "#5a6fb8", doc: "Series palette slot 5.",
  },
  "viz.series.6": {
    kind: "color", layer: R, cssVar: "--rvf-viz-series-6",
    default: "#7d8a1e", doc: "Series palette slot 6.",
  },
  "viz.series.7": {
    kind: "color", layer: R, cssVar: "--rvf-viz-series-7",
    default: "#3d8fa8", doc: "Series palette slot 7.",
  },
  "viz.waveform.stroke": {
    kind: "color", layer: R, cssVar: "--rvf-viz-waveform-stroke",
    default: { ref: "viz.series.0" }, doc: "Waveform center/peak stroke.",
  },
  "viz.waveform.fill": {
    kind: "color", layer: R, cssVar: "--rvf-viz-waveform-fill",
    default: "#2f6fd04d", doc: "Waveform min/max envelope fill.",
  },
  "viz.waveform.rms": {
    kind: "color", layer: R, cssVar: "--rvf-viz-waveform-rms",
    default: "#2f6fd066", doc: "Waveform RMS band fill.",
  },
  "viz.meter.low": {
    kind: "color", layer: R, cssVar: "--rvf-viz-meter-low",
    default: "#3da35f", doc: "Meter fill, low segment.",
  },
  "viz.meter.mid": {
    kind: "color", layer: R, cssVar: "--rvf-viz-meter-mid",
    default: "#d9a32e", doc: "Meter fill, mid segment.",
  },
  "viz.meter.high": {
    kind: "color", layer: R, cssVar: "--rvf-viz-meter-high",
    default: "#d94f3d", doc: "Meter fill, top segment.",
  },
  "viz.meter.peak": {
    kind: "color", layer: R, cssVar: "--rvf-viz-meter-peak",
    default: { ref: "text" }, doc: "Meter held-peak tick color.",
  },
  "viz.meter.clip": {
    kind: "color", layer: R, cssVar: "--rvf-viz-meter-clip",
    default: { ref: "status.error" }, doc: "Meter clip indicator color.",
  },
  "viz.meter.track": {
    kind: "color", layer: R, cssVar: "--rvf-viz-meter-track",
    default: "#e6e9ee", doc: "Meter unfilled track color.",
  },
  "viz.spectrum.stroke": {
    kind: "color", layer: R, cssVar: "--rvf-viz-spectrum-stroke",
    default: { ref: "viz.series.0" }, doc: "Spectrum curve stroke.",
  },
  "viz.spectrum.fill": {
    kind: "color", layer: R, cssVar: "--rvf-viz-spectrum-fill",
    default: "#2f6fd033", doc: "Spectrum under-curve fill.",
  },
  "viz.selection.fill": {
    kind: "color", layer: R, cssVar: "--rvf-viz-selection-fill",
    default: "#2f6fd026", doc: "Range-selection band fill.",
  },
  "viz.selection.stroke": {
    kind: "color", layer: R, cssVar: "--rvf-viz-selection-stroke",
    default: { ref: "interactive" }, doc: "Range-selection edge stroke.",
  },
  "viz.playhead": {
    kind: "color", layer: R, cssVar: "--rvf-viz-playhead",
    default: { ref: "status.error" }, doc: "Playhead/cursor line color.",
  },
  "viz.marker": {
    kind: "color", layer: R, cssVar: "--rvf-viz-marker",
    default: { ref: "text.dim" }, doc: "Generic marker line/label color.",
  },
  "viz.gap": {
    kind: "color", layer: R, cssVar: "--rvf-viz-gap",
    default: { ref: "text.dim" }, doc: "Missing-data hatch line color.",
  },
  "viz.gap.fill": {
    kind: "color", layer: R, cssVar: "--rvf-viz-gap-fill",
    default: "#59616e14", doc: "Missing-data region wash fill.",
  },
  "viz.stale": {
    kind: "color", layer: R, cssVar: "--rvf-viz-stale",
    default: { ref: "status.warning" }, doc: "Stale-data hatch/badge color.",
  },
  "viz.unsupported": {
    kind: "color", layer: R, cssVar: "--rvf-viz-unsupported",
    default: { ref: "status.warning" }, doc: "Unsupported-capability cue color.",
  },
} as const satisfies Record<string, TokenSpec>;

/** Public token key union (the declared role list). */
export type TokenKey = keyof typeof TOKEN_SPECS;

/** Compact density multiplier applied to `densityScalable` defaults. */
export const COMPACT_FACTOR = 0.8;

export type ThemeBase = "light" | "dark";
export type ThemeDensity = "default" | "compact";

/** Serializable theme input accepted by providers/`applyTheme`. */
export interface ThemeInput {
  /** Base theme defaults to start from (default: `"light"`). */
  readonly base?: ThemeBase;
  /** Density preset applied before explicit token overrides. */
  readonly density?: ThemeDensity;
  /** Explicit token overrides; values may alias via `{ ref }`. */
  readonly tokens?: Readonly<Record<string, TokenInput>>;
}

export type TokenDiagnosticKind =
  | "unknown-token"
  | "type-mismatch"
  | "non-finite"
  | "circular-ref"
  | "unknown-ref";

export interface TokenDiagnostic {
  readonly kind: TokenDiagnosticKind;
  /** Offending token key, when attributable. */
  readonly token?: string;
  readonly detail: string;
}

/** Dark-theme overrides for semantic/role colors (sparse map). */
export const REFERENCE_DARK_TOKENS: Readonly<Record<string, TokenInput>> = {
  surface: "#1d2025",
  "surface.raised": "#262a31",
  text: "#e4e7eb",
  "text.dim": "#9aa3af",
  border: "#49505b",
  interactive: "#5a93e0",
  "interactive.hover": "#6da2e6",
  "interactive.active": "#8ab4ec",
  "focus.ring": "#5a93e0",
  "status.error": "#e06a6a",
  "status.warning": "#d9a648",
  "status.pending": "#8a9ae0",
  "knob.arc.track": "#3d434d",
  "fader.track": "#3d434d",
  "toggle.track": "#4a5160",
  "toggle.thumb": "#e4e7eb",
  "seekbar.track": "#3d434d",
  "seekbar.buffered": "#59616e",
  "range.track": "#3d434d",
  "viz.grid": "#363d48",
  "viz.series.1": "#4fbf95",
  "viz.series.2": "#e0a04f",
  "viz.series.3": "#c97ed0",
  "viz.series.4": "#e06a6a",
  "viz.series.5": "#8a9ae0",
  "viz.series.6": "#a8b545",
  "viz.series.7": "#5fb0c9",
  "viz.waveform.fill": "#5a93e04d",
  "viz.waveform.rms": "#5a93e066",
  "viz.meter.low": "#4fbf80",
  "viz.meter.mid": "#e0b44f",
  "viz.meter.high": "#e07060",
  "viz.meter.track": "#363d48",
  "viz.spectrum.fill": "#5a93e033",
  "viz.selection.fill": "#5a93e02e",
  "viz.gap.fill": "#9aa3af1a",
};

/** Map a token key to its CSS custom-property name. */
export function tokenVarName(key: string): string {
  const spec = (TOKEN_SPECS as Record<string, TokenSpec>)[key];
  return spec?.cssVar ?? `--rvf-${key.replaceAll(".", "-")}`;
}

function validateValue(
  kind: TokenKind,
  value: TokenValue,
  token: string,
): TokenDiagnostic | undefined {
  switch (kind) {
    case "color":
    case "string":
      if (typeof value !== "string" || value.length === 0) {
        return {
          kind: "type-mismatch",
          token,
          detail: `token "${token}" expects a non-empty ${kind} string, got ${typeof value}`,
        };
      }
      return undefined;
    case "number":
      if (typeof value !== "number" || !Number.isFinite(value)) {
        return {
          kind: typeof value === "number" ? "non-finite" : "type-mismatch",
          token,
          detail: `token "${token}" expects a finite number, got ${String(value)}`,
        };
      }
      return undefined;
    case "length":
    case "duration":
      if (typeof value === "number") {
        if (!Number.isFinite(value)) {
          return {
            kind: "non-finite",
            token,
            detail: `token "${token}" expects a finite number, got ${value}`,
          };
        }
        if (kind === "duration" && value < 0) {
          return {
            kind: "type-mismatch",
            token,
            detail: `duration token "${token}" must be >= 0, got ${value}`,
          };
        }
        if (kind === "length" && value < 0) {
          // Every length role is a size — a negative value produces an
          // invalid declaration that silently drops the stylesheet
          // fallback, so reject it instead of resolving garbage.
          return {
            kind: "type-mismatch",
            token,
            detail: `length token "${token}" must be >= 0, got ${value}`,
          };
        }
        return undefined;
      }
      if (typeof value === "string" && value.length > 0) return undefined;
      return {
        kind: "type-mismatch",
        token,
        detail: `token "${token}" expects a finite number or CSS ${kind} string, got ${typeof value}`,
      };
  }
}

/** Render a validated value to CSS text (`number` → px / ms / unitless). */
export function tokenCssText(kind: TokenKind, value: TokenValue): string {
  if (typeof value === "string") return value;
  if (kind === "length") return `${value}px`;
  if (kind === "duration") return `${value}ms`;
  return String(value);
}

export interface ResolvedTheme {
  /** cssVar name → resolved CSS text (complete — all known tokens). */
  readonly vars: Readonly<Record<string, string>>;
  /** Token key → resolved CSS text (same resolution, keyed by token). */
  readonly tokens: Readonly<Record<string, string>>;
  readonly diagnostics: readonly TokenDiagnostic[];
}

const isRef = (input: TokenInput): input is TokenRef =>
  typeof input === "object" && input !== null && "ref" in input;

/**
 * Resolve a theme input to concrete CSS-var text.
 *
 * Order: reference defaults → base (`light` is the default; `dark`
 * applies its sparse override map) → density scaling of non-overridden
 * scalable numeric defaults → explicit `tokens` overrides (validated;
 * `{ ref }` aliases resolved with cycle detection). A token whose
 * resolution fails is *omitted* from the output so the reference
 * stylesheet's `var(--rvf-*, <fallback>)` literal applies — the
 * diagnostic records why.
 */
export function resolveTheme(input: ThemeInput = {}): ResolvedTheme {
  const diagnostics: TokenDiagnostic[] = [];
  const specs = TOKEN_SPECS as Record<string, TokenSpec>;
  const dark = input.base === "dark" ? REFERENCE_DARK_TOKENS : {};
  const overrides = input.tokens ?? {};
  const compact = input.density === "compact";

  for (const key of Object.keys(overrides)) {
    if (specs[key] === undefined) {
      diagnostics.push({
        kind: "unknown-token",
        token: key,
        detail: `unknown token "${key}" — ignored (public role list is fixed)`,
      });
    }
  }

  const rawInputs = new Map<string, TokenInput>();
  const fallbackInputs = new Map<string, TokenInput>();
  for (const [key, spec] of Object.entries(specs)) {
    // The fallback is the themed-or-default value *before* the explicit
    // override — an invalid override must fall back to the effective
    // theme value (e.g. dark `surface`), not always the light default.
    let fb: TokenInput = dark[key] ?? spec.default;
    if (
      compact &&
      spec.densityScalable === true &&
      typeof fb === "number"
    ) {
      fb = Number((fb * COMPACT_FACTOR).toFixed(6));
    }
    fallbackInputs.set(key, fb);
    const own = overrides[key];
    rawInputs.set(key, own !== undefined ? own : fb);
  }

  // `undefined` marks a failed resolution; the token is then omitted so
  // the stylesheet fallback (reference default) applies instead.
  const memo = new Map<string, TokenValue | undefined>();
  const resolveInput = (
    spec: TokenSpec,
    input: TokenInput,
    key: string,
    chain: readonly string[],
  ): TokenValue | undefined => {
    if (isRef(input)) {
      const target = input.ref;
      if (specs[target] === undefined) {
        diagnostics.push({
          kind: "unknown-ref",
          token: key,
          detail: `token "${key}" references unknown token "${target}"`,
        });
        return undefined;
      }
      if (chain.includes(target)) {
        diagnostics.push({
          kind: "circular-ref",
          token: key,
          detail: `circular token reference: ${[...chain, target].join(" -> ")}`,
        });
        return undefined;
      }
      return resolveKey(target, [...chain, target]);
    }
    const issue = validateValue(spec.kind, input, key);
    if (issue === undefined) return input;
    diagnostics.push(issue);
    const fallback = fallbackInputs.get(key) ?? spec.default;
    if (input === fallback) return undefined;
    // An invalid override falls back to the effective themed value.
    return resolveInput(spec, fallback, key, chain);
  };
  const resolveKey = (
    key: string,
    chain: readonly string[],
  ): TokenValue | undefined => {
    if (memo.has(key)) return memo.get(key);
    const spec = specs[key];
    const raw = rawInputs.get(key);
    if (spec === undefined || raw === undefined) return undefined;
    const value = resolveInput(spec, raw, key, chain);
    memo.set(key, value);
    return value;
  };

  const vars: Record<string, string> = {};
  const tokens: Record<string, string> = {};
  for (const [key, spec] of Object.entries(specs)) {
    const v = resolveKey(key, [key]);
    if (v === undefined) continue;
    const text = tokenCssText(spec.kind, v);
    vars[spec.cssVar] = text;
    tokens[key] = text;
  }
  return { vars, tokens, diagnostics };
}

/**
 * Resolved reference (light, default-density) values — the literal
 * fallbacks embedded in `var(--rvf-*, <fallback>)` so controls render
 * correctly with no theme applied at all.
 */
export const REFERENCE_FALLBACK: Readonly<Record<string, string>> = (() => {
  const { tokens } = resolveTheme({});
  const out: Record<string, string> = {};
  for (const [key, spec] of Object.entries(TOKEN_SPECS)) {
    out[spec.cssVar] = tokens[key] ?? "";
  }
  return out;
})();

/** `var(--rvf-x, <reference fallback>)` for use inside shadow stylesheets. */
export function tv(key: TokenKey): string {
  const spec = TOKEN_SPECS[key];
  return `var(${spec.cssVar}, ${REFERENCE_FALLBACK[spec.cssVar]})`;
}

/**
 * Apply a theme to a scope element by setting every resolved `--rvf-*`
 * custom property on its inline style. Setting the full resolved map makes
 * the scope self-contained: values neither leak out to siblings nor does
 * an outer theme leak in (Issue #257 §12.3 scope isolation). Returns the
 * diagnostics produced during resolution.
 */
export function applyTheme(
  scope: HTMLElement,
  theme: ThemeInput = {},
): TokenDiagnostic[] {
  const { vars, diagnostics } = resolveTheme(theme);
  // Clear first: a token that now fails resolution is *omitted* from
  // `vars`, and any previously applied var left in place would mask the
  // stylesheet's literal fallback.
  clearTheme(scope);
  for (const [name, value] of Object.entries(vars)) {
    scope.style.setProperty(name, value);
  }
  return [...diagnostics];
}

/** Remove every `--rvf-*` custom property set by {@link applyTheme}. */
export function clearTheme(scope: HTMLElement): void {
  for (const spec of Object.values(TOKEN_SPECS)) {
    scope.style.removeProperty(spec.cssVar);
  }
}

export interface TokenOverrideResult {
  readonly diagnostics: readonly TokenDiagnostic[];
  /** cssVar names actually set — callers track them for later removal. */
  readonly applied: readonly string[];
}

/**
 * Apply declared instance-level token overrides to one element.
 * Overrides are validated against the schema; invalid entries are skipped
 * (reference/theme value remains). Instance overrides sit above scope
 * themes in the resolution order because the element's own inline style
 * always wins over inherited custom properties.
 */
export function applyTokenOverrides(
  el: HTMLElement,
  tokens: Readonly<Record<string, TokenInput>> | undefined,
): TokenOverrideResult {
  const diagnostics: TokenDiagnostic[] = [];
  const applied: string[] = [];
  const specs = TOKEN_SPECS as Record<string, TokenSpec>;
  if (tokens === undefined) return { diagnostics, applied };
  for (const [key, input] of Object.entries(tokens)) {
    const spec = specs[key];
    if (spec === undefined) {
      diagnostics.push({
        kind: "unknown-token",
        token: key,
        detail: `unknown token "${key}" — ignored (public role list is fixed)`,
      });
      continue;
    }
    if (isRef(input)) {
      // Instance-level alias: emit `var(--rvf-<target>, <fallback>)` so
      // the override tracks the ambient *scope* theme (a ref inside a
      // dark scope must not resolve to the light default) while still
      // rendering correctly with no theme applied at all. The reference
      // is validated against the schema; CSS var cycles degrade to the
      // fallback.
      const refSpec = specs[input.ref];
      if (refSpec === undefined) {
        diagnostics.push({
          kind: "unknown-ref",
          token: key,
          detail: `token "${key}" references unknown token "${input.ref}"`,
        });
        continue;
      }
      if (input.ref === key) {
        diagnostics.push({
          kind: "circular-ref",
          token: key,
          detail: `token "${key}" references itself`,
        });
        continue;
      }
      el.style.setProperty(
        spec.cssVar,
        `var(${refSpec.cssVar}, ${REFERENCE_FALLBACK[refSpec.cssVar]})`,
      );
      applied.push(spec.cssVar);
      continue;
    }
    const issue = validateValue(spec.kind, input, key);
    if (issue !== undefined) {
      diagnostics.push(issue);
      continue;
    }
    el.style.setProperty(spec.cssVar, tokenCssText(spec.kind, input));
    applied.push(spec.cssVar);
  }
  return { diagnostics, applied };
}
