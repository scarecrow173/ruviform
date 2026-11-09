/**
 * Issue #261 — `ViewLink` / `LinkRegistry`: the multiwindow lifecycle.
 *
 * A *view* is a logical entity (`viewId`) that survives physical host
 * changes: a Dock move or WebView transfer detaches it from one window
 * and attaches it to another **without changing its identity**
 * (Issue #257 §8, Issue #261 contract).
 *
 * Per-attach contract:
 * - `attach(windowId, generation?)` bumps the view generation — or
 *   adopts the caller-supplied authoritative epoch — and re-subscribes
 *   every registered binding through the port — a fresh projection is
 *   re-read rather than trusting what the old window showed.
 * - Bindings and queries joining an already-attached link adopt the
 *   live view epoch at registration (`registerBinding` /
 *   `registerQuery`): join is an epoch boundary, so a query's
 *   in-flight requests minted under a foreign epoch are cancelled.
 * - `detach(reason)` cancels each binding's in-flight gesture
 *   (contractual cancel — the command still reaches the consumer),
 *   releases every subscription, and cancels every in-flight query.
 * - What detach NEVER does: emit project/engine/job commands. A window
 *   move is a UI event; cancelling domain work because a view moved is
 *   the consumer's decision, made on its own commands — this layer
 *   simply has no verb for it.
 * - All operations are idempotent: re-attaching the same window, or
 *   detaching an already-detached link, is a no-op (StrictMode-style
 *   remount safe).
 */

import type { CancelReason, LogicalTarget } from "../../core/src/index.ts";
import type {
  BindingPort,
  StateProjection,
  SubscriptionContext,
} from "./contract.ts";
import {
  CommandBinding,
  type CommandBindingOptions,
} from "./binding.ts";
import { QueryBinding, type QueryBindingOptions } from "./query.ts";

export type LinkDetachReason = Extract<
  CancelReason,
  "unmount" | "dock-transfer" | "explicit"
> | (string & {});

export interface ViewLinkOptions {
  readonly viewId: string;
  readonly port: BindingPort;
}

type BindingOptionsSansPort = Omit<
  CommandBindingOptions,
  "target" | "port" | "viewId" | "windowId"
>;

type QueryOptionsSansPort = Omit<
  QueryBindingOptions,
  "viewId" | "port"
>;

/**
 * Type-erased view of `CommandBinding` for heterogeneous storage. The
 * erase is safe at this boundary: the only `P`-carrying member is
 * `observe`, and the projection it forwards arrives from the consumer's
 * own `subscribe` listener — the `P` contract is honored at the
 * subscribe call site, so the link may hold bindings of different
 * payload types under one unknown-typed handle.
 */
interface ManagedBinding {
  readonly target: LogicalTarget;
  attach(epoch?: number): void;
  detach(reason: CancelReason): void;
  setContext(ctx: { viewId?: string; windowId?: string }): void;
  observe(projection: StateProjection<unknown>): void;
}

function managed<P>(b: CommandBinding<P>): ManagedBinding {
  return b as unknown as ManagedBinding;
}

export class ViewLink {
  readonly viewId: string;
  private readonly port: BindingPort;
  private readonly bindings = new Set<ManagedBinding>();
  private readonly queries = new Set<QueryBinding>();
  private subscriptions: Array<() => void> = [];
  private generation_ = 0;
  private windowId_: string | undefined;
  private attached_ = false;
  private attachCount_ = 0;
  private subscribeCount_ = 0;

  constructor(options: ViewLinkOptions) {
    this.viewId = options.viewId;
    this.port = options.port;
  }

  get windowId(): string | undefined {
    return this.windowId_;
  }

  get generation(): number {
    return this.generation_;
  }

  get attached(): boolean {
    return this.attached_;
  }

  /** Diagnostics: how many times the link attached, and how many
   *  projection subscriptions it currently holds. */
  get linkStats(): {
    readonly attaches: number;
    readonly activeSubscriptions: number;
    readonly totalSubscriptions: number;
  } {
    return {
      attaches: this.attachCount_,
      activeSubscriptions: this.subscriptions.length,
      totalSubscriptions: this.subscribeCount_,
    };
  }

