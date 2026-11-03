/**
 * VizStyle — the paint contract for the Reference Viz renderers
 * (Issue #260, Issue #257 §12.5).
 *
 * VizStyle is deliberately **independent of data/query/model**: it
 * carries colors, strokes, palette, widths, and fonts only — never
 * timebase, dB range, FFT/source, or validity semantics. Mutating any
 * field must not recompute PCM/FFT/LOD/Source queries; the cache keys
 * and test fixtures enforce that.
 *
 * Resolution is one-directional:
 *
 *   token layer (`--rvf-viz-*`, same resolved values the CSS skin uses)
 *     → declared `VizStyleInput` overrides (the `appearance` property)
 *     → immutable `ResolvedVizStyle`
 *     → DrawOp colors passed to Canvas2D / WebGL2.
 *
 * Invalid entries never fail silently: every skipped value produces a
 * {@link VizStyleDiagnostic} and the token-layer value applies.
 */

import { TOKEN_SPECS } from "../../components/src/tokens.ts";
import type { AxisUnit } from "./axis.ts";

/** Declarable paint overrides — the `appearance` typed property and the
 * product-theme-independent style slot. Serializable data only. */
export interface VizStyleInput {
  readonly background?: string;
  readonly grid?: string;
  readonly gridWidth?: number;
  readonly axis?: string;
  readonly tick?: string;
  /** Tick/legend label size in px (default: `font.size` token). */
  readonly tickSize?: number;
  readonly palette?: readonly string[];
  readonly waveform?: {
    readonly stroke?: string;
    readonly fill?: string;
    readonly rms?: string;
    /** Vertical gap between channel lanes (px). */
    readonly channelGapPx?: number;
    /** Draw the RMS band over the min/max envelope (default true). */
    readonly showRms?: boolean;
  };
  readonly meter?: {
    readonly low?: string;
    readonly mid?: string;
    readonly high?: string;
    readonly peak?: string;
    readonly clip?: string;
    readonly track?: string;
    /** Horizontal/vertical gap between channel bars (px). */
    readonly channelGapPx?: number;
  };
  readonly spectrum?: {
    readonly stroke?: string;
    readonly fill?: string;
  };
  readonly overlay?: {
    readonly selectionFill?: string;
    readonly selectionStroke?: string;
    readonly playhead?: string;
    readonly marker?: string;
  };
  readonly state?: {
    readonly gap?: string;
    readonly gapFill?: string;
    readonly stale?: string;
    readonly unsupported?: string;
  };
}

export interface VizStyleDiagnostic {
  readonly path: string;
  readonly detail: string;
}

/** Immutable resolved paint — the single color source for every backend. */
export interface ResolvedVizStyle {
  readonly background: string;
  readonly grid: string;
  readonly gridWidth: number;
  readonly axis: string;
  readonly tick: string;
  /** Canvas-compatible font string (`"<size>px <family>"`). */
  readonly font: string;
  readonly tickSize: number;
  readonly palette: readonly string[];
  readonly waveform: {
    readonly stroke: string;
    readonly fill: string;
    readonly rms: string;
    readonly channelGapPx: number;
    readonly showRms: boolean;
  };
  readonly meter: {
    readonly low: string;
    readonly mid: string;
    readonly high: string;
    readonly peak: string;
    readonly clip: string;
    readonly track: string;
    readonly channelGapPx: number;
  };
  readonly spectrum: {
    readonly stroke: string;
    readonly fill: string;
  };
  readonly overlay: {
    readonly selectionFill: string;
    readonly selectionStroke: string;
    readonly playhead: string;
    readonly marker: string;
  };
  readonly state: {
    readonly gap: string;
    readonly gapFill: string;
    readonly stale: string;
    readonly unsupported: string;
  };
}

const PALETTE_KEYS = [
  "viz.series.0",
  "viz.series.1",
  "viz.series.2",
  "viz.series.3",
  "viz.series.4",
  "viz.series.5",
  "viz.series.6",
  "viz.series.7",
] as const;

function tokenDefault(key: string): string {
  // Reference defaults resolve `{ ref }` chains via resolveTheme in the
  // token module; for the style resolver we need the same resolution,
  // so reuse the module-level resolution machinery rather than reading
  // `spec.default` raw.
  return resolveTokenRef(key, new Map());
}

const DEFAULT_CACHE = new Map<string, string>();

function resolveTokenRef(
  key: string,
  overrides: Readonly<Record<string, string>> | Map<string, string>,
): string {
  const get = (k: string): string | undefined =>
    overrides instanceof Map ? overrides.get(k) : overrides[k];
  const seen = new Set<string>();
  let k = key;
  for (;;) {
    const direct = get(k);
    if (direct !== undefined) return direct;
    if (DEFAULT_CACHE.has(k)) return DEFAULT_CACHE.get(k) ?? "";
    if (seen.has(k)) return "";
    seen.add(k);
    const spec = (TOKEN_SPECS as Record<string, { default?: unknown } | undefined>)[k];
    const d = spec?.default;
    if (typeof d === "object" && d !== null && "ref" in d) {
      k = (d as { ref: string }).ref;
      continue;
    }
    const v = typeof d === "string" ? d : typeof d === "number" ? String(d) : "";
    DEFAULT_CACHE.set(k, v);
    return v;
  }
}

