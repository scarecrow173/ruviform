/**
 * Shared Custom-Element base for Ruviform UI components.
 *
 * Responsibilities kept here and nowhere else:
 * - shadow-root attach + stylesheet/template render (re-renderable when a
 *   structural attribute changes),
 * - connect/disconnect lifecycle with a cleanup task list (timers,
 *   observers — anything that must not outlive the element),
 * - the "upgrade property" dance for values set before the element was
 *   defined/upgraded,
 * - HTML-presence boolean attribute convention (Issue #257 §13.2:
 *   presence means `true`; no `="false"` string semantics).
 *
 * Heavy initialization never runs in constructors: DOM structure is built
 * on first connect, and registration is always via the explicit
 * `registerRvfElements()` entry point (mount boundary stays explicit).
 */
import {
  applyTokenOverrides,
  type TokenDiagnostic,
  type TokenInput,
} from "./tokens.ts";

export abstract class RvfElement extends HTMLElement {
  protected readonly root: ShadowRoot;
  /**
   * Teardown tasks run on EVERY disconnect — the list is not consumed,
   * so registered functions must be idempotent (a remountable element
   * disconnects more than once).
   */
  private readonly cleanups: Array<() => void> = [];
  private rendered = false;
  private tokens_: Record<string, TokenInput> | undefined;
  private appliedTokenVars_: string[] = [];
  private tokenDiagnostics_: readonly TokenDiagnostic[] = [];

  constructor() {
    super();
    this.root = this.attachShadow({ mode: "open" });
  }

  connectedCallback(): void {
    this.upgradeProperty("tokens");
    this.renderOnce();
    this.syncPresentation();
  }

  disconnectedCallback(): void {
    for (const fn of this.cleanups) fn();
  }

  /** Register an idempotent teardown run on every disconnect. */
  protected addCleanup(fn: () => void): void {
    this.cleanups.push(fn);
  }

  /** Diagnostics from the last `tokens` override application. */
  get tokenDiagnostics(): readonly TokenDiagnostic[] {
    return this.tokenDiagnostics_;
  }

  /**
   * Declared instance-level token overrides (top of the resolution
   * order — above theme scopes and reference defaults). Keys outside
   * the public role list or wrongly typed values are skipped and
   * reported on {@link tokenDiagnostics}.
   */
  get tokens(): Readonly<Record<string, TokenInput>> | undefined {
    return this.tokens_;
  }
  set tokens(t: Readonly<Record<string, TokenInput>> | undefined) {
    this.tokens_ = t === undefined ? undefined : { ...t };
    for (const name of this.appliedTokenVars_) {
      this.style.removeProperty(name);
    }
    const { diagnostics, applied } = applyTokenOverrides(this, t);
    this.tokenDiagnostics_ = diagnostics;
    this.appliedTokenVars_ = [...applied];
  }

  /** Render the shadow tree once; call {@link refresh} to rebuild. */
  protected renderOnce(): void {
    if (this.rendered) return;
    this.rendered = true;
    this.refresh();
  }

  /**
   * Rebuild the shadow DOM (structural attribute changed). Shadow-internal
   * listeners die with the old nodes and are re-installed by
   * {@link afterRender}.
   */
  protected refresh(): void {
    this.renderShadow(this.template(), this.stylesheet());
    this.afterRender();
    this.syncPresentation();
  }

  protected renderShadow(html: string, css: string): void {
    this.root.innerHTML = `<style>${css}</style>${html}`;
  }

  /** Shadow tree markup (without the `<style>` wrapper). */
  protected abstract template(): string;
  /** Shadow tree stylesheet text. */
  protected abstract stylesheet(): string;
  /** Post-render hook: cache part references, bind shadow listeners. */
  protected afterRender(): void {}
  /** Sync attributes/property state into DOM structure and ARIA. */
  protected syncPresentation(): void {}

  /** HTML boolean-attribute convention: presence means `true`. */
  protected boolAttr(name: string): boolean {
    return this.hasAttribute(name);
  }
  protected setBoolAttr(name: string, on: boolean): void {
    if (on) this.setAttribute(name, "");
    else this.removeAttribute(name);
  }

  /**
   * Adopt a property assigned before the element was upgraded/defined so
   * typed setters run with the pre-assigned value (standard upgrade
   * pattern; keeps `el.descriptor = d` working pre-registration).
   */
  protected upgradeProperty<K extends keyof this>(name: K): void {
    if (Object.prototype.hasOwnProperty.call(this, name)) {
      const value = this[name];
      delete this[name];
      this[name] = value;
    }
  }

  /**
   * Apply the `label` attribute to a target element's `aria-label`. An
   * externally managed `aria-labelledby` wins; with neither present the
   * attribute is removed so no stale name remains.
   */
  protected syncLabel(target: Element): void {
    if (this.hasAttribute("aria-labelledby")) return;
    const label = this.getAttribute("label");
    if (label !== null && label !== "") {
      target.setAttribute("aria-label", label);
    } else {
      target.removeAttribute("aria-label");
    }
  }
}
