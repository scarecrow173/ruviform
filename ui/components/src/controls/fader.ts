/**
 * `<rvf-fader>` — linear value control, `orientation="vertical"`
 * (default) or `"horizontal"`. Covers the "Fader/Slider" entry of the
 * Issue #259 control list: the same element serves both roles —
 * `rvf-fader` is the slider semantic.
 *
 * Reference Skin: groove track + filled portion + bordered thumb, label
 * and readout. Absolute pointer geometry (click jumps, drag follows),
 * wheel, keyboard, double-click reset.
 *
 * Parts: `root`, `track`, `fill`, `thumb`, `label`, `readout`.
 * Slots: `label`. Variants: `compact`. Attributes: `orientation`.
 */

import { tv } from "../tokens.ts";
import { BASE_CSS, STATE_CSS, TEXT_CSS } from "../styles.ts";
import {
  RvfControlElement,
  CONTROL_BASE_ATTRIBUTES,
} from "../control-base.ts";
import type { ControlValue, DisplayValue } from "../headless.ts";

export class RvfFader extends RvfControlElement {
  static observedAttributes = [...CONTROL_BASE_ATTRIBUTES, "orientation"];

  protected gestureMode() {
    return "absolute" as const;
  }

  protected gestureSurface(): Element {
    return this.track_ ?? this;
  }

  get orientation(): "vertical" | "horizontal" {
    return this.getAttribute("orientation") === "horizontal"
      ? "horizontal"
      : "vertical";
  }
  set orientation(o: "vertical" | "horizontal") {
    this.setAttribute("orientation", o);
  }

  protected normalizedFromPoint(e: PointerEvent): number {
    const track = this.track_;
    if (track === undefined) return this.displayValue.normalized;
    const rect = track.getBoundingClientRect();
    if (this.orientation === "horizontal") {
      return rect.width === 0 ? 0 : (e.clientX - rect.left) / rect.width;
    }
    return rect.height === 0 ? 0 : 1 - (e.clientY - rect.top) / rect.height;
  }

  protected handleControlAttribute(name: string, _v: string | null): void {
    if (name === "orientation") this.refresh();
    else this.syncPresentation();
  }

  /**
   * The default orientation is vertical while the ARIA slider default is
   * horizontal — always report the axis explicitly so AT does not
   * announce the wrong one.
   */
  protected override syncValueA11y(target: Element): void {
    super.syncValueA11y(target);
    target.setAttribute("aria-orientation", this.orientation);
  }

  private track_: HTMLElement | undefined;
  private fill_: HTMLElement | undefined;
  private thumb_: HTMLElement | undefined;
  private readout_: Element | undefined;

  protected template(): string {
    return `
      <div class="rvf-ctl" part="root">
        <slot name="label"><span class="rvf-label" part="label"></span></slot>
        <div class="rvf-track" part="track">
          <div class="rvf-fill" part="fill"></div>
          <div class="rvf-thumb" part="thumb"></div>
        </div>
        <span class="rvf-readout" part="readout"></span>
      </div>`;
  }

  protected stylesheet(): string {
    return `
      ${BASE_CSS}
      .rvf-ctl {
        display: inline-flex;
        align-items: center;
        gap: ${tv("space.unit")};
        cursor: pointer;
      }
      .rvf-ctl { flex-direction: column; }
      .rvf-track {
        position: relative;
        background: ${tv("fader.track")};
        border-radius: ${tv("radius")};
      }
      :host([orientation="horizontal"]) .rvf-track {
        width: ${tv("fader.length")};
        height: calc(${tv("fader.thickness")} / 3);
      }
      :host(:not([orientation])) .rvf-track,
      :host([orientation="vertical"]) .rvf-track {
        height: ${tv("fader.length")};
        width: calc(${tv("fader.thickness")} / 3);
      }
      .rvf-fill {
        position: absolute;
        background: ${tv("fader.fill")};
        border-radius: ${tv("radius")};
        transition: none;
      }
      :host([data-previewing]) .rvf-fill { background: ${tv("interactive.active")}; }
      :host([orientation="horizontal"]) .rvf-fill { left: 0; top: 0; bottom: 0; }
      :host(:not([orientation])) .rvf-fill,
      :host([orientation="vertical"]) .rvf-fill { left: 0; right: 0; bottom: 0; }
      .rvf-thumb {
        position: absolute;
        background: ${tv("fader.thumb")};
        border: 1px solid ${tv("fader.thumb.border")};
        border-radius: ${tv("radius")};
        box-shadow: 0 1px 2px rgb(0 0 0 / 0.2);
      }
      :host([orientation="horizontal"]) .rvf-thumb {
        width: calc(${tv("fader.thickness")} / 2);
        height: ${tv("fader.thickness")};
        top: 50%;
        transform: translate(-50%, -50%);
      }
      :host(:not([orientation])) .rvf-thumb,
      :host([orientation="vertical"]) .rvf-thumb {
        height: calc(${tv("fader.thickness")} / 2);
        width: ${tv("fader.thickness")};
        left: 50%;
        transform: translate(-50%, 50%);
      }
      ${TEXT_CSS}
      :host([variant="compact"]) .rvf-thumb { box-shadow: none; }
      :host([variant="compact"]) :is(.rvf-track, .rvf-thumb) {
        border-radius: calc(${tv("radius")} / 2);
      }
      ${STATE_CSS}
    `;
  }

  protected afterRender(): void {
    this.track_ =
      this.root.querySelector<HTMLElement>('[part~="track"]') ?? undefined;
    this.fill_ =
      this.root.querySelector<HTMLElement>('[part~="fill"]') ?? undefined;
    this.thumb_ =
      this.root.querySelector<HTMLElement>('[part~="thumb"]') ?? undefined;
    this.readout_ = this.root.querySelector('[part~="readout"]') ?? undefined;
    super.afterRender();
  }

  protected updateVisual(value: ControlValue, display: DisplayValue): void {
    const pct = `${value.normalized * 100}%`;
    if (this.orientation === "horizontal") {
      if (this.fill_ !== undefined) this.fill_.style.width = pct;
      if (this.thumb_ !== undefined) this.thumb_.style.left = pct;
    } else {
      if (this.fill_ !== undefined) this.fill_.style.height = pct;
      if (this.thumb_ !== undefined) this.thumb_.style.bottom = pct;
    }
    if (this.readout_ !== undefined) this.readout_.textContent = display.text;
  }
}
