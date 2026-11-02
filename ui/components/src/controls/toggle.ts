/**
 * `<rvf-toggle>` — binary switch (Issue #259).
 *
 * `role="switch"` + `aria-checked`; click / Space / Enter flips the value
 * through the shared atomic gesture (`set` → begin+commit), so toggles
 * emit the same `rvf-gesture` contract as every other control. The value
 * is binary (`checked` property); the descriptor is fixed to
 * `{min:0, max:1, step:1}` — toggle semantics are inherently two-state.
 *
 * Parts: `root`, `track`, `thumb`, `label`. Slots: `label`.
 * Variants: `compact`.
 */

import { tv } from "../tokens.ts";
import { BASE_CSS, STATE_CSS, TEXT_CSS } from "../styles.ts";
import {
  RvfControlElement,
  CONTROL_BASE_ATTRIBUTES,
} from "../control-base.ts";
import type { ControlValue, DisplayValue, ValueDescriptor } from "../headless.ts";

const TOGGLE_DESCRIPTOR: ValueDescriptor = { min: 0, max: 1, step: 1 };

export class RvfToggle extends RvfControlElement {
  static observedAttributes = [...CONTROL_BASE_ATTRIBUTES];

  protected gestureMode() {
    return "none" as const; // no drag surface — click/Space/Enter only
  }

  protected defaultDescriptor(): ValueDescriptor {
    return TOGGLE_DESCRIPTOR;
  }

  /**
   * The range is part of the toggle's identity — a consumer-supplied
   * descriptor would break the two-state contract, so it is ignored.
   */
  override get descriptor(): ValueDescriptor {
    return TOGGLE_DESCRIPTOR;
  }
  override set descriptor(_d: ValueDescriptor) {
    // intentionally inert: binary range is not consumer-configurable
  }

  /** Binary state. `checked === true` is plain value 1. */
  get checked(): boolean {
    return this.value >= 0.5;
  }
  set checked(v: boolean) {
    this.value = v ? 1 : 0;
  }

  protected bindGestures(): void {
    // No pointer-drag surface: click + Space/Enter only.
  }

  connectedCallback(): void {
    super.connectedCallback();
    this.addEventListener("click", this.onClick_);
    this.addEventListener("keydown", this.onToggleKey_);
  }

  private onClick_ = (e: MouseEvent): void => {
    e.preventDefault();
    this.flip("pointer");
  };

  private onToggleKey_ = (e: KeyboardEvent): void => {
    if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      this.flip("keyboard");
    }
  };

  private flip(source: "pointer" | "keyboard"): void {
    this.dispatchInput({
      type: "set",
      value: { plain: this.checked ? 0 : 1 },
      source,
    });
  }

  protected syncValueA11y(target: Element): void {
    target.setAttribute("role", "switch");
    target.setAttribute("aria-checked", this.checked ? "true" : "false");
    this.setOrRemoveA11y(target);
  }

  private setOrRemoveA11y(target: Element): void {
    const set = (name: string, on: boolean) => {
      if (on) target.setAttribute(name, "true");
      else target.removeAttribute(name);
    };
    set("aria-disabled", this.disabled);
    set("aria-readonly", this.readOnly);
    set("aria-invalid", this.invalid);
    set("aria-busy", this.pending);
  }

  private track_: HTMLElement | undefined;
  private thumb_: HTMLElement | undefined;

  protected template(): string {
    return `
      <div class="rvf-ctl" part="root">
        <span class="rvf-track" part="track"><span class="rvf-thumb" part="thumb"></span></span>
        <slot name="label"><span class="rvf-label" part="label"></span></slot>
      </div>`;
  }

  protected stylesheet(): string {
    return `
      ${BASE_CSS}
      .rvf-ctl {
        display: inline-flex;
        flex-direction: row;
        align-items: center;
        gap: ${tv("space.unit")};
        cursor: pointer;
      }
      .rvf-track {
        display: inline-flex;
        align-items: center;
        width: ${tv("toggle.width")};
        height: ${tv("toggle.height")};
        border-radius: ${tv("toggle.height")};
        background: ${tv("toggle.track")};
        transition: background ${tv("motion.fast")} ease;
        padding: 2px;
      }
      :host([data-previewing]) .rvf-track { background: ${tv("interactive.active")}; }
      .rvf-thumb {
        width: calc(${tv("toggle.height")} - 4px);
        height: calc(${tv("toggle.height")} - 4px);
        border-radius: 50%;
        background: ${tv("toggle.thumb")};
        box-shadow: 0 1px 2px rgb(0 0 0 / 0.25);
        transition: transform ${tv("motion.fast")} ease;
      }
      ${TEXT_CSS}
      :host([variant="compact"]) .rvf-track {
        width: calc(${tv("toggle.width")} * 0.8);
        height: calc(${tv("toggle.height")} * 0.8);
      }
      :host([variant="compact"]) .rvf-thumb {
        width: calc((${tv("toggle.height")} * 0.8) - 4px);
        height: calc((${tv("toggle.height")} * 0.8) - 4px);
      }
      ${STATE_CSS}
    `;
  }

  protected afterRender(): void {
    this.track_ =
      this.root.querySelector<HTMLElement>('[part~="track"]') ?? undefined;
    this.thumb_ =
      this.root.querySelector<HTMLElement>('[part~="thumb"]') ?? undefined;
    super.afterRender();
  }

  protected updateVisual(value: ControlValue, _display: DisplayValue): void {
    const on = value.plain >= 0.5;
    if (this.track_ !== undefined) {
      this.track_.style.background = on
        ? `var(--rvf-toggle-track-on, #2f6fd0)`
        : "";
    }
    if (this.thumb_ !== undefined) {
      this.thumb_.style.transform = on
        ? `translateX(calc(${tv("toggle.width")} - ${tv("toggle.height")}))`
        : "translateX(0)";
    }
  }
}
