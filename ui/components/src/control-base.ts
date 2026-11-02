/**
 * Base class for value-bearing Ruviform controls (Issue #259).
 *
 * Boundary contract (Issue #257 §12/§13):
 * - The element owns ONE headless {@link InteractionControl}. DOM events
 *   become typed {@link ControlInput}s via `gesture-input.ts`; emitted
 *   {@link InteractionEvent}s are forwarded as `rvf-gesture` CustomEvents.
 *   Presentation never re-implements value mapping or gesture semantics.
 * - `value` is a *controlled* property: writes reflect the consumer's
 *   authoritative state and never emit value intents (no command loops).
 *   Contract violations (non-finite, out-of-range under `reject`) still
 *   surface as `rejected` events. A write during an active gesture is
 *   deferred to gesture termination, matching controlled-component
 *   semantics.
 * - Committed value and local preview are distinct style states
 *   (`data-previewing`), and `disabled` / `readOnly` / `unsupported` /
 *   `pending` / `invalid` / `rejected` are separate, perceivable states —
 *   never collapsed into one dimming.
 * - The custom-skin path is subclassing: override `template()` /
 *   `stylesheet()` / `updateVisual()` and keep the inherited gesture,
 *   event, and a11y machinery. Register the subclass under a new tag.
 */

import {
  InteractionControl,
  type ControlInput,
  type ControlValue,
  type DisplayValue,
  type InteractionEvent,
  type ValueDescriptor,
  type ValuePorts,
} from "./headless.ts";
import { RvfElement } from "./element.ts";
import { emitGestureEvent, withSequenceOffset } from "./events.ts";
import {
  attachGestureInput,
  dispatchValueKey,
  type GestureBinding,
  type GestureEndpoint,
  type GestureGeometry,
} from "./gesture-input.ts";

/** Attributes every value control shares (spread into observedAttributes). */
export const CONTROL_BASE_ATTRIBUTES = [
  "control-id",
  "value",
  "label",
  "disabled",
  "readonly",
  "invalid",
  "pending",
  "unsupported",
  "variant",
  "hide-readout",
] as const;

/** How pointer gestures resolve for this control (`none` = no drag surface). */
export type ControlGestureMode = "absolute" | "relative" | "none";

/**
 * ARIA attributes the value-A11y path manages on the a11y target. When
 * the target is an inner element, any copy mirrored onto the host by a
 * pre-render sync is stripped so assistive tech sees one widget.
 */
const CONTROL_HOST_ARIA = [
  "role",
  "aria-valuemin",
  "aria-valuemax",
  "aria-valuenow",
  "aria-valuetext",
  "aria-orientation",
  "aria-disabled",
  "aria-readonly",
  "aria-invalid",
  "aria-busy",
  "aria-label",
] as const;

/**
 * Value-control base. Subclasses provide markup/style, geometry, and the
 * per-control variant list; gesture semantics come from the headless
 * contract.
 */
export abstract class RvfControlElement extends RvfElement {
  private control_: InteractionControl | undefined;
  private descriptor_: ValueDescriptor | undefined;
  private ports_: ValuePorts | undefined;
  private target_: string | undefined;
  private originView_: string | undefined;
  private pendingExternal_: number | undefined;
  private rejectedTimer_: ReturnType<typeof setTimeout> | undefined;
  private gestureBinding_: GestureBinding | undefined;
  private managedTabindex_ = false;
  private seqOffset_ = 0;
  private emittedSeqMax_ = 0;

  constructor() {
    super();
    this.addCleanup(() => this.onTeardown());
  }

  // ── per-control contract ─────────────────────────────────────

  /** Pointer geometry mode: `absolute` (track) or `relative` (knob). */
  protected abstract gestureMode(): ControlGestureMode;
  /** `absolute` mode: event → unclamped normalized track position. */
  protected normalizedFromPoint(_e: PointerEvent): number {
    return 0;
  }
  /** `relative` mode: px covering the full range (default 150). */
  protected dragSpanPx(): number {
    return 150;
  }
  /** Element the pointer wiring attaches to (track/body part). */
  protected gestureSurface(): Element {
    return this;
  }
  /** Element receiving slider ARIA + focus (default: host). */
  protected a11yTarget(): Element {
    return this;
  }
  /** Whether the host itself is tabbable (false when an inner part is). */
  protected hostFocusable(): boolean {
    return true;
  }
  /** Descriptor used when the consumer supplies none. */
  protected defaultDescriptor(): ValueDescriptor {
    return { min: 0, max: 1 };
  }
  /** Declared `variant` values; anything else is ignored with a note. */
  protected supportedVariants(): readonly string[] {
    return ["compact"];
  }
  /** Paint committed/preview state into parts + readout. */
  protected abstract updateVisual(
    value: ControlValue,
    display: DisplayValue,
  ): void;

