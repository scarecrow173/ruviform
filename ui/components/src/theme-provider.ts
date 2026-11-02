/**
 * `<rvf-theme>` — scoped theme provider (Issue #257 §12.3, Issue #259).
 *
 * Wrap any subtree in `<rvf-theme>` to give it a complete, isolated
 * theme: the resolved `--rvf-*` custom properties are set on the
 * provider's own inline style and inherit downward — through light DOM
 * and into descendant shadow trees — but never outward to siblings, and
 * an enclosing theme cannot leak in because every known var is set
 * explicitly (scope isolation by construction, not by convention).
 *
 * Inputs:
 * - `base` attribute: `"light"` (default) | `"dark"`,
 * - `density` attribute: `"default"` | `"compact"`,
 * - `theme` property: a serializable {@link ThemeInput} (typed property —
 *   never JSON-in-attribute),
 * - `tokens` property: per-scope token overrides (same schema as
 *   `theme.tokens`; merged over the resolved theme).
 *
 * Diagnostics from the last resolution are exposed on
 * {@link themeDiagnostics}; failed tokens are omitted so the
 * stylesheet's literal fallback applies.
 */

import {
  applyTheme,
  clearTheme,
  type ThemeBase,
  type ThemeDensity,
  type ThemeInput,
  type TokenDiagnostic,
  type TokenInput,
} from "./tokens.ts";

export class RvfTheme extends HTMLElement {
  static observedAttributes = ["base", "density"];

  private theme_: ThemeInput | undefined;
  private tokens_: Readonly<Record<string, TokenInput>> | undefined;
  private diagnostics_: readonly TokenDiagnostic[] = [];

  /** Serializable theme input (typed property; attribute stays string-free). */
  get theme(): ThemeInput | undefined {
    return this.theme_;
  }
  set theme(t: ThemeInput | undefined) {
    this.theme_ = t === undefined ? undefined : { ...t };
    this.applyResolved();
  }

  /** Per-scope token overrides, applied over the resolved theme. */
  get tokens(): Readonly<Record<string, TokenInput>> | undefined {
    return this.tokens_;
  }
  set tokens(t: Readonly<Record<string, TokenInput>> | undefined) {
    this.tokens_ = t === undefined ? undefined : { ...t };
    this.applyResolved();
  }

  get base(): ThemeBase {
    return this.getAttribute("base") === "dark" ? "dark" : "light";
  }
  set base(v: ThemeBase) {
    this.setAttribute("base", v);
  }

  get density(): ThemeDensity {
    return this.getAttribute("density") === "compact"
      ? "compact"
      : "default";
  }
  set density(v: ThemeDensity) {
    this.setAttribute("density", v);
  }

  /** Diagnostics from the most recent theme resolution. */
  get themeDiagnostics(): readonly TokenDiagnostic[] {
    return this.diagnostics_;
  }

  connectedCallback(): void {
    // Render a slot pass-through; no shadow styling of the subtree.
    if (this.shadowRoot === null) {
      this.attachShadow({ mode: "open" }).innerHTML =
        "<style>:host{display:contents}</style><slot></slot>";
    }
    this.upgrade("theme");
    this.upgrade("tokens");
    this.applyResolved();
  }

  disconnectedCallback(): void {
    // Leaving the DOM detaches the scope anyway; clearing keeps a re-
    // connected element from carrying a stale theme if re-parented.
    clearTheme(this);
  }

  attributeChangedCallback(
    _name: string,
    oldValue: string | null,
    newValue: string | null,
  ): void {
    if (oldValue === newValue) return;
    this.applyResolved();
  }

  private upgrade(name: "theme" | "tokens"): void {
    const self = this as unknown as Record<string, unknown>;
    if (Object.prototype.hasOwnProperty.call(self, name)) {
      const value = self[name];
      delete self[name];
      self[name] = value;
    }
  }

  private applyResolved(): void {
    if (!this.isConnected) return;
    const theme: ThemeInput = {
      ...(this.theme_ ?? {}),
      base: this.theme_?.base ?? this.base,
      density: this.theme_?.density ?? this.density,
      tokens: {
        ...(this.theme_?.tokens ?? {}),
        ...(this.tokens_ ?? {}),
      },
    };
    this.diagnostics_ = applyTheme(this, theme);
  }
}
