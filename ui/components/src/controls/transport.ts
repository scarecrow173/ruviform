/**
 * `<rvf-transport>` — transport buttons: Play / Pause / Resume / Stop
 * (Issue #259).
 *
 * The element *displays* capability and state and *emits intents* — it
 * contains no transport model, RT clock, undo stack, or DAW control
 * (§12/§13 boundary):
 * - `state` (attribute/property) is the consumer-declared transport
 *   display state; the matching button lights up via `data-active`.
 * - `capabilities` (typed property) marks each action possible or not —
 *   impossible actions render disabled and never emit.
 * - A click on an enabled button emits one `rvf-transport` intent
 *   (`{action, controlId, sequence}`); intents are not command receipts.
 *
 * Parts: `root`, `play`, `pause`, `resume`, `stop`. Slots: `label`.
 * Buttons are real `<button>`s — keyboard (Enter/Space) and ARIA come
 * free; group labelling uses the `label` attribute.
 */

import { tv } from "../tokens.ts";
import { BASE_CSS, STATE_CSS, TEXT_CSS } from "../styles.ts";
import { RvfElement } from "../element.ts";
import {
  emitTransportEvent,
  type TransportAction,
} from "../events.ts";

/** Declared per-action capability (missing keys default to `true`). */
export interface TransportCapabilities {
  readonly play?: boolean;
  readonly pause?: boolean;
  readonly resume?: boolean;
  readonly stop?: boolean;
}

const ACTIONS: readonly TransportAction[] = [
  "play",
  "pause",
  "resume",
  "stop",
];

const GLYPHS: Record<TransportAction, string> = {
  play: '<path d="M9 6 L18 12 L9 18 Z" />',
  pause: '<rect x="8" y="6" width="3" height="12"/><rect x="13" y="6" width="3" height="12"/>',
  resume:
    '<path d="M11 6 L20 12 L11 18 Z"/><rect x="6" y="6" width="2.4" height="12"/>',
  stop: '<rect x="7" y="7" width="10" height="10"/>',
};

const ACTION_LABELS: Record<TransportAction, string> = {
  play: "Play",
  pause: "Pause",
  resume: "Resume",
  stop: "Stop",
};

export class RvfTransport extends RvfElement {
  static observedAttributes = ["label", "state", "disabled"];

  private seq_ = 0;
  private capabilities_: TransportCapabilities | undefined;
  private buttons_ = new Map<TransportAction, HTMLButtonElement>();

  /**
   * Per-action capability flags. `false` disables the action's button
   * (aria-disabled + inert); missing actions default to capable.
   */
  get capabilities(): TransportCapabilities {
    return { ...this.capabilities_ };
  }
  set capabilities(c: TransportCapabilities | undefined) {
    this.capabilities_ = c === undefined ? undefined : { ...c };
    this.syncPresentation();
  }

  /** Consumer-declared transport display state (`playing`, …). */
  get state(): string {
    return this.getAttribute("state") ?? "stopped";
  }
  set state(v: string) {
    this.setAttribute("state", v);
  }

  get disabled(): boolean {
    return this.boolAttr("disabled");
  }
  set disabled(v: boolean) {
    this.setBoolAttr("disabled", v);
  }

  get label(): string {
    return this.getAttribute("label") ?? "";
  }
  set label(v: string) {
    if (v === "") this.removeAttribute("label");
    else this.setAttribute("label", v);
  }

  /** Monotonic intent sequence (per element, starts at 1). */
  get sequence(): number {
    return this.seq_;
  }

  attributeChangedCallback(
    _name: string,
    oldValue: string | null,
    newValue: string | null,
  ): void {
    if (oldValue === newValue) return;
    this.syncPresentation();
  }

  connectedCallback(): void {
    this.upgradeProperty("capabilities");
    super.connectedCallback();
  }

