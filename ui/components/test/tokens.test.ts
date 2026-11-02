/** Token contract + theme-provider tests (Issue #259). */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import "./env.ts";
import {
  applyTheme,
  clearTheme,
  resolveTheme,
  tokenVarName,
  TOKEN_SPECS,
  RvfTheme,
  registerRvfElements,
  type RvfControlElement,
} from "../src/index.ts";
import { mount, unmount } from "./env.ts";

registerRvfElements();

describe("resolveTheme", () => {
  it("resolves every declared token to a css var", () => {
    const { vars, diagnostics } = resolveTheme({});
    assert.equal(diagnostics.length, 0);
    assert.equal(Object.keys(vars).length, Object.keys(TOKEN_SPECS).length);
    assert.equal(vars["--rvf-interactive"], "#2f6fd0");
    // role refs resolve through semantics
    assert.equal(vars["--rvf-knob-arc"], "#2f6fd0");
    assert.equal(vars["--rvf-control-label"], "#59616e");
    // length tokens render px
    assert.equal(vars["--rvf-knob-size"], "40px");
    // duration renders ms
    assert.equal(vars["--rvf-motion-fast"], "80ms");
  });

  it("dark base overrides only the sparse dark map", () => {
    const { vars } = resolveTheme({ base: "dark" });
    assert.equal(vars["--rvf-surface"], "#1d2025");
    assert.equal(vars["--rvf-text"], "#e4e7eb");
    // untouched entries keep reference values
    assert.equal(vars["--rvf-knob-arc"], "#5a93e0"); // refs re-resolve to dark interactive
    assert.equal(vars["--rvf-font-size"], "12px");
  });

  it("compact density scales densityScalable numeric defaults only", () => {
    const def = resolveTheme({});
    const cmp = resolveTheme({ density: "compact" });
    assert.equal(cmp.vars["--rvf-knob-size"], "32px"); // 40 * 0.8
    assert.equal(cmp.vars["--rvf-font-size"], "9.6px"); // 12 * 0.8
    // non-scalable stays
    assert.equal(cmp.vars["--rvf-radius"], def.vars["--rvf-radius"]);
  });

  it("explicit overrides win over density but not scale", () => {
    const { vars } = resolveTheme({
      density: "compact",
      tokens: { "knob.size": 44 },
    });
    assert.equal(vars["--rvf-knob-size"], "44px");
  });

  it("reports unknown-token and skips it", () => {
    const { vars, diagnostics } = resolveTheme({
      tokens: { "made.up": "#fff" },
    });
    assert.equal(diagnostics.length, 1);
    assert.equal(diagnostics[0]?.kind, "unknown-token");
    assert.equal(diagnostics[0]?.token, "made.up");
    assert.equal(vars["--rvf-made-up"], undefined);
  });

  it("reports type-mismatch and falls back to the token default", () => {
    const { vars, diagnostics } = resolveTheme({
      tokens: { interactive: 42 },
    });
    assert.equal(diagnostics[0]?.kind, "type-mismatch");
    assert.equal(vars["--rvf-interactive"], "#2f6fd0");
  });

  it("reports non-finite numbers", () => {
    const { diagnostics } = resolveTheme({
      tokens: { "knob.size": Number.NaN },
    });
    assert.equal(diagnostics[0]?.kind, "non-finite");
  });

  it("resolves override refs and reports unknown refs", () => {
    const { vars, diagnostics } = resolveTheme({
      tokens: {
        "knob.arc": { ref: "status.error" },
        "fader.fill": { ref: "not.a.token" },
      },
    });
    assert.equal(vars["--rvf-knob-arc"], "#c63b3b");
    assert.ok(diagnostics.some((d) => d.kind === "unknown-ref"));
    // failed token omitted → stylesheet literal fallback applies
    assert.equal(vars["--rvf-fader-fill"], undefined);
  });

  it("detects circular refs", () => {
    // a → b → a
    const { diagnostics } = resolveTheme({
      tokens: {
        "knob.arc": { ref: "fader.fill" },
        "fader.fill": { ref: "knob.arc" },
      },
    });
    assert.ok(diagnostics.some((d) => d.kind === "circular-ref"));
  });
});

describe("tokenVarName", () => {
  it("maps declared keys to their css var", () => {
    assert.equal(tokenVarName("interactive"), "--rvf-interactive");
    assert.equal(tokenVarName("knob.arc.track"), "--rvf-knob-arc-track");
  });
});

