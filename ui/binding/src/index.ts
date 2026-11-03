/**
 * `@ruviform/ui-binding` — Issue #261 generic Command/Query binding.
 *
 * Layers (top → bottom):
 *
 * ```
 *   dom.ts      thin DOM adapter — rvf-gesture/rvf-transport intents in,
 *               pending/invalid/unsupported/value reflections out.
 *               Structural DOM types only; no DOM lib required.
 *   link.ts     ViewLink + LinkRegistry — logical view ids across
 *               windows; dock-move = detach + attach, never a domain
 *               cancel.
 *   binding.ts  CommandBinding — intent dispatch, bounded preview
 *               coalescing, receipt lifecycle (accepted/applied/
 *               rejected/unsupported/timeout), revision-guarded
 *               projections.
 *   query.ts    QueryBinding — bounded viewport queries, per-view
 *               generation, cancel, stale-reply discard.
 *   coalesce.ts IntentQueue — latest-wins for previews only;
 *               begin/commit/cancel/action never drop.
 *   contract.ts CommandIntent / CommandReceipt / StateProjection /
 *               BindingPort / ViewportQuery — the wire agreement.
 * ```
 *
 * Consumer ownership boundary: authorization, target/revision
 * validation, undo, domain truth, and job lifecycle live in the
 * consumer gateway behind {@link BindingPort}. This package connects
 * UI intents to that port and tracks what happened — it never asserts
 * a side effect the consumer did not report.
 */

export * from "./contract.ts";
export * from "./coalesce.ts";
export * from "./binding.ts";
export * from "./query.ts";
export * from "./link.ts";
export * from "./dom.ts";

// The headless contract types consumers name when wiring elements are
// re-exported so a binding adapter needs only this package's surface.
export type {
  CancelReason,
  ControlId,
  ControlInput,
  ControlValue,
  GestureId,
  InteractionEvent,
  LogicalTarget,
  OriginView,
  SequenceNumber,
} from "../../core/src/index.ts";