  /** Create a binding on this link's port and register it for
   *  attach/detach management. */
  createBinding<P = unknown>(
    target: LogicalTarget,
    options?: BindingOptionsSansPort,
  ): CommandBinding<P> {
    const binding = new CommandBinding<P>({
      target,
      port: this.port,
      ...(this.attached_ && this.windowId_ !== undefined
        ? { windowId: this.windowId_ }
        : {}),
      viewId: this.viewId,
      ...(options ?? {}),
    });
    this.registerBinding(binding);
    return binding;
  }

  /** Register an externally created binding. Returns an unregister
   *  that detaches it from link lifecycle management — the binding is
   *  removed from attach/detach handling and, if the link subscribed
   *  it at registration, that subscription is released. */
  registerBinding<P>(binding: CommandBinding<P>): () => void {
    const m = managed(binding);
    this.bindings.add(m);
    binding.setContext({
      viewId: this.viewId,
      ...(this.windowId_ !== undefined
        ? { windowId: this.windowId_ }
        : {}),
    });
    // A binding joining an attached link adopts the live view epoch and
    // is subscribed immediately — the same state a pre-registered
    // binding holds after `attach`, so every managed binding dispatches
    // under one epoch (external registration is not a second rule).
    let off: (() => void) | undefined;
    if (this.attached_) {
      binding.attach(this.generation_);
      off = this.subscribeBinding(binding);
    }
    let live = true;
    return () => {
      if (!live) return;
      live = false;
      this.bindings.delete(m);
      if (off !== undefined) {
        const at = this.subscriptions.indexOf(off);
        if (at >= 0) {
          // Still live: release it here. After `detach` the array is
          // already drained and every `off` already ran once — calling
          // it again would double-count at the port.
          this.subscriptions.splice(at, 1);
          off();
        }
      }
    };
  }

  /** Create a query binding on this link's port and register it for
   *  attach/detach management — on an attached link it adopts the
   *  live view epoch (see {@link ViewLink.registerQuery}). */
  createQuery(options?: QueryOptionsSansPort): QueryBinding {
    const q = new QueryBinding({ viewId: this.viewId, port: this.port, ...(options ?? {}) });
    this.registerQuery(q);
    return q;
  }

  /**
   * Register an externally created query for attach/detach lifecycle
   * management. Returns an unregister that removes the query from
   * link lifecycle handling — membership only: unlike bindings,
   * queries hold no subscription, so there is nothing to release.
   *
   * On an already-attached link the query adopts the live view epoch
   * — the same adopt-on-join rule as {@link ViewLink.registerBinding}.
   * Join is an epoch transition for the query: `attach` cancels every
   * in-flight request it carries, so work minted under a foreign epoch
   * resolves `cancelled` rather than landing under the adopted one.
   * On a detached link registration defers adoption to the next
   * `attach`, which re-epochs every registered query.
   */
  registerQuery(query: QueryBinding): () => void {
    this.queries.add(query);
    if (this.attached_) query.attach(this.generation_);
    let live = true;
    return () => {
      if (!live) return;
      live = false;
      this.queries.delete(query);
    };
  }

