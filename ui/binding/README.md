# @ruviform/ui-binding

Generic Command/Query binding layer for Ruviform (Issue #261): the
integration contract that connects `ui/components` intents and `ui/viz`
viewport needs to a **consumer-owned** Command/Query gateway — state
receipts, bounded preview coalescing, revision-guarded projections, and
multiwindow view links — built on `@ruviform/ui-core` (Issue #258).

**Plain TypeScript only.** No UI framework, runtime, DOM, WebView, or
renderer dependency is allowed in `src/` — not for consumers, not
transitively. `happy-dom` appears in `devDependencies` *only* to
provide a DOM for `node --test`; nothing under `src/` imports it, and
the DOM adapter works on structural `EventTargetLike` objects.

**No domain state ownership.** Authorization, target/revision
validation, undo semantics, domain truth, and Project/Engine/Job
lifecycle live behind the consumer's {@link BindingPort}. This package
connects UI intents to that port and reports exactly what the consumer
reported — it never promotes an intent into a claim about side effects.

## Architecture

```
ui/binding layers (top → bottom)

  dom.ts      thin DOM adapter — rvf-gesture / rvf-transport intents in,
              pending / invalid / unsupported / value reflections out.
              Structural DOM types only; idempotent bind/unbind.
  link.ts     ViewLink + LinkRegistry — stable logical viewId across
              windows; dock-move = detach + attach, never a domain
              cancel; generation-scoped re-subscription.
  binding.ts  CommandBinding — intent→command translation, bounded
              preview coalescing, receipt lifecycle, ack timeout,
              revision-guarded projection ingestion.
  query.ts    QueryBinding — bounded viewport queries (maxBytes,
              maxInflight), per-view generation, AbortSignal +
              explicit cancel channel, stale-reply discard.
  coalesce.ts IntentQueue — latest-wins for previews only;
              begin / commit / cancel / action never drop or reorder.
  contract.ts CommandIntent / CommandReceipt / StateProjection /
              BindingPort / ViewportQuery — the wire agreement.
```

## The port

The consumer gateway implements any subset of `BindingPort`:

```ts
interface BindingPort {
  dispatch(intent: CommandIntent, ctx: DispatchContext): void; // required
  subscribe?<P>(target, listener, ctx): Unsubscribe | undefined;
  query?<T>(query: ViewportQuery, signal?: AbortSignal): Promise<QueryReply<T>>;
  cancelQuery?(queryId: QueryId): void;
}
```

The binding feeds back through two consumer-driven inlets:
`binding.ack(receipt)` for command receipts and `binding.observe(projection)`
for authoritative state — plus `port.subscribe` for push projections.

### Command lifecycle

| Kind | Droppable? | Receipt expected? |
| --- | --- | --- |
| `begin` / `cancel` | never | no |
| `preview` | yes — bounded latest-wins per gesture | no |
| `commit` / `action` | never | yes (`ackTimeoutMs`, default 5 s) |

### Display phases

`preview → pending → accepted → applied → observed` are distinct;
`rejected`, `unsupported`, and `timeout` are explicit terminal
failures; `stale` means the binding is detached. Delayed, reordered,
and duplicate receipts are absorbed and counted
(`binding.diagnostics`), never applied twice and never regressed. A
projection that does not advance the observed revision is dropped.

## Multiwindow linking

`ViewLink` keeps `viewId` stable across physical hosts: a Dock move is
`detach("dock-transfer")` + `attach(newWindowId)` under the same
logical view. Detach cancels the in-flight gesture (the cancel command
still reaches the consumer), releases subscriptions, and cancels
in-flight queries — and **never** emits Project/Engine/Job commands.
`LinkRegistry` owns the viewId → link map for a host shell.

## DOM adapter

`bindControlElement(el, binding)` wires `rvf-gesture` events into the
binding and reflects `BindingViewState` back (`pending`, `invalid`,
`unsupported`, controlled `value` writes, revert-on-reject).
`bindTransportElement` turns `rvf-transport` activations into
non-droppable `action` commands. Both are idempotent per
(element, binding) pair — StrictMode-style remounts do not stack
listeners or duplicate commands.

## Usage

```ts
import {
  CommandBinding,
  ViewLink,
  bindControlElement,
} from "@ruviform/ui-binding";

const port = myGateway; // implements BindingPort
const link = new ViewLink({ viewId: "mixer.strip.1", port });
const binding = link.createBinding("track.vol");
link.attach("main-window");
const unbind = bindControlElement(faderEl, binding);
```

Vanilla TypeScript consumers use the same classes without any DOM;
framework adapters wrap this package rather than forking it.