  // ── headless control ownership ───────────────────────────────

  /** The headless controller (created lazily). Exposed for skins/tests. */
  get control(): InteractionControl | undefined {
    return this.control_;
  }

  /** Stable control id: `control-id` attr → `id` attr → tag name. */
  get controlId(): string {
    return (
      this.getAttribute("control-id") ??
      (this.id !== "" ? this.id : this.localName)
    );
  }
  set controlId(v: string) {
    this.setAttribute("control-id", v);
  }

  private ensureControl(): InteractionControl {
    if (this.control_ === undefined) {
      this.control_ = new InteractionControl({
        controlId: this.controlId,
        descriptor: this.descriptor_ ?? this.defaultDescriptor(),
        ...(this.ports_ === undefined ? {} : { ports: this.ports_ }),
        ...(this.target_ === undefined ? {} : { target: this.target_ }),
        ...(this.originView_ === undefined
          ? {}
          : { originView: this.originView_ }),
      });
      this.syncAvailability();
    }
    return this.control_;
  }

  /**
   * Rebuild the controller (descriptor/ports/identity changed). An
   * in-flight gesture is terminated per the detach contract and the
   * resulting events are forwarded — an invisible rebuild must never
   * strand a gesture. `preserveCommitted`: identity/ports swaps keep the
   * committed value (dock-transfer semantics); a descriptor swap resets
   * to the new descriptor's default since the old value may be
   * meaningless in the new range. A staged `value` write survives the
   * rebuild and lands on the new controller — the consumer's last write
   * is authoritative regardless of property ordering. Event sequence
   * numbers continue across the rebuild: the element is the identity
   * consumers see, so a controller swap must not restart the stream.
   */
  private rebuildControl(preserveCommitted: boolean): void {
    const old = this.control_;
    if (old === undefined) {
      this.ensureControl();
      this.syncPresentation();
      return;
    }
    const pending = this.pendingExternal_;
    this.pendingExternal_ = undefined;
    const committed = old.getCommittedValue().plain;
    // Detach only when a gesture is in flight — an idle detach emits a
    // noise `noop` per rebuild.
    if (old.getPhase() === "active") {
      this.forward(old.dispatch({ type: "detach", reason: "explicit" }));
    }
    // Rebase so the new controller's sequence continues the stream.
    this.seqOffset_ = this.emittedSeqMax_;
    this.control_ = undefined;
    this.ensureControl();
    // Both the preserved commit and the consumer's newer staged write
    // flow through the deferred-write path — they apply when the new
    // controller can accept them, and the later write wins.
    this.pendingExternal_ = pending ?? (preserveCommitted ? committed : undefined);
    this.processEvents([], false);
  }

  // ── typed properties (never JSON-in-attribute per §13.2) ─────

  /** Serializable value descriptor — the control's single source of range. */
  get descriptor(): ValueDescriptor {
    return this.descriptor_ ?? this.defaultDescriptor();
  }
  set descriptor(d: ValueDescriptor) {
    this.descriptor_ = d;
    if (this.control_ !== undefined) this.rebuildControl(false);
  }

  /** Local function ports (formatter/parser/custom mapping). Never serialized. */
  get ports(): ValuePorts | undefined {
    return this.ports_;
  }
  set ports(p: ValuePorts | undefined) {
    this.ports_ = p;
    if (this.control_ !== undefined) this.rebuildControl(true);
  }

  /** Opaque logical target carried in event meta (consumer-owned). */
  get target(): string | undefined {
    return this.target_;
  }
  set target(t: string | undefined) {
    this.target_ = t;
    if (this.control_ !== undefined) this.rebuildControl(true);
  }
  /** Opaque origin-view id carried in event meta (consumer-owned). */
  get originView(): string | undefined {
    return this.originView_;
  }
  set originView(v: string | undefined) {
    this.originView_ = v;
    if (this.control_ !== undefined) this.rebuildControl(true);
  }