function color(
  tokens: Readonly<Record<string, string>>,
  key: string,
): string {
  return tokens[key] ?? tokenDefault(key);
}

function pxOf(text: string | undefined, fallback: number): number {
  if (text === undefined) return fallback;
  const t = text.trim();
  const m = /^(-?\d+(?:\.\d+)?)px$/.exec(t);
  if (m !== null) return Number.parseFloat(m[1] ?? "0");
  // Plain numbers read as px; any other CSS unit (rem/em/%) is
  // unsupported for paint and must not be misread as a px count.
  const n = Number.parseFloat(t);
  return /^-?\d+(\.\d+)?$/.test(t) && Number.isFinite(n) ? n : fallback;
}

function num(input: number | undefined): number | undefined {
  return input !== undefined && Number.isFinite(input) ? input : undefined;
}

function str(input: string | undefined): string | undefined {
  return input !== undefined && input.length > 0 ? input : undefined;
}

/**
 * Resolve the effective VizStyle.
 *
 * `tokens` is the ambient resolved token map (token key → CSS text), as
 * produced by `resolveTheme`/`collectVizTokens`; missing keys fall back
 * to reference defaults. `input` overrides sit above the token layer.
 * A token `length` value arrives as `"Npx"` text and is parsed back to
 * a number — the same value the CSS skin would compute.
 */
export function resolveVizStyle(
  tokens: Readonly<Record<string, string>>,
  input?: VizStyleInput,
): { style: ResolvedVizStyle; diagnostics: VizStyleDiagnostic[] } {
  const diagnostics: VizStyleDiagnostic[] = [];
  const bad = (path: string, detail: string) =>
    diagnostics.push({ path, detail });
  const pickColor = (
    path: string,
    inputVal: string | undefined,
    tokenVal: string,
  ): string => {
    const v = str(inputVal);
    if (inputVal !== undefined && v === undefined) {
      bad(path, `expected a non-empty color string, got ${JSON.stringify(inputVal)}`);
    }
    return v ?? tokenVal;
  };
  const pickNum = (
    path: string,
    inputVal: number | undefined,
    tokenVal: number,
  ): number => {
    const v = num(inputVal);
    if (inputVal !== undefined && v === undefined) {
      bad(path, `expected a finite number, got ${String(inputVal)}`);
    }
    return v ?? tokenVal;
  };
  const pickNonNeg = (
    path: string,
    inputVal: number | undefined,
    tokenVal: number,
  ): number => {
    const v = pickNum(path, inputVal, tokenVal);
    if (v < 0) {
      bad(path, `expected >= 0, got ${v}`);
      return tokenVal;
    }
    return v;
  };

  const palette: string[] = PALETTE_KEYS.map((k, i) => {
    const v = input?.palette?.[i];
    if (v !== undefined) {
      const s = str(v);
      if (s === undefined) {
        bad(`palette[${i}]`, "expected a non-empty color string");
      } else {
        return s;
      }
    }
    return color(tokens, k);
  });

  const tickSize = pickNonNeg(
    "tickSize",
    input?.tickSize,
    pxOf(tokens["font.size"], 12),
  );
  const font = `${tickSize}px ${tokens["font.family"] ?? resolveTokenRef("font.family", tokens)}`;

  const style: ResolvedVizStyle = {
    background: pickColor("background", input?.background, color(tokens, "viz.background")),
    grid: pickColor("grid", input?.grid, color(tokens, "viz.grid")),
    gridWidth: pickNonNeg(
      "gridWidth",
      input?.gridWidth,
      pxOf(tokens["viz.grid.width"], 1),
    ),
    axis: pickColor("axis", input?.axis, color(tokens, "viz.axis")),
    tick: pickColor("tick", input?.tick, color(tokens, "viz.tick")),
    font,
    tickSize,
    palette,
    waveform: {
      stroke: pickColor(
        "waveform.stroke",
        input?.waveform?.stroke,
        color(tokens, "viz.waveform.stroke"),
      ),
      fill: pickColor(
        "waveform.fill",
        input?.waveform?.fill,
        color(tokens, "viz.waveform.fill"),
      ),
      rms: pickColor(
        "waveform.rms",
        input?.waveform?.rms,
        color(tokens, "viz.waveform.rms"),
      ),
      channelGapPx: pickNonNeg(
        "waveform.channelGapPx",
        input?.waveform?.channelGapPx,
        4,
      ),
      showRms: input?.waveform?.showRms ?? true,
    },
    meter: {
      low: pickColor("meter.low", input?.meter?.low, color(tokens, "viz.meter.low")),
      mid: pickColor("meter.mid", input?.meter?.mid, color(tokens, "viz.meter.mid")),
      high: pickColor("meter.high", input?.meter?.high, color(tokens, "viz.meter.high")),
      peak: pickColor("meter.peak", input?.meter?.peak, color(tokens, "viz.meter.peak")),
      clip: pickColor("meter.clip", input?.meter?.clip, color(tokens, "viz.meter.clip")),
      track: pickColor("meter.track", input?.meter?.track, color(tokens, "viz.meter.track")),
      channelGapPx: pickNonNeg("meter.channelGapPx", input?.meter?.channelGapPx, 6),
    },
    spectrum: {
      stroke: pickColor(
        "spectrum.stroke",
        input?.spectrum?.stroke,
        color(tokens, "viz.spectrum.stroke"),
      ),
      fill: pickColor(
        "spectrum.fill",
        input?.spectrum?.fill,
        color(tokens, "viz.spectrum.fill"),
      ),
    },
    overlay: {
      selectionFill: pickColor(
        "overlay.selectionFill",
        input?.overlay?.selectionFill,
        color(tokens, "viz.selection.fill"),
      ),
      selectionStroke: pickColor(
        "overlay.selectionStroke",
        input?.overlay?.selectionStroke,
        color(tokens, "viz.selection.stroke"),
      ),
      playhead: pickColor(
        "overlay.playhead",
        input?.overlay?.playhead,
        color(tokens, "viz.playhead"),
      ),
      marker: pickColor(
        "overlay.marker",
        input?.overlay?.marker,
        color(tokens, "viz.marker"),
      ),
    },
    state: {
      gap: pickColor("state.gap", input?.state?.gap, color(tokens, "viz.gap")),
      gapFill: pickColor(
        "state.gapFill",
        input?.state?.gapFill,
        color(tokens, "viz.gap.fill"),
      ),
      stale: pickColor("state.stale", input?.state?.stale, color(tokens, "viz.stale")),
      unsupported: pickColor(
        "state.unsupported",
        input?.state?.unsupported,
        color(tokens, "viz.unsupported"),
      ),
    },
  };
  return { style, diagnostics };
}

