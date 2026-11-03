/**
 * VizStyle resolution tests — the §12.5 paint contract:
 *
 * - resolved style comes from `--rvf-viz-*` tokens (same values the CSS
 *   skin consumes) + `VizStyleInput` overrides above them,
 * - invalid overrides produce diagnostics, never silent acceptance,
 * - a light→dark theme swap changes resolved paint WITHOUT touching
 *   data — the cache-key discipline asserted here is what the element
 *   tests then verify end-to-end.
 */
import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  applyTheme,
  resolveTheme,
  REFERENCE_DARK_TOKENS,
  tokenVarName,
} from "../../components/src/tokens.ts";
import { collectVizTokens, resolveVizStyle } from "../src/style.ts";
import { mount } from "./env.ts";

describe("resolveVizStyle", () => {
  it("reference defaults resolve every paint role", () => {
    const { style, diagnostics } = resolveVizStyle({});
    assert.equal(diagnostics.length, 0);
    assert.ok(style.background.length > 0);
    assert.ok(style.palette.length === 8);
    assert.ok(style.palette.every((c) => c.length > 0));
    assert.ok(style.waveform.stroke.length > 0);
    assert.ok(style.meter.clip.length > 0);
    assert.ok(style.state.gap.length > 0);
    assert.ok(style.font.includes("px"));
  });

  it("input overrides sit above tokens; bad values diagnose+fallback", () => {
    const { style, diagnostics } = resolveVizStyle(
      { "viz.waveform.stroke": "#111111" },
      {
        waveform: { stroke: "#ff0000", channelGapPx: -3 },
        gridWidth: Number.NaN,
        tick: "",
      },
    );
    assert.equal(style.waveform.stroke, "#ff0000"); // input wins
    assert.equal(style.waveform.channelGapPx, 4); // -3 → default
    assert.equal(style.gridWidth, 1); // NaN → default
    assert.ok(diagnostics.length >= 3);
    assert.ok(diagnostics.some((d) => d.path === "waveform.channelGapPx"));
  });

  it("palette overrides map per index", () => {
    const { style } = resolveVizStyle({}, { palette: ["#aaa", "#bbb"] });
    assert.equal(style.palette[0], "#aaa");
    assert.equal(style.palette[1], "#bbb");
    assert.ok((style.palette[2] ?? "").length > 0); // rest default
  });
});

describe("theme switching (light → dark)", () => {
  it("resolveTheme dark tokens change resolved paint, no data fields", () => {
    const light = resolveVizStyle(resolveTheme({ base: "light" }).tokens).style;
    const dark = resolveVizStyle(resolveTheme({ base: "dark" }).tokens).style;
    assert.notEqual(light.meter.track, dark.meter.track);
    assert.notEqual(light.grid, dark.grid);
    assert.equal(light.waveform.channelGapPx, dark.waveform.channelGapPx);
    // Sanity: the dark map actually carries viz tokens now.
    assert.ok("viz.grid" in REFERENCE_DARK_TOKENS);
  });
});

describe("collectVizTokens", () => {
  it("reads theme-scope vars through the ancestor chain", () => {
    const scope = document.createElement("div");
    applyTheme(scope, { base: "dark" });
    const inner = document.createElement("div");
    scope.append(inner);
    const el = mount("div");
    inner.append(el);
    document.body.append(scope);

    const tokens = collectVizTokens(el);
    assert.equal(tokens["viz.meter.track"], "#363d48"); // dark value
    el.remove();
    scope.remove();
  });

  it("nearer scopes override farther; instance tokens win last", () => {
    const outer = document.createElement("div");
    applyTheme(outer, { tokens: { "viz.grid": "#010203" } });
    const inner = document.createElement("div");
    inner.style.setProperty(tokenVarName("viz.grid"), "#040506");
    outer.append(inner);
    document.body.append(outer);
    const el = document.createElement("div");
    inner.append(el);

    let tokens = collectVizTokens(el);
    assert.equal(tokens["viz.grid"], "#040506"); // inner wins

    el.style.setProperty(tokenVarName("viz.grid"), "#070809");
    tokens = collectVizTokens(el);
    assert.equal(tokens["viz.grid"], "#070809"); // instance wins last
    outer.remove();
  });

  it("resolves var(--rvf-*) chains written by { ref } overrides", () => {
    const scope = document.createElement("div");
    applyTheme(scope, { tokens: { interactive: "#00ff00" } });
    document.body.append(scope);
    const el = document.createElement("div");
    scope.append(el);
    // Simulates what `el.tokens = { "viz.waveform.stroke": { ref:
    // "interactive" } }` writes through applyTokenOverrides.
    el.style.setProperty(
      tokenVarName("viz.waveform.stroke"),
      `var(${tokenVarName("interactive")}, #000000)`,
    );
    const tokens = collectVizTokens(el);
    assert.equal(tokens["viz.waveform.stroke"], "#00ff00");
    scope.remove();
  });
});
