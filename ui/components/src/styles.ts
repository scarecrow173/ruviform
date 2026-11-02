/**
 * Shared stylesheet fragments for the Reference Skin (Issue #259).
 *
 * State styling hooks live on the HOST element's attributes so they are
 * observable and styleable from both inside (shadow CSS) and outside
 * (consumer CSS via `:host([disabled])` etc.):
 *
 * - `disabled` / `readonly` / `unsupported` — distinct unavailable kinds
 *   (never one merged dimming; §12.2/§12.6).
 * - `pending` — consumer-declared "commit dispatched, confirmation
 *    outstanding" (aria-busy mirrors it).
 * - `invalid` — consumer-declared invalid; `data-rejected` — transient
 *    contract-violation flash driven by `rejected` events.
 * - `data-previewing` — a gesture is in flight; the displayed value is a
 *    local preview, not committed state (§13.2 projection rule).
 *
 * Focus uses `:focus-visible` semantics and the `focus.ring`/`focus.width`
 * tokens. Motion honors `prefers-reduced-motion` by collapsing all
 * transitions/animations to zero.
 */

import { tv } from "./tokens.ts";

/** Reset + typography + focus ring shared by every control. */
export const BASE_CSS = `
  :host {
    box-sizing: border-box;
    display: inline-block;
    font-family: ${tv("font.family")};
    font-size: ${tv("font.size")};
    color: ${tv("text")};
    -webkit-tap-highlight-color: transparent;
    user-select: none;
    touch-action: none;
  }
  :host([hidden]) { display: none; }
  *, *::before, *::after { box-sizing: inherit; }
  :host(:focus-visible) {
    outline: ${tv("focus.width")} solid ${tv("focus.ring")};
    outline-offset: 2px;
  }
  @media (prefers-reduced-motion: reduce) {
    *, *::before, *::after {
      transition-duration: 0.01ms !important;
      animation-duration: 0.01ms !important;
      animation-iteration-count: 1 !important;
    }
  }
`;

/** Unavailable-state rules (opacity plus non-color cues per §12.6). */
export const STATE_CSS = `
  :host([disabled]), :host([unsupported]) {
    cursor: not-allowed;
  }
  :host([disabled]) .rvf-ctl, :host([unsupported]) .rvf-ctl {
    opacity: ${tv("state.disabled")};
  }
  :host([unsupported]) .rvf-ctl {
    /* unsupported is not just dim: a hatched edge marks absent capability */
    outline: 1px dashed ${tv("border")};
    outline-offset: 1px;
  }
  :host([readonly]) .rvf-ctl {
    cursor: default;
    /* readonly needs a perceivable cue beyond cursor: a dotted edge
       marks "visible but not editable" without dimming the value. */
    outline: 1px dotted ${tv("border")};
    outline-offset: 1px;
  }
  :host([pending]) .rvf-ctl {
    outline: 1px solid ${tv("status.pending")};
    outline-offset: 1px;
  }
  :host([invalid]) .rvf-ctl, :host([data-rejected]) .rvf-ctl {
    outline: ${tv("focus.width")} solid ${tv("status.error")};
    outline-offset: 1px;
  }
  :host([data-previewing]) .rvf-readout {
    color: ${tv("interactive.active")};
  }
`;

/** Label + readout text styling shared by value controls. */
export const TEXT_CSS = `
  .rvf-label {
    color: ${tv("control.label")};
    font-size: ${tv("font.size")};
    line-height: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rvf-readout {
    color: ${tv("control.readout")};
    font-size: ${tv("font.size")};
    font-variant-numeric: tabular-nums;
    line-height: 1.2;
    transition: color ${tv("motion.fast")} ease;
  }
  :host([hide-readout]) .rvf-readout { display: none; }
`;
