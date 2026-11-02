/**
 * `<rvf-seekbar>` — horizontal seek/position control (Issue #259).
 *
 * Same slider semantics as the fader plus a `buffered` indicator
 * (portion of media already available, e.g. loaded ahead of the
 * playhead — pure display, consumer-driven). Click jumps (begin `at`),
 * drag follows, wheel/keyboard adjust.
 *
 * Parts: `root`, `track`, `buffered`, `fill`, `thumb`, `label`,
 * `readout`. Slots: `label`. Variants: `compact`.
 */

import { tv } from "../tokens.ts";
import { BASE_CSS, STATE_CSS, TEXT_CSS } from "../styles.ts";
import {
  RvfControlElement,
  CONTROL_BASE_ATTRIBUTES,
} from "../control-base.ts";
import type { ControlValue, DisplayValue } from "../headless.ts";

export class RvfSeekbar extends RvfControlElement {
  static observedAttributes = [...CONTROL_BASE_ATTRIBUTES];

  protected gestureMode() {
    return "absolute" as const;
  }

  protected gestureSurface(): Element {
    return this.track_ ?? this;
  }

  private buffered_ = 0;
  /**
   * Buffered-ahead fraction in `[0,1]` (display-only indicator; e.g.
   * media already loaded past the playhead). Clamped, non-finite → 0.
   */
  get buffered(): number {
    return this.buffered_;
  }
  set buffered(v: number) {
    this.buffered_ = Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 0;
    this.syncBuffered();
  }

  protected normalizedFromPoint(e: PointerEvent): number {
    const track = this.track_;
    if (track === undefined) return this.displayValue.normalized;
    const rect = track.getBoundingClientRect();
    return rect.width === 0 ? 0 : (e.clientX - rect.left) / rect.width;
  }

  private track_: HTMLElement | undefined;
  private bufferedEl_: HTMLElement | undefined;
  private fill_: HTMLElement | undefined;
  private thumb_: HTMLElement | undefined;
  private readout_: Element | undefined;

  protected template(): string {
    return `
      <div class="rvf-ctl" part="root">
        <slot name="label"><span class="rvf-label" part="label"></span></slot>
        <div class="rvf-track" part="track">
          <div class="rvf-buffered" part="buffered"></div>
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
        flex-direction: row;
        align-items: center;
        gap: ${tv("space.unit")};
        cursor: pointer;
        min-width: 80px;
      }
      .rvf-track {
        position: relative;
        flex: 1 1 auto;
        height: ${tv("seekbar.height")};
        background: ${tv("seekbar.track")};
        border-radius: ${tv("radius")};
        overflow: hidden;
      }
      .rvf-buffered {
        position: absolute;
        left: 0; top: 0; bottom: 0;
        width: 0;
        background: ${tv("seekbar.buffered")};
      }
      .rvf-fill {
        position: absolute;
        left: 0; top: 0; bottom: 0;
        width: 0;
        background: ${tv("seekbar.fill")};
      }
      :host([data-previewing]) .rvf-fill { background: ${tv("interactive.active")}; }
      .rvf-thumb {
        position: absolute;
        top: 0; bottom: 0;
        width: 2px;
        margin-left: -1px;
        background: ${tv("text")};
      }
      ${TEXT_CSS}
      :host([variant="compact"]) .rvf-track {
        height: calc(${tv("seekbar.height")} * 0.7);
      }
      ${STATE_CSS}
    `;
  }

  protected afterRender(): void {
    this.track_ =
      this.root.querySelector<HTMLElement>('[part~="track"]') ?? undefined;
    this.bufferedEl_ =
      this.root.querySelector<HTMLElement>('[part~="buffered"]') ?? undefined;
    this.fill_ =
      this.root.querySelector<HTMLElement>('[part~="fill"]') ?? undefined;
    this.thumb_ =
      this.root.querySelector<HTMLElement>('[part~="thumb"]') ?? undefined;
    this.readout_ = this.root.querySelector('[part~="readout"]') ?? undefined;
    super.afterRender();
    this.syncBuffered();
  }

  private syncBuffered(): void {
    if (this.bufferedEl_ !== undefined) {
      this.bufferedEl_.style.width = `${this.buffered_ * 100}%`;
    }
  }

  protected updateVisual(value: ControlValue, display: DisplayValue): void {
    const pct = `${value.normalized * 100}%`;
    if (this.fill_ !== undefined) this.fill_.style.width = pct;
    if (this.thumb_ !== undefined) this.thumb_.style.left = pct;
    if (this.readout_ !== undefined) this.readout_.textContent = display.text;
  }
}