/** Resolved values when no theme/appearance applies — reference light
 * defaults read straight out of `TOKEN_SPECS`. (Declared after
 * `resolveVizStyle` so module init can't hit a TDZ on its inputs.) */
export const REFERENCE_VIZ_STYLE: ResolvedVizStyle = resolveVizStyle(
  {},
  undefined,
).style;

/**
 * Collect the ambient `--rvf-*` values that apply to `el` by walking
 * the composed ancestor chain (through shadow hosts). Nearer scopes
 * override farther ones; the element's own inline vars — set by the
 * `tokens` instance-override property — win last. Returns a token-key →
 * CSS-text map for {@link resolveVizStyle}.
 *
 * This reads the *token contract* (inline vars applied by `<rvf-theme>`
 * and `tokens`), not arbitrary author CSS — Canvas/WebGL paint cannot
 * consume stylesheets, so the resolved values come from the same
 * resolution inputs on purpose (§12.5).
 */
export function collectVizTokens(el: Element): Record<string, string> {
  const chain: Element[] = [];
  for (
    let node: Element | null = el;
    node !== null;
    node = composedParent(node)
  ) {
    chain.unshift(node);
  }
  const raw: Record<string, string> = {};
  for (const node of chain) {
    const style = (node as HTMLElement).style;
    if (style === undefined) continue;
    for (const [key, spec] of Object.entries(TOKEN_SPECS)) {
      const v = style.getPropertyValue(spec.cssVar).trim();
      if (v !== "") raw[key] = v;
    }
  }
  // Instance `{ ref }` overrides land as literal `var(--rvf-target, fb)`
  // text — paint consumes resolved values only, so substitute chains
  // against the collected map (cycle → fallback; unresolvable + no
  // fallback → the key drops to its token default).
  const cssVarToKey = new Map<string, string>();
  for (const [key, spec] of Object.entries(TOKEN_SPECS)) {
    cssVarToKey.set(spec.cssVar, key);
  }
  const resolveVal = (v: string): string => {
    const seen = new Set<string>();
    let out = v;
    for (;;) {
      const m = /var\(\s*(--[\w-]+)\s*(?:,\s*([^)]*))?\)/.exec(out);
      if (m === null) return out;
      const name = m[1] ?? "";
      const fb = m[2]?.trim();
      let rep: string;
      const key = cssVarToKey.get(name);
      const target = key === undefined ? undefined : raw[key];
      if (seen.has(name) || target === undefined) {
        rep = fb ?? "";
      } else {
        seen.add(name);
        rep = target;
      }
      out = out.slice(0, m.index) + rep + out.slice(m.index + m[0].length);
      if (seen.size > 16) return ""; // pathological nesting → drop
    }
  };
  const out: Record<string, string> = {};
  for (const [key, v] of Object.entries(raw)) {
    const r = resolveVal(v).trim();
    if (r !== "") out[key] = r;
  }
  return out;
}

function composedParent(el: Element): Element | null {
  const parent = el.parentElement;
  if (parent !== null) return parent;
  const root = el.getRootNode();
  return root instanceof ShadowRoot ? root.host : null;
}