  protected template(): string {
    const buttons = ACTIONS.map(
      (a) => `
      <button class="rvf-btn" part="${a}" type="button" data-action="${a}"
        aria-label="${ACTION_LABELS[a]}">
        <svg viewBox="0 0 24 24" aria-hidden="true">${GLYPHS[a]}</svg>
      </button>`,
    ).join("");
    return `
      <div class="rvf-ctl" part="root" role="group">
        <slot name="label"><span class="rvf-label" part="label"></span></slot>
        <span class="rvf-btns">${buttons}</span>
      </div>`;
  }

  protected stylesheet(): string {
    return `
      ${BASE_CSS}
      .rvf-ctl {
        display: inline-flex;
        align-items: center;
        gap: ${tv("space.unit")};
      }
      .rvf-btns { display: inline-flex; gap: calc(${tv("space.unit")} / 2); }
      .rvf-btn {
        width: ${tv("transport.size")};
        height: ${tv("transport.size")};
        display: inline-flex;
        align-items: center;
        justify-content: center;
        padding: 4px;
        border: 1px solid ${tv("border")};
        border-radius: ${tv("radius")};
        background: ${tv("transport.bg")};
        color: ${tv("transport.fg")};
        cursor: pointer;
        transition: background ${tv("motion.fast")} ease,
          color ${tv("motion.fast")} ease;
      }
      .rvf-btn svg { width: 100%; height: 100%; fill: currentColor; }
      .rvf-btn:hover:not([disabled]) { background: ${tv("surface")}; }
      .rvf-btn:active:not([disabled]) {
        background: ${tv("interactive.active")};
        color: ${tv("surface.raised")};
      }
      .rvf-btn:focus-visible {
        outline: ${tv("focus.width")} solid ${tv("focus.ring")};
        outline-offset: 1px;
      }
      .rvf-btn[disabled] {
        opacity: ${tv("state.disabled")};
        cursor: not-allowed;
      }
      .rvf-btn[data-active] {
        background: ${tv("transport.active")};
        border-color: ${tv("transport.active")};
        color: ${tv("surface.raised")};
      }
      ${TEXT_CSS}
      ${STATE_CSS}
    `;
  }

  protected afterRender(): void {
    this.buttons_.clear();
    for (const a of ACTIONS) {
      const b =
        this.root.querySelector<HTMLButtonElement>(`[part~="${a}"]`) ??
        undefined;
      if (b !== undefined) {
        this.buttons_.set(a, b);
        // detail === 0 marks a keyboard-initiated click (Enter/Space).
        b.addEventListener("click", (e) =>
          this.activate(a, (e as MouseEvent).detail === 0 ? "keyboard" : "pointer"),
        );
      }
    }
  }

  private activate(action: TransportAction, source: string): void {
    // Never emit for a non-capable action — regardless of how the input
    // event arrived (a disabled native button normally swallows clicks,
    // but the guard belongs at the emission boundary).
    const b = this.buttons_.get(action);
    if (b === undefined || b.disabled) return;
    this.seq_ += 1;
    emitTransportEvent(this, {
      action,
      controlId:
        this.getAttribute("control-id") ??
        (this.id !== "" ? this.id : this.localName),
      sequence: this.seq_,
      source,
    });
  }

  protected syncPresentation(): void {
    // `state` marks which action the transport is *in* — `playing` lights
    // play, `stopped` lights stop, `paused` lights pause (the state the
    // transport sits in; resume remains an available affordance).
    const lit: string | undefined = {
      playing: "play",
      paused: "pause",
      stopped: "stop",
    }[this.state];
    for (const [a, b] of this.buttons_) {
      const capable = this.capabilities_?.[a] !== false;
      b.disabled = this.disabled || !capable;
      b.setAttribute("aria-disabled", String(this.disabled || !capable));
      if (a === lit) {
        b.setAttribute("data-active", "");
      } else {
        b.removeAttribute("data-active");
      }
    }
    const ctl = this.root.querySelector('[part~="root"]');
    if (ctl !== null) {
      this.syncLabel(ctl);
      // Pre-render sync may have placed the label on the host; the root
      // part is the labelled widget.
      this.removeAttribute("aria-label");
    }
    const labelPart = this.root.querySelector('[part~="label"]');
    if (labelPart !== null) labelPart.textContent = this.label;
  }
}