  /**
   * Controlled committed value (plain domain). Writes never emit value
   * intents; a write during an active gesture is applied when the gesture
   * terminates.
   */
  get value(): number {
    return (
      this.pendingExternal_ ??
      this.control_?.getCommittedValue().plain ??
      this.mapperDefault()
    );
  }
  set value(v: number) {
    // Controlled writes stage through `pendingExternal_` so they survive
    // controller rebuilds, apply on connect, and defer while a gesture
    // is active or the control is unavailable — a consumer write is
    // authoritative state and is never dropped silently.
    this.pendingExternal_ = v;
    this.processEvents([], false);
  }

  /** Current displayed value — preview while a gesture is active. */
  get displayValue(): ControlValue {
    const c = this.control_;
    if (c !== undefined) return c.getValue();
    // Pre-control: estimate the normalized position from the descriptor
    // so initial paint lands at the real value, not range minimum.
    const plain = this.value;
    const d = this.descriptor;
    const normalized =
      d.max > d.min ? Math.min(1, Math.max(0, (plain - d.min) / (d.max - d.min))) : 0;
    return { plain, normalized };
  }

  // ── attribute surface ────────────────────────────────────────

  get disabled(): boolean {
    return this.boolAttr("disabled");
  }
  set disabled(v: boolean) {
    this.setBoolAttr("disabled", v);
  }
  get readOnly(): boolean {
    return this.boolAttr("readonly");
  }
  set readOnly(v: boolean) {
    this.setBoolAttr("readonly", v);
  }
  get invalid(): boolean {
    return this.boolAttr("invalid");
  }
  set invalid(v: boolean) {
    this.setBoolAttr("invalid", v);
  }
  get pending(): boolean {
    return this.boolAttr("pending");
  }
  set pending(v: boolean) {
    this.setBoolAttr("pending", v);
  }
  /** Marks the control's capability as unsupported (consumer-declared). */
  get unsupported(): boolean {
    return this.boolAttr("unsupported");
  }
  set unsupported(v: boolean) {
    this.setBoolAttr("unsupported", v);
  }
  get label(): string {
    return this.getAttribute("label") ?? "";
  }
  set label(v: string) {
    if (v === "") this.removeAttribute("label");
    else this.setAttribute("label", v);
  }
  get hideReadout(): boolean {
    return this.boolAttr("hide-readout");
  }
  set hideReadout(v: boolean) {
    this.setBoolAttr("hide-readout", v);
  }
  get variant(): string | undefined {
    return this.getAttribute("variant") ?? undefined;
  }
  set variant(v: string | undefined) {
    if (v === undefined) this.removeAttribute("variant");
    else this.setAttribute("variant", v);
  }

  // ── lifecycle ────────────────────────────────────────────────

  connectedCallback(): void {
    this.upgradeProperty("descriptor");
    this.upgradeProperty("ports");
    this.upgradeProperty("value");
    this.upgradeProperty("tokens");
    this.upgradeProperty("target");
    this.upgradeProperty("originView");
    super.connectedCallback();
    this.ensureControl();
    // `addEventListener` dedupes an identical listener, so re-adding on
    // reconnect is safe and keeps the keyboard contract after remount.
    if (this.hostFocusable()) {
      this.addEventListener("keydown", this.onKeyDown);
    }
    this.syncPresentation();
    // Flush a `value` staged before connect (attribute or early property
    // write) — it was deferred, not lost.
    this.processEvents([], false);
  }

  /**
   * Disconnect teardown — registered once, must be idempotent: the
   * cleanup list survives for every disconnect of a remountable element.
   * Gesture listeners stay bound (their shadow nodes persist across a
   * disconnect); only timers and in-flight gestures are terminated.
   */
  private onTeardown(): void {
    if (this.rejectedTimer_ !== undefined) {
      clearTimeout(this.rejectedTimer_);
      this.rejectedTimer_ = undefined;
    }
    this.gestureBinding_?.suspend();
    // Terminate an in-flight gesture per the detach contract; the events
    // still dispatch on `this` so direct listeners observe the ending.
    if (this.control_ !== undefined && this.control_.getPhase() === "active") {
      this.forward(this.control_.dispatch({ type: "detach", reason: "unmount" }));
    }
  }

  attributeChangedCallback(
    name: string,
    oldValue: string | null,
    newValue: string | null,
  ): void {
    if (oldValue === newValue) return;
    if (this.handleBaseAttribute(name, newValue)) return;
    this.handleControlAttribute(name, newValue);
  }

