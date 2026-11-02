/**
 * `<rvf-numeric-input>` — text-entry value control (Issue #259).
 *
 * A real `<input>` inside the shadow tree carries `role="spinbutton"`
 * plus the full value ARIA — keyboard, IME, and screen-reader semantics
 * stay native. Typing model:
 * - first `input` event opens the `edit` gesture (`begin`); each
 *   parseable keystroke emits `preview` (non-parseable text stays
 *   unreported — the field is simply still being edited),
 * - Enter commits parseable text; blur commits too,
 * - invalid text on commit → `rejected` (non-finite) + gesture stays
 *   alive so the user can fix the text; blur additionally cancels with
 *   `lost-focus` and restores the committed display,
 * - Escape cancels and restores the committed display,
 * - IME `compositionstart..end` suspends all dispatches (no half-token
 *   commits),
 * - ArrowUp/Down adjust by the declared step, spin buttons click-adjust.
 *
 * Parts: `root`, `field`, `input`, `spin-up`, `spin-down`, `label`.
 * Slots: `label`. Variants: `compact` (hides the spin buttons).
 */

import { tv } from "../tokens.ts";
import { BASE_CSS, STATE_CSS, TEXT_CSS } from "../styles.ts";
import {
  RvfControlElement,
  CONTROL_BASE_ATTRIBUTES,
} from "../control-base.ts";
import type {
  ControlInput,
  ControlValue,
  DisplayValue,
} from "../headless.ts";

const EDIT_GESTURE_PREFIX = "edit";

export class RvfNumericInput extends RvfControlElement {
  static observedAttributes = [...CONTROL_BASE_ATTRIBUTES];

  protected gestureMode() {
    return "none" as const; // no pointer-drag surface
  }

  protected hostFocusable(): boolean {
    return false; // focus lands on the inner input
  }

  protected a11yTarget(): Element {
    return this.input_ ?? this;
  }

  private input_: HTMLInputElement | undefined;
  private editing_ = false;
  private composing_ = false;
  // Gesture ids are minted per edit session — a committed/cancelled id
  // is `already-terminated` to the contract and cannot be reused.
  private editGestureId_: string | undefined;
  private editSeq_ = 0;

  protected template(): string {
    return `
      <div class="rvf-ctl" part="root">
        <slot name="label"><span class="rvf-label" part="label"></span></slot>
        <span class="rvf-field" part="field">
          <button class="rvf-spin rvf-down" part="spin-down" type="button"
            tabindex="-1" aria-hidden="true">−</button>
          <input class="rvf-input" part="input" type="text"
            inputmode="decimal" autocomplete="off" spellcheck="false" />
          <button class="rvf-spin rvf-up" part="spin-up" type="button"
            tabindex="-1" aria-hidden="true">+</button>
        </span>
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
      .rvf-field {
        display: inline-flex;
        align-items: stretch;
        height: ${tv("numeric.height")};
        border: 1px solid ${tv("numeric.border")};
        border-radius: ${tv("radius")};
        background: ${tv("numeric.bg")};
        overflow: hidden;
      }
      :host(:focus-within) .rvf-field {
        outline: ${tv("focus.width")} solid ${tv("focus.ring")};
        outline-offset: 1px;
      }
      :host(:focus-visible) { outline: none; }
      .rvf-input {
        border: 0;
        background: transparent;
        color: ${tv("text")};
        font: inherit;
        font-variant-numeric: tabular-nums;
        width: 6.5em;
        padding: 0 ${tv("space.unit")};
        outline: none;
        min-width: 3em;
        /* :host sets user-select:none; the editable field restores it. */
        user-select: text;
      }
      .rvf-spin {
        border: 0;
        background: transparent;
        color: ${tv("control.label")};
        font: inherit;
        width: ${tv("numeric.height")};
        cursor: pointer;
        padding: 0;
      }
      .rvf-spin:hover { background: ${tv("surface")}; color: ${tv("text")}; }
      :host([variant="compact"]) .rvf-spin { display: none; }
      :host([invalid]) .rvf-field, :host([data-rejected]) .rvf-field {
        border-color: ${tv("status.error")};
        outline: none;
      }
      ${TEXT_CSS}
      ${STATE_CSS}
    `;
  }

  protected afterRender(): void {
    // The DOM field is rebuilt — the old edit gesture was detached by
    // refresh, so resync element-side flags or the fresh field renders
    // empty (syncField skips while `editing_`) until the next keystroke.
    this.editing_ = false;
    this.composing_ = false;
    this.editGestureId_ = undefined;
    this.input_ =
      this.root.querySelector<HTMLInputElement>('[part~="input"]') ??
      undefined;
    const up = this.root.querySelector('[part~="spin-up"]');
    const down = this.root.querySelector('[part~="spin-down"]');
    if (this.input_ !== undefined) {
      this.input_.addEventListener("input", this.onTextInput_);
      this.input_.addEventListener("keydown", this.onInputKey_);
      this.input_.addEventListener("focusout", this.onBlur_);
      this.input_.addEventListener(
        "compositionstart",
        () => (this.composing_ = true),
      );
      this.input_.addEventListener("compositionend", () => {
        this.composing_ = false;
        this.onTextInput_();
      });
    }
    up?.addEventListener("click", () => this.stepBy(1));
    down?.addEventListener("click", () => this.stepBy(-1));
    super.afterRender();
  }

