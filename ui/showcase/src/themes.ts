/**
 * Showcase theme matrix (Issue #263 — Reference Skin / Appearance
 * 互換性検証).
 *
 * Every entry is a plain serializable {@link ThemeInput} — the same
 * shape a consumer assigns to `<rvf-theme>.theme` / `.tokens`. The two
 * product-shape exemplars are *verification samples*: they exercise the
 * token/variant surface the way a DAW-style or analysis-style consumer
 * theme would, but they are NOT the products' real skins and this
 * package holds no consumer-specific types.
 *
 * Matrix intent (#263): the same mock gesture/command scenario runs
 * unchanged across every column — appearance is data, never an API
 * fork.
 */

import type { ThemeInput } from "../../components/src/index.ts";

/** One column of the showcase matrix. */
export interface ShowcaseTheme {
  /** Stable id used in fixture handles and tests. */
  readonly id: string;
  /** Human-readable column label for the showcase page. */
  readonly label: string;
  /** The serializable theme applied to the column's `<rvf-theme>` scope. */
  readonly theme: ThemeInput;
}

export const SHOWCASE_THEMES: readonly ShowcaseTheme[] = [
  {
    id: "reference-light",
    label: "Reference Light",
    theme: { base: "light" },
  },
  {
    id: "reference-dark",
    label: "Reference Dark",
    theme: { base: "dark" },
  },
  {
    id: "daw-exemplar",
    label: "DAW-shape exemplar",
    // Verification sample with a DAW-style warm-accent dark skin —
    // exercises semantic + role + viz tokens over the dark base.
    theme: {
      base: "dark",
      tokens: {
        interactive: "#e0954f",
        "interactive.hover": "#eba862",
        "interactive.active": "#f2bd7e",
        "focus.ring": "#e0954f",
        "fader.fill": "#e0954f",
        "knob.arc": "#e0954f",
        "toggle.track.on": "#c9762e",
        "viz.series.0": "#e0954f",
        "viz.waveform.stroke": "#e0954f",
        "viz.waveform.fill": "#e0954f3d",
        "viz.waveform.rms": "#e0954f66",
        "viz.spectrum.stroke": "#e0954f",
        "viz.spectrum.fill": "#e0954f2e",
        "viz.meter.low": "#4fbf80",
        "viz.meter.mid": "#e0b44f",
        "viz.meter.high": "#e07060",
      },
    },
  },
  {
    id: "analysis-exemplar",
    label: "Analysis-shape exemplar",
    // Verification sample with an analysis-style cool-accent dark
    // skin — same token slots, different values; proves exchange is
    // data-only.
    theme: {
      base: "dark",
      tokens: {
        interactive: "#4fb8c9",
        "interactive.hover": "#62c4d4",
        "interactive.active": "#7fd0dd",
        "focus.ring": "#4fb8c9",
        "seekbar.fill": "#4fb8c9",
        "range.fill": "#4fb8c9",
        "viz.series.0": "#4fb8c9",
        "viz.series.1": "#8ad08a",
        "viz.waveform.stroke": "#4fb8c9",
        "viz.waveform.fill": "#4fb8c93d",
        "viz.waveform.rms": "#4fb8c966",
        "viz.spectrum.stroke": "#4fb8c9",
        "viz.spectrum.fill": "#4fb8c92e",
        "viz.playhead": "#e0e4ea",
        "viz.marker": "#e0c95f",
      },
    },
  },
  {
    id: "custom-skin",
    label: "Custom skin + compact",
    // A custom-skin exemplar: compact density + a `{ ref }` indirection
    // + instance-feel palette — the skin-exchange path a consumer
    // design system would take.
    theme: {
      base: "light",
      density: "compact",
      tokens: {
        interactive: "#7a4fd0",
        "interactive.hover": "#8d63d8",
        "interactive.active": "#a07ee0",
        "focus.ring": "#7a4fd0",
        "knob.arc": { ref: "interactive" },
        "fader.fill": { ref: "interactive" },
        "viz.series.0": "#7a4fd0",
        "viz.waveform.stroke": "#7a4fd0",
        "viz.waveform.fill": "#7a4fd033",
        "viz.spectrum.stroke": "#7a4fd0",
        "viz.spectrum.fill": "#7a4fd02a",
      },
    },
  },
];

export type ShowcaseThemeId = (typeof SHOWCASE_THEMES)[number]["id"];

export function showcaseTheme(id: string): ShowcaseTheme {
  const t = SHOWCASE_THEMES.find((x) => x.id === id);
  if (t === undefined) throw new Error(`unknown showcase theme "${id}"`);
  return t;
}