  /**
   * Attach the view to a window — the first mount, or the far side of
   * a dock transfer. Idempotent for the same window; a different window
   * while still attached performs an implicit `detach("dock-transfer")`
   * first so in-flight work terminates under the old epoch.
   *
   * `generation`, when supplied, is the consumer's authoritative attach
   * epoch (e.g. the backend's attach-response value): it replaces the
   * local increment so every binding, query, and subscription mints
   * under the epoch the backend expects. Supplying a different value
   * while attached to the *same* window is an epoch transition — the
   * old mounted session is over, so the link performs an implicit
   * `detach("unmount")` first; the same value stays a no-op. The value
   * must be a non-negative safe integer: anything else is rejected
   * with a `RangeError` before any link state changes.
   *
   * The supplied epoch must be the consumer's authoritative value —
   * monotonic per view like the backend attach counter. Reusing a
   * dead epoch value for the same view is contract-legal input but
   * defeats the numeric stale guards: the caller owns epoch reuse.
   */
  attach(windowId: string, generation?: number): void {
    if (
      generation !== undefined &&
      (!Number.isSafeInteger(generation) || generation < 0)
    ) {
      throw new RangeError(
        `ViewLink.attach: generation must be a non-negative safe integer, got ${String(generation)}`,
      );
    }
    if (
      this.attached_ &&
      this.windowId_ === windowId &&
      (generation === undefined || generation === this.generation_)
    ) {
      return;
    }
    if (this.attached_) {
      this.detach(this.windowId_ === windowId ? "unmount" : "dock-transfer");
    }
    this.attached_ = true;
    this.windowId_ = windowId;
    this.generation_ = generation ?? this.generation_ + 1;
    this.attachCount_ += 1;
    for (const binding of this.bindings) {
      binding.setContext({ viewId: this.viewId, windowId });
      binding.attach(this.generation_);
      this.subscribeBinding(binding);
    }
    for (const query of this.queries) query.attach(this.generation_);
  }

  /**
   * Detach the view — unmount, dock transfer, or host close. Cancels
   * in-flight gestures (contractual cancel), releases subscriptions,
   * cancels in-flight queries. Domain entities — Project, Engine,
   * long-running Jobs — are never signalled: this layer owns UI
   * lifecycle only.
   */
  detach(reason: LinkDetachReason = "unmount"): void {
    if (!this.attached_) return;
    this.attached_ = false;
    for (const off of this.subscriptions) off();
    this.subscriptions = [];
    for (const binding of this.bindings) binding.detach(reason);
    for (const query of this.queries) query.detach();
  }

  private subscribeBinding(
    binding: ManagedBinding,
  ): (() => void) | undefined {
    if (this.port.subscribe === undefined) return undefined;
    const ctx: SubscriptionContext = {
      viewId: this.viewId,
      generation: this.generation_,
      ...(this.windowId_ !== undefined ? { windowId: this.windowId_ } : {}),
    };
    const generation = this.generation_;
    const off = this.port.subscribe(
      binding.target,
      (p: StateProjection<unknown>) => {
        // Generation at *delivery* must match — a projection pushed to
        // a dead epoch's listener is dropped by the binding's own
        // attached guard, but skipping it here keeps the count honest.
        if (generation !== this.generation_ || !this.attached_) return;
        binding.observe(p);
      },
      ctx,
    );
    this.subscribeCount_ += 1;
    if (off !== undefined) this.subscriptions.push(off);
    return off;
  }
}

/**
 * Logical-view registry: `viewId` is stable across windows, so the map
 * — not the window — owns the link. `move` = detach + attach under the
 * same id; `close` detaches and optionally drops the entry.
 */
export class LinkRegistry {
  private readonly links = new Map<string, ViewLink>();

  /** Get or create the link for a logical view id. */
  link(viewId: string, port: BindingPort): ViewLink {
    let link = this.links.get(viewId);
    if (link === undefined) {
      link = new ViewLink({ viewId, port });
      this.links.set(viewId, link);
    }
    return link;
  }

  get(viewId: string): ViewLink | undefined {
    return this.links.get(viewId);
  }

  /** Attach a view to a window (creates the link on first use). */
  attachView(viewId: string, windowId: string, port: BindingPort): ViewLink {
    const link = this.link(viewId, port);
    link.attach(windowId);
    return link;
  }

  /** Dock transfer: same logical view, new window. */
  moveView(viewId: string, windowId: string): void {
    this.links.get(viewId)?.attach(windowId);
  }

  /** Detach without forgetting the view (dock move / remount). */
  detachView(viewId: string, reason: LinkDetachReason = "unmount"): void {
    this.links.get(viewId)?.detach(reason);
  }

  /** Window closed for good: detach and drop the logical entry. */
  closeView(viewId: string): void {
    const link = this.links.get(viewId);
    if (link === undefined) return;
    link.detach("explicit");
    this.links.delete(viewId);
  }
}