describe("rvf-theme provider", () => {
  it("applies resolved vars to its own inline style", () => {
    const el = mount("rvf-theme") as RvfTheme;
    assert.equal(el.style.getPropertyValue("--rvf-surface"), "#f4f5f7");
    assert.equal(el.style.getPropertyValue("--rvf-interactive"), "#2f6fd0");
    unmount(el);
  });

  it("dark base via attribute", () => {
    const el = mount("rvf-theme", { base: "dark" }) as RvfTheme;
    assert.equal(el.style.getPropertyValue("--rvf-surface"), "#1d2025");
    unmount(el);
  });

  it("theme + tokens properties resolve together", () => {
    const el = mount("rvf-theme") as RvfTheme;
    el.theme = { base: "dark" };
    el.tokens = { interactive: "#ff0000" };
    assert.equal(el.style.getPropertyValue("--rvf-surface"), "#1d2025");
    assert.equal(el.style.getPropertyValue("--rvf-interactive"), "#ff0000");
    assert.equal(el.themeDiagnostics.length, 0);
    unmount(el);
  });

  it("scope isolation: an inner theme does not leak to siblings", () => {
    const outer = mount("rvf-theme") as RvfTheme;
    const inner = document.createElement("rvf-theme") as RvfTheme;
    const sibling = document.createElement("div");
    inner.tokens = { interactive: "#00ff00" };
    outer.append(inner, sibling);
    assert.equal(
      outer.style.getPropertyValue("--rvf-interactive"),
      "#2f6fd0",
    );
    assert.equal(
      inner.style.getPropertyValue("--rvf-interactive"),
      "#00ff00",
    );
    unmount(outer);
  });

  it("records diagnostics for bad tokens", () => {
    const el = mount("rvf-theme") as RvfTheme;
    el.tokens = { bogus: 1 };
    assert.ok(el.themeDiagnostics.some((d) => d.kind === "unknown-token"));
    unmount(el);
  });

  it("clears its vars on disconnect", () => {
    const el = mount("rvf-theme") as RvfTheme;
    unmount(el);
    assert.equal(el.style.getPropertyValue("--rvf-surface"), "");
  });
});

describe("instance token overrides on a control", () => {
  it("applies literal overrides and reports them", () => {
    const el = mount("rvf-knob") as RvfControlElement;
    el.tokens = { "knob.arc": "#123456" };
    assert.equal(
      el.style.getPropertyValue("--rvf-knob-arc"),
      "#123456",
    );
    assert.equal(el.tokenDiagnostics.length, 0);
    unmount(el);
  });

  it("{ref} emits var() tracking the ambient scope theme", () => {
    const theme = mount("rvf-theme", { base: "dark" }) as RvfTheme;
    const el = document.createElement("rvf-knob") as RvfControlElement;
    theme.append(el);
    el.tokens = { "knob.arc": { ref: "text" } };
    // aliases the dark scope's --rvf-text, not the light literal; the
    // embedded fallback keeps the control styled with no theme at all.
    assert.equal(
      el.style.getPropertyValue("--rvf-knob-arc"),
      "var(--rvf-text, #1c1f24)",
    );
    unmount(theme);
  });

  it("unknown ref and self-ref are diagnosed, not applied", () => {
    const el = mount("rvf-knob") as RvfControlElement;
    el.tokens = {
      "knob.arc": { ref: "no.such" },
      "fader.fill": { ref: "fader.fill" },
    };
    assert.equal(el.style.getPropertyValue("--rvf-knob-arc"), "");
    assert.equal(el.style.getPropertyValue("--rvf-fader-fill"), "");
    const kinds = el.tokenDiagnostics.map((d) => d.kind);
    assert.ok(kinds.includes("unknown-ref"));
    assert.ok(kinds.includes("circular-ref"));
    unmount(el);
  });

  it("replacing tokens clears previously applied vars", () => {
    const el = mount("rvf-knob") as RvfControlElement;
    el.tokens = { "knob.arc": "#123456" };
    el.tokens = { "fader.fill": "#654321" };
    assert.equal(el.style.getPropertyValue("--rvf-knob-arc"), "");
    assert.equal(el.style.getPropertyValue("--rvf-fader-fill"), "#654321");
    el.tokens = undefined;
    assert.equal(el.style.getPropertyValue("--rvf-fader-fill"), "");
    unmount(el);
  });
});

describe("applyTheme/clearTheme", () => {
  it("sets and removes every var on the scope", () => {
    const scope = document.createElement("div");
    const diags = applyTheme(scope, { base: "dark" });
    assert.equal(diags.length, 0);
    assert.equal(scope.style.getPropertyValue("--rvf-surface"), "#1d2025");
    clearTheme(scope);
    assert.equal(scope.style.getPropertyValue("--rvf-surface"), "");
  });
});
