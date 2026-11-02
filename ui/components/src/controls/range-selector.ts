/**
 * `<rvf-range-selector>` — two-edge range control (Issue #259).
 *
 * One descriptor, two headless `InteractionControl`s (`<id>-lo`,
 * `<id>-hi`) — each edge runs the same Begin/Preview/Commit/Cancel
 * contract; `rvf-gesture` `meta.controlId` distinguishes the edge. Edges
 * may not cross: the opposite edge clamps every value input at the
 * dispatch boundary — pointer, keyboard, wheel, and programmatic
 * `valueLow`/`valueHigh` writes alike (a composite-control constraint,
 * not re-implemented mapping). A track click moves the nearer edge
 * (atomic set); thumb drags preserve the grab offset; wheel and
 * per-thumb keyboard adjust each edge independently.
 *
 * Parts: `root`, `track`, `fill`, `thumb-lo`, `thumb-hi`, `label`,
 * `readout`. Slots: `label`. Variants: `compact`.
 */

import {
  InteractionControl,
  type ControlInput,
  type InteractionEvent,
  type ValueDescriptor,
  type ValueInput,
  type ValuePorts,
} from "../headless.ts";
import { RvfElement } from "../element.ts";
import { emitGestureEvent, withSequenceOffset } from "../events.ts";
import {
  attachGestureInput,
  dispatchValueKey,
  type GestureBinding,
  type GestureEndpoint,
} from "../gesture-input.ts";
import { tv } from "../tokens.ts";
import { BASE_CSS, STATE_CSS, TEXT_CSS } from "../styles.ts";

type Edge = "lo" | "hi";

export class RvfRangeSelector extends RvfElement {
  static observedAttributes = [
    "control-id",
    "label",
    "disabled",
    "readonly",
    "invalid",
    "pending",
    "unsupported",
    "variant",
  ];

  private controls_: Partial<Record<Edge, InteractionControl>> = {};
  private descriptor_: ValueDescriptor | undefined;
  private ports_: ValuePorts | undefined;
  private pending_: Partial<Record<Edge, number>> = {};
  private rejectedTimer_: ReturnType<typeof setTimeout> | undefined;
  private bindings_: GestureBinding[] = [];
  private seqOffsets_: Record<Edge, number> = { lo: 0, hi: 0 };
  private emittedSeqMax_: Record<Edge, number> = { lo: 0, hi: 0 };
  private target_: string | undefined;
  private originView_: string | undefined;

  constructor() {
    super();
    this.addCleanup(() => this.onTeardown());
  }

  // ── typed properties ─────────────────────────────────────────

  get descriptor(): ValueDescriptor {
    return this.descriptor_ ?? { min: 0, max: 1 };
  }
  set descriptor(d: ValueDescriptor) {
    this.descriptor_ = d;
    this.rebuild(false);
  }
  get ports(): ValuePorts | undefined {
    return this.ports_;
  }
  set ports(p: ValuePorts | undefined) {
    this.ports_ = p;
    this.rebuild(true);
  }
  get target(): string | undefined {
    return this.target_;
  }
  set target(t: string | undefined) {
    this.target_ = t;
    this.rebuild(true);
  }
  /** Opaque origin-view id carried in event meta (consumer-owned). */
  get originView(): string | undefined {
    return this.originView_;
  }
  set originView(v: string | undefined) {
    this.originView_ = v;
    this.rebuild(true);
  }

  get controlId(): string {
    return (
      this.getAttribute("control-id") ??
      (this.id !== "" ? this.id : this.localName)
    );
  }
  set controlId(v: string) {
    this.setAttribute("control-id", v);
  }