  /** Base attribute handling; returns `true` when consumed. */
  protected handleBaseAttribute(
    name: string,
    value: string | null,
  ): boolean {
    switch (name) {
      case "value":
        // Stage like a property write — attribute writes during parse
        // must defer until connect so a descriptor assigned before
        // mount still applies to them.
        if (value !== null) {
          this.pendingExternal_ = Number(value);
          this.processEvents([], false);
        }
        return true;
      case "disabled":
      case "readonly":
      case "unsupported":
        this.syncAvailability();
        this.syncPresentation();
        return true;
      case "invalid":
      case "pending":
      case "label":
        this.syncPresentation();
        return true;
      case "variant":
        if (
          value !== null &&
          !this.supportedVariants().includes(value)
        ) {
          this.onDiagnostic?.(`unsupported variant "${value}" on <${this.localName}>`);
        }
        this.refresh();
        return true;
      case "hide-readout":
        this.refresh();
        return true;
      case "control-id":
        if (this.control_ !== undefined) this.rebuildControl(true);
        return true;
      default:
        return false;
    }
  }

  /** Per-control attribute hook (e.g. `orientation`). Default: re-sync. */
  protected handleControlAttribute(
    _name: string,
    _value: string | null,
  ): void {
    this.syncPresentation();
  }

  /** Optional diagnostic sink (unsupported variant etc.). Assignable. */
  onDiagnostic: ((detail: string) => void) | undefined;

  // ── event processing ─────────────────────────────────────────

  /**
   * Process controller events: update visual state, forward as
   * `rvf-gesture` CustomEvents, then flush any deferred external write.
   * `suppressAtomic` swallows the synthesized begin/commit echo of
   * consumer-side `value` writes (controlled-property no-loop rule) while
   * still forwarding violations.
   */
  protected processEvents(
    events: InteractionEvent[],
    suppressAtomic: boolean,
  ): void {
    for (const e of events) {
      // Rebase onto element-level ordering: the element is the identity
      // consumers see, so a controller rebuild must not restart the
      // `meta.sequence` stream under the same controlId.
      const rebased = withSequenceOffset(e, this.seqOffset_);
      if (rebased.meta.sequence > this.emittedSeqMax_) {
        this.emittedSeqMax_ = rebased.meta.sequence;
      }
      if (!(suppressAtomic && e.meta.atomic === true)) {
        emitGestureEvent(this, rebased);
      }
      if (e.type === "rejected") this.flashRejected();
    }
    // A staged controlled write lands when the control is connected,
    // idle, and available — it is authoritative state, never dropped.
    if (
      this.pendingExternal_ !== undefined &&
      this.control_ !== undefined &&
      this.control_.getPhase() === "idle" &&
      this.isConnected &&
      !this.isUnavailable()
    ) {
      const v = this.pendingExternal_;
      this.pendingExternal_ = undefined;
      this.applyExternalValue(v);
    }
    this.syncPresentation();
  }

  /** Element+descriptor availability as the controller would gate it. */
  protected isUnavailable(): boolean {
    const d = this.descriptor;
    return (
      this.disabled ||
      this.readOnly ||
      this.unsupported ||
      d.disabled === true ||
      d.readOnly === true
    );
  }

  /**
   * Dispatch a typed input through the element's event pipeline.
   * Custom skins call this instead of `control.dispatch` — direct
   * controller dispatches bypass `rvf-gesture` forwarding and the
   * staged-write flush.
   */
  dispatchInput(input: ControlInput): void {
    const c = this.control_;
    if (c === undefined) return;
    this.processEvents(c.dispatch(input), false);
  }

  /** Endpoint handed to gesture-input wiring. */
  protected readonly endpoint: GestureEndpoint = {
    control: () => this.control_,
    forward: (events) => this.processEvents(events, false),
  };

  private forward(events: InteractionEvent[]): void {
    this.processEvents(events, false);
  }

  private applyExternalValue(v: number): void {
    const events = this.ensureControl().dispatch({
      type: "set",
      value: { plain: v },
      source: "programmatic",
    });
    this.processEvents(events, true);
  }

  private flashRejected(): void {
    this.setAttribute("data-rejected", "");
    if (this.rejectedTimer_ !== undefined) clearTimeout(this.rejectedTimer_);
    this.rejectedTimer_ = setTimeout(() => {
      this.rejectedTimer_ = undefined;
      this.removeAttribute("data-rejected");
    }, 400);
  }

