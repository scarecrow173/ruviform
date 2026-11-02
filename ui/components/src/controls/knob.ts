/**
 * `<rvf-knob>` — rotary value control (Issue #259).
 *
 * Reference Skin: SVG body + 270° value arc + position indicator, label
 * and readout. Drag up/down (relative mode: `dragSpanPx` covers the full
 * range), wheel, arrows/PageUp/PageDown/Home/End, double-click reset —
 * all through the shared headless gesture contract.
 *
 * Parts: `root`, `dial`, `arc`, `arc-track`, `indicator`, `label`,
 * `readout`. Slots: `label`. Variants: `compact`.
 */

import { tv } from "../tokens.ts";
import { BASE_CSS, STATE_CSS, TEXT_CSS } from "../styles.ts";
import {
  RvfControlElement,
  CONTROL_BASE_ATTRIBUTES,
} from "../control-base.ts";
import type { ControlValue, DisplayValue } from "../headless.ts";

const ARC_DEGREES = 270;
const ARC_START_DEG = 135;
const RADIUS = 15.5;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;
const ARC_LEN = (ARC_DEGREES / 360) * CIRCUMFERENCE;

export class RvfKnob extends RvfControlElement {
  static observedAttributes = [...CONTROL_BASE_ATTRIBUTES];

  protected gestureMode() {
    return "relative" as const;
  }

  protected gestureSurface(): Element {
    return this.dial_ ?? this;
  }

  private dial_: Element | undefined;
  private arcFill_: SVGCircleElement | undefined;
  private indicator_: SVGGElement | undefined;
  private readout_: Element | undefined;

  protected template(): string {
    return `
      <div class="rvf-ctl" part="root">
        <slot name="label"><span class="rvf-label" part="label"></span></slot>
        <svg class="rvf-dial" part="dial" viewBox="0 0 40 40" aria-hidden="true">
          <circle class="rvf-arc-track" part="arc-track"
            cx="20" cy="20" r="${RADIUS}" fill="none"
            stroke-dasharray="${ARC_LEN} ${CIRCUMFERENCE}"
            transform="rotate(${ARC_START_DEG} 20 20)" />
          <circle class="rvf-arc" part="arc"
            cx="20" cy="20" r="${RADIUS}" fill="none"
            stroke-dasharray="0 ${CIRCUMFERENCE}"
            transform="rotate(${ARC_START_DEG} 20 20)" />
          <g class="rvf-indicator" part="indicator">
            <line x1="20" y1="20" x2="20" y2="8" />
          </g>
        </svg>
        <span class="rvf-readout" part="readout"></span>
      </div>`;
  }

  protected stylesheet(): string {
    return `
      ${BASE_CSS}
      .rvf-ctl {
        display: inline-flex;
        flex-direction: column;
        align-items: center;
        gap: ${tv("space.unit")};
        cursor: ns-resize;
      }
      .rvf-dial {
        width: ${tv("knob.size")};
        height: ${tv("knob.size")};
        display: block;
        border-radius: 50%;
        background: ${tv("knob.body")};
        box-shadow: inset 0 0 0 1px ${tv("border")};
        transition: box-shadow ${tv("motion.fast")} ease;
      }
      .rvf-arc-track { stroke: ${tv("knob.arc.track")}; stroke-width: 3; stroke-linecap: round; }
      .rvf-arc { stroke: ${tv("knob.arc")}; stroke-width: 3; stroke-linecap: round; }
      :host([data-previewing]) .rvf-arc { stroke: ${tv("interactive.active")}; }
      .rvf-indicator line { stroke: ${tv("knob.indicator")}; stroke-width: 2; stroke-linecap: round; }
      ${TEXT_CSS}
      :host([variant="compact"]) .rvf-dial {
        width: calc(${tv("knob.size")} * 0.75);
        height: calc(${tv("knob.size")} * 0.75);
      }
      ${STATE_CSS}
    `;
  }

  protected afterRender(): void {
    this.dial_ = this.root.querySelector('[part~="dial"]') ?? undefined;
    this.arcFill_ =
      this.root.querySelector<SVGCircleElement>('[part~="arc"]') ?? undefined;
    this.indicator_ =
      this.root.querySelector<SVGGElement>('[part~="indicator"]') ??
      undefined;
    this.readout_ = this.root.querySelector('[part~="readout"]') ?? undefined;
    super.afterRender();
  }

  protected updateVisual(value: ControlValue, display: DisplayValue): void {
    const n = value.normalized;
    this.arcFill_?.setAttribute(
      "stroke-dasharray",
      `${n * ARC_LEN} ${CIRCUMFERENCE}`,
    );
    this.indicator_?.setAttribute(
      "transform",
      `rotate(${-ARC_START_DEG + n * ARC_DEGREES} 20 20)`,
    );
    if (this.readout_ !== undefined) this.readout_.textContent = display.text;
  }
}