  get valueLow(): number {
    return (
      this.pending_.lo ??
      this.controls_.lo?.getCommittedValue().plain ??
      this.descriptor.min
    );
  }
  set valueLow(v: number) {
    this.setEdge("lo", v);
  }
  get valueHigh(): number {
    return (
      this.pending_.hi ??
      this.controls_.hi?.getCommittedValue().plain ??
      this.descriptor.max
    );
  }
  set valueHigh(v: number) {
    this.setEdge("hi", v);
  }
  /** Convenience accessor for both edges. */
  get range(): { low: number; high: number } {
    return { low: this.valueLow, high: this.valueHigh };
  }
  set range(r: { low: number; high: number }) {
    this.valueLow = r.low;
    this.valueHigh = r.high;
  }

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
  get unsupported(): boolean {
    return this.boolAttr("unsupported");
  }
  set unsupported(v: boolean) {
    this.setBoolAttr("unsupported", v);
  }
  get variant(): string | undefined {
    return this.getAttribute("variant") ?? undefined;
  }
  set variant(v: string | undefined) {
    if (v === undefined) this.removeAttribute("variant");
    else this.setAttribute("variant", v);
  }
  get label(): string {
    return this.getAttribute("label") ?? "";
  }
  set label(v: string) {
    if (v === "") this.removeAttribute("label");
    else this.setAttribute("label", v);
  }

  // ── controller pair ──────────────────────────────────────────

  private ensureControls(): void {
    if (this.controls_.lo === undefined) {
      this.controls_.lo = this.makeControl("lo");
      this.controls_.hi = this.makeControl("hi");
      // Default span covers the whole range, regardless of the
      // descriptor's defaultValue.
      this.controls_.lo.dispatch({
        type: "set",
        value: { extreme: "min" },
        source: "programmatic",
      });
      this.controls_.hi.dispatch({
        type: "set",
        value: { extreme: "max" },
        source: "programmatic",
      });
      this.syncAvailability();
    }
  }

  private makeControl(edge: Edge): InteractionControl {
    return new InteractionControl({
      controlId: `${this.controlId}-${edge}`,
      descriptor: this.descriptor,
      ...(this.ports_ === undefined ? {} : { ports: this.ports_ }),
      ...(this.target_ === undefined ? {} : { target: this.target_ }),
      ...(this.originView_ === undefined
        ? {}
        : { originView: this.originView_ }),
    });
  }

  /**
   * Rebuild the edge pair. `preserveCommitted`: identity/ports swaps
   * keep both edges' committed values; a descriptor swap resets to the
   * full-range default span. Staged edge writes survive the rebuild —
   * the consumer's last write is authoritative regardless of ordering —
   * and each edge's event sequence continues across the swap.
   */
  private rebuild(preserveCommitted: boolean): void {
    if (this.controls_.lo === undefined) {
      this.ensureControls();
      this.syncPresentation();
      return;
    }
    const staged: Partial<Record<Edge, number>> = { ...this.pending_ };
    this.pending_ = {};
    for (const edge of ["lo", "hi"] as const) {
      const c = this.controls_[edge];
      if (c === undefined) continue;
      if (preserveCommitted) staged[edge] ??= c.getCommittedValue().plain;
      if (c.getPhase() === "active") {
        this.processEdgeEvents(
          edge,
          c.dispatch({ type: "detach", reason: "explicit" }),
          false,
        );
      }
      // Rebase so the new controller continues this edge's sequence.
      this.seqOffsets_[edge] = this.emittedSeqMax_[edge];
    }
    this.controls_ = {};
    this.ensureControls();
    this.pending_ = staged;
    this.processEdgeEvents("lo", [], false);
    this.processEdgeEvents("hi", [], false);
  }

  private setEdge(edge: Edge, v: number): void {
    // Stage like the value-control contract: writes survive rebuilds,
    // apply on connect, defer while a gesture is active or the control
    // is unavailable — consumer writes are never dropped silently.
    this.pending_[edge] = v;
    this.processEdgeEvents(edge, [], false);
  }