  /** Push element+descriptor capability flags into the controller. */
  protected syncAvailability(): void {
    const c = this.control_;
    if (c === undefined) return;
    const d = this.descriptor;
    const events = c.setAvailability({
      disabled: this.disabled || d.disabled === true,
      readOnly: this.readOnly || d.readOnly === true,
      unsupported: this.unsupported,
    });
    // Always process — an availability transition may make the control
    // usable again, which is what flushes a staged controlled write.
    this.forward(events);
  }

  /** Bind pointer/wheel/dblclick on the gesture surface post-render. */
  protected bindGestures(): void {
    this.gestureBinding_?.detach();
    const mode = this.gestureMode();
    if (mode === "none") {
      this.gestureBinding_ = undefined;
      return;
    }
    const geometry: GestureGeometry = {
      mode,
      normalizedFromPoint: (e) => this.normalizedFromPoint(e),
      dragSpanPx: this.dragSpanPx(),
      focusTarget: () => this.a11yTarget() as HTMLElement,
    };
    this.gestureBinding_ = attachGestureInput(
      this.gestureSurface(),
      this.endpoint,
      geometry,
    );
  }

  protected afterRender(): void {
    this.bindGestures();
  }

  /**
   * Rebuild the shadow tree. An in-flight gesture is terminated first:
   * the pointer capture and tracking closure die with the old nodes, so
   * leaving it active would strand it permanently.
   */
  protected override refresh(): void {
    const c = this.control_;
    if (c !== undefined && c.getPhase() === "active") {
      this.forward(c.dispatch({ type: "detach", reason: "explicit" }));
    }
    super.refresh();
  }

  // ── shared sync ──────────────────────────────────────────────

  protected syncPresentation(): void {
    const c = this.control_;
    this.setBoolAttr("data-previewing", c?.getPhase() === "active");
    const target = this.a11yTarget();
    this.syncLabel(target);
    this.syncLabelPart();
    this.syncValueA11y(target);
    if (target !== this) {
      // A pre-render sync may have mirrored ARIA onto the host while the
      // inner target did not exist yet — strip the stale copy so AT sees
      // exactly one widget.
      for (const name of CONTROL_HOST_ARIA) this.removeAttribute(name);
    }
    if (this.hostFocusable()) {
      if (this.disabled) {
        if (this.managedTabindex_ && this.getAttribute("tabindex") === "0") {
          this.removeAttribute("tabindex");
        }
      } else if (!this.hasAttribute("tabindex")) {
        this.setAttribute("tabindex", "0");
        this.managedTabindex_ = true;
      }
    }
    this.updateVisual(this.displayValue, this.displayText());
  }

  /** Reflect the `label` attribute into the label part's text. */
  private syncLabelPart(): void {
    const part = this.root.querySelector('[part~="label"]');
    if (part !== null) part.textContent = this.label;
  }

  /** Slider-role ARIA on the a11y target (override for other roles). */
  protected syncValueA11y(target: Element): void {
    const d = this.descriptor;
    const v = this.displayValue;
    target.setAttribute("role", "slider");
    target.setAttribute("aria-valuemin", String(d.min));
    target.setAttribute("aria-valuemax", String(d.max));
    target.setAttribute("aria-valuenow", String(v.plain));
    target.setAttribute("aria-valuetext", this.displayText().text);
    const orientation = this.getAttribute("orientation");
    if (orientation !== null) {
      target.setAttribute("aria-orientation", orientation);
    } else {
      target.removeAttribute("aria-orientation");
    }
    this.setOrRemove(target, "aria-disabled", this.disabled);
    this.setOrRemove(target, "aria-readonly", this.readOnly);
    this.setOrRemove(target, "aria-invalid", this.invalid);
    this.setOrRemove(target, "aria-busy", this.pending);
  }

  private setOrRemove(target: Element, name: string, on: boolean): void {
    if (on) target.setAttribute(name, "true");
    else target.removeAttribute(name);
  }

  /** Formatted display text for the current (display) value. */
  protected displayText(): DisplayValue {
    const c = this.ensureControl();
    return c.mapper.format(this.displayValue.plain);
  }

  private mapperDefault(): number {
    try {
      return this.ensureControl().mapper.defaultPlain();
    } catch {
      // An invalid consumer-supplied descriptor must not break rendering.
      return this.descriptor_?.min ?? 0;
    }
  }

  /** Keydown entry point for host-focusable controls. */
  protected onKeyDown = (e: KeyboardEvent): void => {
    if (dispatchValueKey(e, this.endpoint)) e.preventDefault();
  };
}