  // ── editing gesture ──────────────────────────────────────────

  private parse_(): { ok: true; plain: number } | { ok: false } {
    const c = this.control;
    if (c === undefined || this.input_ === undefined) return { ok: false };
    const r = c.mapper.parse(this.input_.value);
    return r.ok ? { ok: true, plain: r.value } : { ok: false };
  }

  private dispatchTyped(input: ControlInput): void {
    const c = this.control;
    if (c === undefined) return;
    this.processEvents(c.dispatch(input), false);
  }

  private onTextInput_ = (): void => {
    if (this.composing_) return;
    // If the `edit` gesture died externally (controller rebuild, detach,
    // refresh) while `editing_` stayed set, resync the element-side
    // flags — otherwise every keystroke dispatches on a dead gesture.
    if (
      this.editing_ &&
      this.control?.getActiveGestureId() !== this.editGestureId_
    ) {
      this.editing_ = false;
      this.composing_ = false;
    }
    if (!this.editing_) {
      this.editing_ = true;
      this.editGestureId_ = `${EDIT_GESTURE_PREFIX}-${++this.editSeq_}`;
      this.dispatchTyped({
        type: "begin",
        gestureId: this.editGestureId_,
        source: "keyboard",
      });
    }
    const p = this.parse_();
    if (p.ok && this.editGestureId_ !== undefined) {
      this.dispatchTyped({
        type: "preview",
        gestureId: this.editGestureId_,
        value: { plain: p.plain },
      });
    }
  };

  /** Commit parseable text; on invalid text report rejected and keep editing. */
  private commitEdit(commitSource: "enter" | "blur"): void {
    if (!this.editing_ || this.editGestureId_ === undefined) return;
    const gid = this.editGestureId_;
    const p = this.parse_();
    if (p.ok) {
      this.editing_ = false;
      this.editGestureId_ = undefined;
      this.dispatchTyped({
        type: "commit",
        gestureId: gid,
        value: { plain: p.plain },
      });
      this.syncField();
      return;
    }
    // Invalid text: report the violation (gesture survives), then on blur
    // end the gesture via lost-focus cancel and restore committed text.
    this.dispatchTyped({
      type: "commit",
      gestureId: gid,
      value: { plain: Number.NaN },
    });
    if (commitSource === "blur") {
      this.editing_ = false;
      this.editGestureId_ = undefined;
      this.dispatchTyped({
        type: "cancel",
        gestureId: gid,
        reason: "lost-focus",
      });
      this.syncField();
    }
  }

  private cancelEdit(): void {
    if (!this.editing_ || this.editGestureId_ === undefined) return;
    const gid = this.editGestureId_;
    this.editing_ = false;
    this.editGestureId_ = undefined;
    this.dispatchTyped({
      type: "cancel",
      gestureId: gid,
      reason: "escape",
    });
    this.syncField();
  }

  private stepBy(steps: number): void {
    const c = this.control;
    if (c === undefined) return;
    // If the field holds parseable uncommitted text, land it first so the
    // step applies on top — matching native spinbutton behavior.
    if (this.editing_ && this.parse_().ok) this.commitEdit("enter");
    this.processEvents(
      c.dispatch({ type: "adjust", steps, source: "keyboard" }),
      false,
    );
    this.syncField();
  }

  private onInputKey_ = (e: KeyboardEvent): void => {
    // IME composition swallows key dispatch too — Enter/Arrow keys
    // delivered mid-composition must not commit or step half text.
    if (this.composing_ || e.isComposing) return;
    switch (e.key) {
      case "Enter":
        e.preventDefault();
        this.commitEdit("enter");
        return;
      case "Escape":
        e.preventDefault();
        this.cancelEdit();
        return;
      case "ArrowUp":
        e.preventDefault();
        this.stepBy(1);
        return;
      case "ArrowDown":
        e.preventDefault();
        this.stepBy(-1);
        return;
      case "PageUp":
        e.preventDefault();
        this.stepBy(10);
        return;
      case "PageDown":
        e.preventDefault();
        this.stepBy(-10);
        return;
      default:
        return;
    }
  };

  private onBlur_ = (): void => {
    // Commit valid text; invalid text was already rejected by commitEdit
    // and is reverted to the committed display.
    this.commitEdit("blur");
  };

  /** Push the display value back into the field when not being edited. */
  private syncField(): void {
    if (this.input_ !== undefined && !this.editing_) {
      this.input_.value = this.displayText().text;
    }
  }

  protected syncValueA11y(target: Element): void {
    const d = this.descriptor;
    const v = this.displayValue;
    target.setAttribute("role", "spinbutton");
    target.setAttribute("aria-valuemin", String(d.min));
    target.setAttribute("aria-valuemax", String(d.max));
    target.setAttribute("aria-valuenow", String(v.plain));
    target.setAttribute("aria-valuetext", this.displayText().text);
    const set = (name: string, on: boolean) => {
      if (on) target.setAttribute(name, "true");
      else target.removeAttribute(name);
    };
    set("aria-disabled", this.disabled);
    set("aria-readonly", this.readOnly);
    set("aria-invalid", this.invalid);
    set("aria-busy", this.pending);
    if (this.input_ !== undefined) {
      this.input_.disabled = this.disabled;
      this.input_.readOnly = this.readOnly;
    }
  }

  protected updateVisual(_v: ControlValue, _d: DisplayValue): void {
    this.syncField();
  }
}
