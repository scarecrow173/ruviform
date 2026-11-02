/**
 * Public DOM event contract (Issue #257 §13.2, Issue #259).
 *
 * All component intents travel as namespaced `CustomEvent`s that are
 * `bubbles: true` + `composed: true` so they cross the shadow boundary and
 * reach ancestors (including framework roots that listen at the document).
 * These are *UI intents* — never Command execution receipts. The consumer
 * binding (#261) owns target resolution, authorization, and receipts.
 *
 * `detail` payloads carry the headless contract types verbatim so a
 * consumer can feed events into its own dispatcher without re-mapping.
 */

import type { InteractionEvent } from "./headless.ts";

/** DOM event type carrying one headless {@link InteractionEvent} per dispatch. */
export const RVF_GESTURE_EVENT = "rvf-gesture";

/** `rvf-gesture` — `detail` is the headless `InteractionEvent` verbatim. */
export type RvfGestureEvent = CustomEvent<InteractionEvent>;

/** Transport intent actions. Transport state itself is a consumer model. */
export type TransportAction = "play" | "pause" | "resume" | "stop";

/** `detail` of {@link RVF_TRANSPORT_EVENT}. */
export interface TransportIntent {
  readonly action: TransportAction;
  /** Element `controlId` (defaults to the element id). */
  readonly controlId: string;
  /** Per-element monotonic sequence, starting at 1. */
  readonly sequence: number;
  /** Physical input source when known (pointer/keyboard/programmatic). */
  readonly source?: string;
}

/** DOM event type carrying a transport intent. */
export const RVF_TRANSPORT_EVENT = "rvf-transport";

/** `rvf-transport` — `detail` is a {@link TransportIntent}. */
export type RvfTransportEvent = CustomEvent<TransportIntent>;

const INTENT_INIT: CustomEventInit = { bubbles: true, composed: true };

/**
 * Rebase an event's `meta.sequence` by `offset`. The element — not the
 * controller instance — is the identity consumers see, so a controller
 * rebuild (descriptor/ports/id swap) must not restart the sequence
 * stream under the same `controlId`; adapters dedupe/order by it.
 */
export function withSequenceOffset(
  event: InteractionEvent,
  offset: number,
): InteractionEvent {
  if (offset === 0) return event;
  return {
    ...event,
    meta: { ...event.meta, sequence: event.meta.sequence + offset },
  } as InteractionEvent;
}

/** Dispatch a headless interaction event as a DOM CustomEvent on `el`. */
export function emitGestureEvent(
  el: Element,
  event: InteractionEvent,
): void {
  el.dispatchEvent(
    new CustomEvent<InteractionEvent>(RVF_GESTURE_EVENT, {
      ...INTENT_INIT,
      detail: event,
    }),
  );
}

/** Dispatch a transport intent as a DOM CustomEvent on `el`. */
export function emitTransportEvent(el: Element, intent: TransportIntent): void {
  el.dispatchEvent(
    new CustomEvent<TransportIntent>(RVF_TRANSPORT_EVENT, {
      ...INTENT_INIT,
      detail: intent,
    }),
  );
}