  private applyExternal(edge: Edge, v: number): void {
    const c = this.controls_[edge];
    if (c === undefined) return;
    this.processEdgeEvents(
      edge,
      c.dispatch({ type: "set", value: this.clampValueFor(edge, { plain: v }), source: "programmatic" }),
      true,
    );
  }

  private processEdgeEvents(
    edge: Edge,
    events: InteractionEvent[],
    suppressAtomic: boolean,
  ): void {
    for (const e of events) {
      const rebased = withSequenceOffset(e, this.seqOffsets_[edge]);
      if (rebased.meta.sequence > this.emittedSeqMax_[edge]) {
        this.emittedSeqMax_[edge] = rebased.meta.sequence;
      }
      if (!(suppressAtomic && e.meta.atomic === true)) {
        emitGestureEvent(this, rebased);
      }
      if (e.type === "rejected") this.flashRejected();
    }
    const pending = this.pending_[edge];
    const c = this.controls_[edge];
    if (
      pending !== undefined &&
      c !== undefined &&
      c.getPhase() === "idle" &&
      this.isConnected &&
      !this.isUnavailable()
    ) {
      delete this.pending_[edge];
      this.applyExternal(edge, pending);
    }
    this.syncPresentation();
  }

  /** Element+descriptor availability as the controller would gate it. */
  private isUnavailable(): boolean {
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
   * The no-crossing invariant, applied at the input boundary so pointer,
   * keyboard, wheel, and programmatic writes all observe it. A value
   * input resolves to a normalized target clamped at the opposite edge;
   * inputs that fail contract resolution pass through untouched so the
   * controller still reports its rejection.
   */
  private clampValueFor(
    edge: Edge,
    v: ValueInput,
    fine = false,
  ): ValueInput {
    const c = this.controls_[edge];
    if (c === undefined) return v;
    let n: number | undefined;
    if ("normalized" in v) n = v.normalized;
    else if ("plain" in v) {
      const r = c.mapper.resolvePlain(v.plain);
      if (r.ok) n = r.normalized;
    } else if ("steps" in v) {
      const r = c.mapper.adjustNormalized(
        c.getValue().normalized,
        v.steps,
        fine,
      );
      if (r.ok) n = r.normalized;
    } else {
      const plain =
        v.extreme === "min"
          ? this.descriptor.min
          : v.extreme === "max"
            ? this.descriptor.max
            : c.mapper.defaultPlain();
      const r = c.mapper.resolvePlain(plain);
      if (r.ok) n = r.normalized;
    }
    if (n === undefined) return v;
    const bound = this.edgeNormalized(edge === "lo" ? "hi" : "lo");
    const clamped = edge === "lo" ? Math.min(n, bound) : Math.max(n, bound);
    return { normalized: clamped };
  }

  private endpointFor(edge: Edge): GestureEndpoint {
    return {
      control: () => this.controls_[edge],
      forward: (events) => this.processEdgeEvents(edge, events, false),
      preprocess: (input) => this.clampInputFor(edge, input),
    };
  }

  /** Translate value-carrying inputs through the edge clamp. */
  private clampInputFor(edge: Edge, input: ControlInput): ControlInput {
    switch (input.type) {
      case "begin":
        if (input.at === undefined) return input;
        return { ...input, at: this.clampValueFor(edge, input.at) };
      case "preview":
        return { ...input, value: this.clampValueFor(edge, input.value) };
      case "commit":
        if (input.value === undefined) return input;
        return { ...input, value: this.clampValueFor(edge, input.value) };
      case "set":
        return { ...input, value: this.clampValueFor(edge, input.value) };
      case "adjust":
        return {
          type: "set",
          value: this.clampValueFor(
            edge,
            { steps: input.steps },
            input.modifiers?.fine === true,
          ),
          ...(input.gestureId === undefined
            ? {}
            : { gestureId: input.gestureId }),
          ...(input.source === undefined ? {} : { source: input.source }),
          ...(input.modifiers === undefined
            ? {}
            : { modifiers: input.modifiers }),
          ...(input.seq === undefined ? {} : { seq: input.seq }),
        };
      case "reset":
        return {
          type: "set",
          value: this.clampValueFor(edge, { extreme: "default" }),
          ...(input.gestureId === undefined
            ? {}
            : { gestureId: input.gestureId }),
          ...(input.source === undefined ? {} : { source: input.source }),
          ...(input.seq === undefined ? {} : { seq: input.seq }),
        };
      default:
        return input;
    }
  }

  private flashRejected(): void {
    this.setAttribute("data-rejected", "");
    if (this.rejectedTimer_ !== undefined) clearTimeout(this.rejectedTimer_);
    this.rejectedTimer_ = setTimeout(() => {
      this.rejectedTimer_ = undefined;
      this.removeAttribute("data-rejected");
    }, 400);
  }

  /**
   * Disconnect teardown — idempotent (runs on every disconnect).
   * Gesture listeners stay bound (their shadow nodes persist across a
   * remount); only timers and in-flight gestures are terminated.
   */
  private onTeardown(): void {
    if (this.rejectedTimer_ !== undefined) {
      clearTimeout(this.rejectedTimer_);
      this.rejectedTimer_ = undefined;
    }
    for (const b of this.bindings_) b.suspend();
    for (const edge of ["lo", "hi"] as const) {
      const c = this.controls_[edge];
      if (c !== undefined && c.getPhase() === "active") {
        this.processEdgeEvents(
          edge,
          c.dispatch({ type: "detach", reason: "unmount" }),
          false,
        );
      }
    }
  }

  // ── lifecycle ────────────────────────────────────────────────

  connectedCallback(): void {
    this.upgradeProperty("descriptor");
    this.upgradeProperty("ports");
    this.upgradeProperty("valueLow");
    this.upgradeProperty("valueHigh");
    super.connectedCallback();
    this.ensureControls();
    this.syncPresentation();
    // Flush staged edge writes deferred until connect.
    this.processEdgeEvents("lo", [], false);
    this.processEdgeEvents("hi", [], false);
  }

  attributeChangedCallback(
    name: string,
    oldValue: string | null,
    newValue: string | null,
  ): void {
    if (oldValue === newValue) return;
    if (name === "variant") this.refresh();
    else if (name === "control-id") this.rebuild(true);
    else if (name === "disabled" || name === "readonly" || name === "unsupported") {
      this.syncAvailability();
      this.syncPresentation();
    } else {
      this.syncPresentation();
    }
  }

  /**
   * Rebuild the shadow tree. In-flight edge gestures are terminated
   * first: the pointer capture and tracking closures die with the old
   * nodes, so leaving them active would strand them permanently.
   */
  protected override refresh(): void {
    for (const edge of ["lo", "hi"] as const) {
      const c = this.controls_[edge];
      if (c !== undefined && c.getPhase() === "active") {
        this.processEdgeEvents(
          edge,
          c.dispatch({ type: "detach", reason: "explicit" }),
          false,
        );
      }
    }
    super.refresh();
  }

  private syncAvailability(): void {
    const flags = {
      disabled: this.disabled || this.descriptor.disabled === true,
      readOnly: this.readOnly || this.descriptor.readOnly === true,
      unsupported: this.boolAttr("unsupported"),
    };
    for (const edge of ["lo", "hi"] as const) {
      const c = this.controls_[edge];
      if (c === undefined) continue;
      const events = c.setAvailability(flags);
      // Always process — becoming available again flushes staged writes.
      this.processEdgeEvents(edge, events, false);
    }
  }

  // ── geometry ─────────────────────────────────────────────────

  private normalizedFromPoint(e: PointerEvent): number {
    const track = this.track_;
    if (track === undefined) return 0;
    const rect = track.getBoundingClientRect();
    return rect.width === 0 ? 0 : (e.clientX - rect.left) / rect.width;
  }

  private edgeNormalized(edge: Edge): number {
    return this.controls_[edge]?.getValue().normalized ?? (edge === "lo" ? 0 : 1);
  }

  /** Clamp a normalized edge target so edges cannot cross. */
  private clampNormalizedFor(edge: Edge, n: number): number {
    const bound = this.edgeNormalized(edge === "lo" ? "hi" : "lo");
    return edge === "lo" ? Math.min(n, bound) : Math.max(n, bound);
  }

  // ── render ───────────────────────────────────────────────────

  private track_: HTMLElement | undefined;
  private fill_: HTMLElement | undefined;
  private thumbLo_: HTMLElement | undefined;
  private thumbHi_: HTMLElement | undefined;
  private readout_: Element | undefined;

  protected template(): string {
    return `
      <div class="rvf-ctl" part="root">
        <slot name="label"><span class="rvf-label" part="label"></span></slot>
        <div class="rvf-track" part="track">
          <div class="rvf-fill" part="fill"></div>
          <div class="rvf-thumb rvf-lo" part="thumb-lo" tabindex="0"></div>
          <div class="rvf-thumb rvf-hi" part="thumb-hi" tabindex="0"></div>
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
        min-width: 100px;
      }
      .rvf-track {
        position: relative;
        flex: 1 1 auto;
        height: ${tv("range.height")};
        background: ${tv("range.track")};
        border-radius: ${tv("radius")};
        cursor: pointer;
      }
      .rvf-fill {
        position: absolute;
        top: 0; bottom: 0;
        background: ${tv("range.fill")};
        border-radius: ${tv("radius")};
      }
      :host([data-previewing]) .rvf-fill { background: ${tv("interactive.active")}; }
      .rvf-thumb {
        position: absolute;
        top: 50%;
        width: calc(${tv("range.height")} * 0.8);
        height: calc(${tv("range.height")} * 1.2);
        background: ${tv("range.thumb")};
        border: 1px solid ${tv("range.thumb.border")};
        border-radius: ${tv("radius")};
        transform: translate(-50%, -50%);
        cursor: ew-resize;
      }
      .rvf-thumb:focus-visible {
        outline: ${tv("focus.width")} solid ${tv("focus.ring")};
        outline-offset: 1px;
      }
      ${TEXT_CSS}
      :host([variant="compact"]) .rvf-track {
        height: calc(${tv("range.height")} * 0.7);
      }
      ${STATE_CSS}
    `;
  }

  protected afterRender(): void {
    this.track_ =
      this.root.querySelector<HTMLElement>('[part~="track"]') ?? undefined;
    this.fill_ =
      this.root.querySelector<HTMLElement>('[part~="fill"]') ?? undefined;
    this.thumbLo_ =
      this.root.querySelector<HTMLElement>('[part~="thumb-lo"]') ?? undefined;
    this.thumbHi_ =
      this.root.querySelector<HTMLElement>('[part~="thumb-hi"]') ?? undefined;
    this.readout_ = this.root.querySelector('[part~="readout"]') ?? undefined;

    for (const b of this.bindings_) b.detach();
    this.bindings_ = [];
    for (const [edge, thumb] of [
      ["lo", this.thumbLo_],
      ["hi", this.thumbHi_],
    ] as const) {
      if (thumb === undefined) continue;
      const binding = attachGestureInput(thumb, this.endpointFor(edge), {
        mode: "absolute",
        normalizedFromPoint: (e) => this.normalizedFromPoint(e),
        // Thumb grabs preserve the grab offset — a press on a thumb is
        // not a track click and must not re-center the edge.
        grabOffset: true,
        clamp: (n) => this.clampNormalizedFor(edge, n),
        focusTarget: () => thumb,
      });
      this.bindings_.push(binding);
      thumb.addEventListener("keydown", (e) => {
        if (dispatchValueKey(e as KeyboardEvent, this.endpointFor(edge))) {
          e.preventDefault();
        }
      });
    }
    // Track click jumps the nearer edge (atomic intent).
    this.track_?.addEventListener("pointerdown", (e) => {
      if ((e.target as Element | null)?.closest?.('[part~="thumb-lo"], [part~="thumb-hi"]')) return;
      const n = this.normalizedFromPoint(e as PointerEvent);
      const edge: Edge =
        Math.abs(n - this.edgeNormalized("lo")) <=
        Math.abs(n - this.edgeNormalized("hi"))
          ? "lo"
          : "hi";
      const c = this.controls_[edge];
      if (c === undefined) return;
      this.processEdgeEvents(
        edge,
        c.dispatch({
          type: "set",
          value: { normalized: this.clampNormalizedFor(edge, n) },
          source: "pointer",
        }),
        false,
      );
    });
  }

  // ── sync ─────────────────────────────────────────────────────

  protected syncPresentation(): void {
    this.setBoolAttr(
      "data-previewing",
      this.controls_.lo?.getPhase() === "active" ||
        this.controls_.hi?.getPhase() === "active",
    );
    for (const [edge, thumb] of [
      ["lo", this.thumbLo_],
      ["hi", this.thumbHi_],
    ] as const) {
      if (thumb === undefined) continue;
      const c = this.controls_[edge];
      const v = c?.getValue() ?? {
        plain: edge === "lo" ? this.descriptor.min : this.descriptor.max,
        normalized: edge === "lo" ? 0 : 1,
      };
      thumb.setAttribute("role", "slider");
      thumb.setAttribute("aria-valuemin", String(this.descriptor.min));
      thumb.setAttribute("aria-valuemax", String(this.descriptor.max));
      thumb.setAttribute("aria-valuenow", String(v.plain));
      const d = c?.mapper.format(v.plain) ?? { text: String(v.plain) };
      thumb.setAttribute("aria-valuetext", d.text);
      const side = edge === "lo" ? "minimum" : "maximum";
      const label = this.label;
      thumb.setAttribute(
        "aria-label",
        label === "" ? side : `${label} ${side}`,
      );
      const set = (name: string, on: boolean) => {
        if (on) thumb.setAttribute(name, "true");
        else thumb.removeAttribute(name);
      };
      set("aria-disabled", this.disabled);
      set("aria-readonly", this.readOnly);
      set("aria-invalid", this.boolAttr("invalid"));
      set("aria-busy", this.boolAttr("pending"));
      thumb.style.left = `${v.normalized * 100}%`;
      if (this.disabled) thumb.removeAttribute("tabindex");
      else if (!thumb.hasAttribute("tabindex"))
        thumb.setAttribute("tabindex", "0");
    }
    const lo = this.edgeNormalized("lo");
    const hi = this.edgeNormalized("hi");
    if (this.fill_ !== undefined) {
      this.fill_.style.left = `${lo * 100}%`;
      this.fill_.style.width = `${(hi - lo) * 100}%`;
    }
    if (this.readout_ !== undefined) {
      const c = this.controls_.lo;
      const fmt = (n: number): string => {
        const plain =
          this.descriptor.min + n * (this.descriptor.max - this.descriptor.min);
        return (c?.mapper.format(plain) ?? { text: String(plain) }).text;
      };
      this.readout_.textContent = `${fmt(lo)} – ${fmt(hi)}`;
    }
    const labelPart = this.root.querySelector('[part~="label"]');
    if (labelPart !== null) labelPart.textContent = this.label;
    const ctl = this.root.querySelector('[part~="root"]');
    if (ctl !== null) {
      this.syncLabel(ctl);
      ctl.setAttribute("role", "group");
      // Pre-render sync may have placed the label on the host; the root
      // part is the labelled widget.
      this.removeAttribute("aria-label");
    }
  }
}
