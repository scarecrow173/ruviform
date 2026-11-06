/**
 * Issue #261 — `ViewLink` / `LinkRegistry`: the multiwindow lifecycle.
 *
 * A *view* is a logical entity (`viewId`) that survives physical host
 * changes: a Dock move or WebView transfer detaches it from one window
 * and attaches it to another **without changing its identity**
 * (Issue #257 §8, Issue #261 contract).
 *
 * Per-attach contract:
 * - `attach(windowId)` bumps the view generation and re-subscribes
 *   every registered binding through the port — a fresh projection is
 *   re-read rather than trusting what the old window showed.
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
    if (this.attached_) {
      binding.attach(this.generation_);
      this.subscribeBinding(binding);
    }
    return binding;
  }

  /** Register an externally created binding. Returns an unregister
   *  that detaches it from link lifecycle management. */
  registerBinding<P>(binding: CommandBinding<P>): () => void {
    const m = managed(binding);
    this.bindings.add(m);
    binding.setContext({
      viewId: this.viewId,
      ...(this.windowId_ !== undefined
        ? { windowId: this.windowId_ }
        : {}),
    });
    let live = true;
    return () => {
      if (!live) return;
      live = false;
      this.bindings.delete(m);
    };
  }

  /** Create a query binding on this link's port and register it. */
  createQuery(options?: QueryOptionsSansPort): QueryBinding {
    const q = new QueryBinding({ viewId: this.viewId, port: this.port, ...(options ?? {}) });
    this.registerQuery(q);
    return q;
  }

  registerQuery(query: QueryBinding): () => void {
    this.queries.add(query);
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
   */
  attach(windowId: string): void {
    if (this.attached_ && this.windowId_ === windowId) return;
    if (this.attached_) this.detach("dock-transfer");
    this.attached_ = true;
    this.windowId_ = windowId;
    this.generation_ += 1;
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

  private subscribeBinding(binding: ManagedBinding): void {
    if (this.port.subscribe === undefined) return;
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
