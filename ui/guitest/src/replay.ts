/**
 * Issue #311 — deterministic replay driver.
 *
 * Injects the *input column* of a scenario (or a `*.v1.jsonl`
 * recording's `dir:"in"` records) into the real `InteractionControl` /
 * `CommandBinding` / `QueryBinding` / `ViewLink` path and collects the
 * observable trace. Judgment is external: the trace is compared
 * against {@link ContractModel}'s expected trace (the independent
 * oracle) plus any `expect` pins — recorded responses are never the
 * PASS criterion (spec §5, AC-2/AC-3).
 *
 * Determinism comes from: injected `ManualTimers` clock + deferred
 * flush (queue drains only on `flush` ops), synchronous scripted
 * backends, and `setImmediate`-bounded awaits between ops.
 */

import type {
  BindingPort,
  CommandReceipt,
  QueryOutcome,
  StateProjection,
  SubscriptionContext,
  Unsubscribe,
  ViewportQuery,
} from "../../binding/src/contract.ts";
import {
  CommandBinding,
  QueryBinding,
  LinkRegistry,
} from "../../binding/src/index.ts";
import { InteractionControl } from "../../core/src/interaction.ts";
import type { ControlInput } from "../../core/src/interaction.ts";
import {
  ScriptedAnalysisBackend,
  ScriptedDawBackend,
} from "./consumers.ts";
import { ContractModel, type RunTrace } from "./oracle.ts";
import { ManualTimers, deferredFlush } from "./timing.ts";
import type { IpcRecorder } from "./recorder.ts";
import { parseRecording, type RecordLine } from "./jsonl.ts";

export interface ScenarioTarget {
  readonly control: string;
  readonly target: string;
  readonly initial: number;
}

export interface ScenarioSetup {
  readonly consumer: "daw" | "analysis";
  readonly viewId: string;
  readonly windowId: string;
  readonly ackTimeoutMs?: number;
  readonly maxInflight?: number;
  readonly targets: readonly ScenarioTarget[];
}

export interface ScenarioDoc {
  readonly id: string;
  readonly seed?: number | null;
  readonly setup: ScenarioSetup;
  readonly in: readonly Record<string, unknown>[];
  readonly expect?: Record<string, unknown>;
}

type Backend = ScriptedDawBackend | ScriptedAnalysisBackend;

interface Handles {
  timers: ManualTimers;
  deferred: ReturnType<typeof deferredFlush>;
  link: ReturnType<LinkRegistry["link"]>;
  backend: Backend;
  bindings: Map<string, CommandBinding<{ plain: number; normalized: number }>>;
  controls: Map<string, InteractionControl>;
  query: QueryBinding | undefined;
  registry: LinkRegistry;
  trace: {
    dispatched: RunTrace["dispatched"];
    phases: RunTrace["phases"];
    subscriptions: RunTrace["subscriptions"];
    unsubscriptions: RunTrace["unsubscriptions"];
    queries: RunTrace["queries"];
  };
  queryOutcomes: Map<string, Promise<QueryOutcome<unknown>>>;
  queryAs: Map<string, string>;
  mintedBy: Map<string, CommandBinding<unknown>>;
  lastBeforeDetach: string | undefined;
  outcomes: Map<string, string>;
}

function asStr(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
}

async function settle(): Promise<void> {
  await new Promise((r) => setImmediate(r));
}

/** Build the real object graph for one scenario. When `recorder` is
 *  enabled the port it wraps also feeds the JSONL artifact. */
export function buildHarness(
  setup: ScenarioSetup,
  recorder?: IpcRecorder,
): Handles {
  const timers = new ManualTimers();
  const flush = deferredFlush();
  const backend: Backend =
    setup.consumer === "daw"
      ? new ScriptedDawBackend()
      : new ScriptedAnalysisBackend();

  const trace: Handles["trace"] = {
    dispatched: [],
    phases: [],
    subscriptions: [],
    unsubscriptions: [],
    queries: [],
  };
  const queryOutcomes = new Map<string, Promise<QueryOutcome<unknown>>>();
  const queryAs = new Map<string, string>();
  const mintedBy = new Map<string, CommandBinding<unknown>>();
  const outcomes = new Map<string, string>();

  // Driver-level port shim: captures the binding→consumer crossings as
  // trace events. This observation is the SUT output (always on); the
  // IpcRecorder layer on top is the optional artifact.
  const shim: BindingPort = {
    dispatch(intent, ctx) {
      trace.dispatched.push({
        binding: intent.controlId,
        commandId: ctx.commandId,
        kind: intent.kind,
        controlId: intent.controlId,
        target: intent.target,
        sequence: intent.sequence,
        ...(intent.kind !== "action" && "gestureId" in intent
          ? { gestureId: intent.gestureId }
          : {}),
        ...(intent.kind === "action" ? { action: intent.action } : {}),
        generation: ctx.generation,
        expectReceipt: ctx.expectReceipt,
        ...(ctx.viewId !== undefined ? { viewId: ctx.viewId } : {}),
        ...(ctx.windowId !== undefined ? { windowId: ctx.windowId } : {}),
      });
      backend.dispatch(intent, ctx);
    },
    subscribe(target, listener, ctx) {
      trace.subscriptions.push({
        binding: controlForTarget(target),
        target,
        viewId: ctx.viewId,
        ...(ctx.windowId !== undefined ? { windowId: ctx.windowId } : {}),
        generation: ctx.generation,
      });
      const off = backend.subscribe?.(target, listener, ctx);
      return off === undefined
        ? undefined
        : () => {
            trace.unsubscriptions.push({
              binding: controlForTarget(target),
              target,
            });
            off();
          };
    },
    ...(backend instanceof ScriptedAnalysisBackend
      ? {
          async query<T>(
            query: ViewportQuery,
            signal?: AbortSignal,
          ): Promise<import("../../binding/src/contract.ts").QueryReply<T>> {
            trace.queries.push({
              queryId: query.queryId,
              viewId: query.viewId,
              generation: query.generation,
              sourceId: query.sourceId,
              sourceRevision: query.sourceRevision,
              start: query.start,
              end: query.end,
              ...(query.maxBytes !== undefined
                ? { maxBytes: query.maxBytes }
                : {}),
            });
            return backend.query<T>(query, signal);
          },
          cancelQuery(queryId: string) {
            backend.cancelQuery(queryId);
          },
        }
      : {}),
  };

  const port = recorder === undefined ? shim : recorder.wrapPort(shim);
  const registry = new LinkRegistry();
  const link = registry.link(setup.viewId, port);
  const bindings = new Map<
    string,
    CommandBinding<{ plain: number; normalized: number }>
  >();
  const controls = new Map<string, InteractionControl>();

  for (const t of setup.targets) {
    if (setup.consumer === "daw") {
      (backend as ScriptedDawBackend).addTarget(t.target, t.initial);
    }
    const control = new InteractionControl({
      controlId: t.control,
      descriptor: { min: 0, max: 1, defaultValue: t.initial },
      target: t.target,
    });
    controls.set(t.control, control);
    const binding = link.createBinding<{ plain: number; normalized: number }>(
      t.target,
      {
        controlId: t.control,
        clock: () => timers.now,
        schedule: timers.schedule,
        flushWith: flush.flushWith,
        ...(setup.ackTimeoutMs !== undefined
          ? { ackTimeoutMs: setup.ackTimeoutMs }
          : {}),
        onState: (s) => {
          trace.phases.push({ binding: t.control, phase: s.phase });
        },
      },
    );
    bindings.set(t.control, binding);
  }

  const query =
    setup.consumer === "analysis"
      ? link.createQuery({
          ...(setup.maxInflight !== undefined
            ? { maxInflight: setup.maxInflight }
            : {}),
          revisionProbe: (sourceId) =>
            sourceId === undefined
              ? undefined
              : (backend as ScriptedAnalysisBackend).currentRevision,
        })
      : undefined;

  function controlForTarget(target: string): string {
    for (const t of setup.targets) {
      if (t.target === target) return t.control;
    }
    return target;
  }

  // Route gateway receipts to the owning binding's ack channel; an
  // unminted id goes to the first binding (the port-edge arrival).
  if (setup.consumer === "daw") {
    (backend as ScriptedDawBackend).deliver = (receipt) => {
      const owner = mintedBy.get(receipt.commandId) ?? bindings.values().next().value;
      owner?.ack(receipt as CommandReceipt);
    };
  }

  link.attach(setup.windowId);

  return {
    timers,
    deferred: flush,
    link,
    backend,
    bindings,
    controls,
    query,
    registry,
    trace,
    queryOutcomes,
    queryAs,
    mintedBy,
    lastBeforeDetach: undefined,
    outcomes,
  };
}

/** Resolve a receipt `ref` to a concrete commandId. */
function resolveRef(h: Handles, ref: string | undefined, commandId?: string): string | undefined {
  const log = h.trace.dispatched;
  if (ref === undefined || ref === "last") return log.at(-1)?.commandId;
  if (ref === "first") return log[0]?.commandId;
  if (ref === "dead-epoch") return h.lastBeforeDetach ?? log.at(-1)?.commandId;
  if (ref === "unknown") return commandId;
  return ref; // literal commandId
}

/** Run one `in` op list through the harness. */
export async function runOps(
  h: Handles,
  setup: ScenarioSetup,
  ops: readonly Record<string, unknown>[],
  model: ContractModel,
  recorder?: IpcRecorder,
): Promise<void> {
  for (const op of ops) {
    const name = asStr(op.op);
    switch (name) {
      case "gesture": {
        const controlId = asStr(op.control) ?? "";
        const control = h.controls.get(controlId);
        const binding = h.bindings.get(controlId);
        if (control === undefined || binding === undefined) {
          throw new Error(`scenario: unknown control ${controlId}`);
        }
        for (const input of (op.inputs as ControlInput[]) ?? []) {
          recorder?.record("in", "gesture", {
            controlId,
            gestureId: "gestureId" in input ? input.gestureId : undefined,
            inputType: input.type,
            source: "source" in input ? input.source : undefined,
          });
          for (const e of control.dispatch(input)) {
            binding.event(e);
            model.binding(controlId)?.onInteractionEvent(e);
          }
        }
        break;
      }
      case "action": {
        const controlId = asStr(op.control) ?? setup.targets[0]?.control;
        const binding = h.bindings.get(controlId ?? "");
        recorder?.record("in", "action", { name: op.name });
        binding?.action(asStr(op.name) ?? "");
        model.binding(controlId ?? "")?.pushAction(asStr(op.name) ?? "");
        break;
      }
      case "flush": {
        // The event-loop deferred flush: drain every scheduled flush
        // callback in registration order — the production semantic,
        // not a manual per-binding drain.
        h.deferred.runAll();
        model.flush();
        break;
      }
      case "advance": {
        const ms = typeof op.ms === "number" ? op.ms : 0;
        h.timers.advance(ms);
        model.advance(ms);
        break;
      }
      case "receipt": {
        const commandId = resolveRef(
          h,
          asStr(op.ref),
          asStr(op.commandId),
        );
        if (commandId === undefined) break;
        const receipt: CommandReceipt = {
          commandId,
          status: op.status as CommandReceipt["status"],
          ...(op.revision !== undefined
            ? { revision: op.revision as number | string }
            : {}),
          ...(op.detail !== undefined ? { detail: op.detail as string } : {}),
        };
        recorder?.record("in", "receipt", {
          commandId,
          status: receipt.status,
          ref: asStr(op.ref) ?? "last",
        });
        const owner =
          h.mintedBy.get(commandId) ?? h.bindings.values().next().value;
        owner?.ack(receipt);
        model.ack(receipt);
        break;
      }
      case "projection": {
        const target = asStr(op.target) ?? "";
        recorder?.record("in", "projection", {
          target,
          revision: op.revision as number | string,
          hasValue: true,
        });
        if (h.backend instanceof ScriptedDawBackend) {
          h.backend.pushProjection(target, {
            revision: op.revision as number | string,
            value: op.value as { plain: number; normalized: number },
          });
        } else {
          h.backend.publish({
            revision: op.revision as number | string,
            value: op.value,
          } as StateProjection<unknown>);
        }
        model.projection(target, op.revision as number | string);
        break;
      }
      case "detach": {
        h.lastBeforeDetach = h.trace.dispatched.at(-1)?.commandId;
        const reason = asStr(op.reason) ?? "unmount";
        recorder?.record("in", "detach", { reason });
        h.registry.get(setup.viewId)?.detach(reason);
        model.detach(reason);
        break;
      }
      case "attach": {
        const windowId = asStr(op.windowId) ?? setup.windowId;
        recorder?.record("in", "attach", { windowId });
        h.registry.get(setup.viewId)?.attach(windowId);
        model.attach(windowId);
        break;
      }
      case "closeView": {
        h.lastBeforeDetach = h.trace.dispatched.at(-1)?.commandId;
        recorder?.record("in", "closeView", {});
        h.registry.closeView(setup.viewId);
        model.closeView();
        break;
      }
      case "query": {
        const qb = h.query;
        if (qb === undefined) {
          throw new Error("scenario: query op but no query binding");
        }
        const as = asStr(op.as) ?? `q-${h.queryAs.size + 1}`;
        const p = qb.request({
          sourceId: asStr(op.sourceId) ?? "",
          sourceRevision: op.sourceRevision as number | string,
          start: typeof op.start === "number" ? op.start : 0,
          end: typeof op.end === "number" ? op.end : 0,
          ...(typeof op.maxBytes === "number"
            ? { maxBytes: op.maxBytes }
            : {}),
        });
        h.queryOutcomes.set(as, p);
        void p.then((o) => {
          h.outcomes.set(as, o.ok ? "ok" : o.error.kind);
          recorder?.record("out", "queryOutcome", {
            as,
            outcome: o.ok ? "ok" : o.error.kind,
          });
        });
        // The minted queryId is the last issued query.
        const q = (h.backend as ScriptedAnalysisBackend).queries.at(-1);
        if (q !== undefined) h.queryAs.set(as, q.queryId);
        model.query(as, {
          sourceId: asStr(op.sourceId) ?? "",
          sourceRevision: op.sourceRevision as number | string,
          start: typeof op.start === "number" ? op.start : 0,
          end: typeof op.end === "number" ? op.end : 0,
          ...(typeof op.maxBytes === "number"
            ? { maxBytes: op.maxBytes }
            : {}),
        });
        break;
      }
      case "resolveQuery": {
        const as = asStr(op.as) ?? "";
        const queryId = h.queryAs.get(as) ?? "";
        const reply = (op.reply ?? {}) as Record<string, unknown>;
        (h.backend as ScriptedAnalysisBackend).resolveQuery(queryId, reply);
        model.resolveQuery(as, reply);
        break;
      }
      case "rejectQuery": {
        const as = asStr(op.as) ?? "";
        const queryId = h.queryAs.get(as) ?? "";
        (h.backend as ScriptedAnalysisBackend).rejectQuery(queryId);
        model.rejectQuery(as);
        break;
      }
      case "setSourceRevision": {
        (h.backend as ScriptedAnalysisBackend).currentRevision =
          op.revision as number | string;
        break;
      }
      default:
        throw new Error(`scenario: unknown in op ${String(name)}`);
    }
    // mintedBy bookkeeping from the dispatch trace
    for (const d of h.trace.dispatched) {
      if (!h.mintedBy.has(d.commandId)) {
        h.mintedBy.set(d.commandId, h.bindings.get(d.binding) as CommandBinding<unknown>);
      }
    }
    await settle();
  }
}

/** Finalize the run into a `RunTrace` (same shape the oracle emits). */
export async function collectTrace(
  h: Handles,
  setup: ScenarioSetup,
): Promise<RunTrace> {
  await settle();
  const bindingDiagnostics: RunTrace["bindingDiagnostics"] = {};
  const finalStates: RunTrace["finalStates"] = {};
  for (const [id, b] of h.bindings) {
    bindingDiagnostics[id] = b.diagnostics;
    const s = b.state;
    finalStates[id] = {
      phase: s.phase,
      timedOut: s.timedOut,
      attached: s.attached,
      generation: s.generation,
    };
  }
  const queryDiagnostics: RunTrace["queryDiagnostics"] = {};
  if (h.query !== undefined) {
    queryDiagnostics[setup.viewId] = h.query.diagnostics;
  }
  // `closeView` drops the link from the registry — its stats live on
  // the link object itself, so read them via the harness's own ref.
  const stats = h.link.linkStats;
  return {
    dispatched: h.trace.dispatched,
    phases: h.trace.phases,
    subscriptions: h.trace.subscriptions,
    unsubscriptions: h.trace.unsubscriptions,
    queries: h.trace.queries,
    queryOutcomes: Object.fromEntries(h.outcomes),
    bindingDiagnostics,
    queryDiagnostics,
    link: {
      attaches: stats.attaches,
      activeSubscriptions: stats.activeSubscriptions,
      totalSubscriptions: stats.totalSubscriptions,
      attached: h.link.attached,
      ...(h.link.windowId !== undefined ? { windowId: h.link.windowId } : {}),
    },
    finalStates,
  };
}

/**
 * Run a full scenario: real objects + oracle in lockstep. Returns both
 * traces — callers compare them (deep equality is the contract check).
 */
export async function runScenario(
  doc: ScenarioDoc,
  recorder?: IpcRecorder,
): Promise<{ actual: RunTrace; expected: RunTrace }> {
  const h = buildHarness(doc.setup, recorder);
  const model = new ContractModel(doc.setup.viewId, doc.setup.consumer, doc.setup.ackTimeoutMs ?? 5000, {
    maxInflight: doc.setup.maxInflight ?? 4,
    defaultMaxBytes: 16 * 1024 * 1024,
    ...(doc.setup.consumer === "analysis"
      ? {
          probe: () =>
            (h.backend as ScriptedAnalysisBackend).currentRevision,
        }
      : {}),
  });
  for (const t of doc.setup.targets) {
    model.addBinding(t.control, t.target);
  }
  if (doc.setup.consumer === "analysis") model.addQuery();
  model.attach(doc.setup.windowId);
  await runOps(h, doc.setup, doc.in, model, recorder);
  return {
    actual: await collectTrace(h, doc.setup),
    expected: model.traceResult(),
  };
}

// ── expect pins ─────────────────────────────────────────────────────
//
// Scenario `expect` semantics (fixtures/README.md §scenario-binding):
// - `dispatched`: ordered subsequence — each expected rec must appear
//   in the actual dispatch log in order; recs carrying `controlId`
//   match only that control's dispatches (interleaved global order is
//   NOT asserted — per-control order is the contract). Fields not
//   listed in a rec are ignored.
// - `phases`: ordered subsequence of phase names across all bindings.
// - `diagnostics`: counters summed across all bindings.
// - `finalState`: subset of the FIRST binding's final state.
// - `linkAttached` / `linkStats`: `trace.link` subset.
// - `queryOutcomes`: `{as → outcome}` subset equality.

function recMatches(actual: Record<string, unknown>, pin: Record<string, unknown>): boolean {
  for (const [k, v] of Object.entries(pin)) {
    if (actual[k] !== v) return false;
  }
  return true;
}

export function checkExpects(
  trace: RunTrace,
  expect: Record<string, unknown>,
  firstControl: string,
): string[] {
  const fails: string[] = [];

  const pinDispatched = expect.dispatched as
    | readonly Record<string, unknown>[]
    | undefined;
  if (pinDispatched !== undefined) {
    const globalCursor = { at: 0 };
    const perControl = new Map<string, number>();
    for (const [i, pin] of pinDispatched.entries()) {
      const ctrl = pin.controlId as string | undefined;
      const pool = ctrl === undefined
        ? trace.dispatched
        : trace.dispatched.filter((d) => d.controlId === ctrl);
      let cursor = ctrl === undefined
        ? globalCursor.at
        : (perControl.get(ctrl) ?? 0);
      let hit = -1;
      for (let j = cursor; j < pool.length; j += 1) {
        if (recMatches(pool[j] as unknown as Record<string, unknown>, pin)) {
          hit = j;
          break;
        }
      }
      if (hit < 0) {
        fails.push(`dispatched[${i}]: no match for ${JSON.stringify(pin)}`);
        break;
      }
      cursor = hit + 1;
      if (ctrl === undefined) globalCursor.at = cursor;
      else perControl.set(ctrl, cursor);
    }
  }

  const pinPhases = expect.phases as readonly string[] | undefined;
  if (pinPhases !== undefined) {
    let at = 0;
    for (const [i, phase] of pinPhases.entries()) {
      const hit = trace.phases.findIndex((p, j) => j >= at && p.phase === phase);
      if (hit < 0) {
        fails.push(`phases[${i}]: no '${phase}' after index ${at}`);
        break;
      }
      at = hit + 1;
    }
  }

  const pinDiag = expect.diagnostics as Record<string, number> | undefined;
  if (pinDiag !== undefined) {
    for (const [k, v] of Object.entries(pinDiag)) {
      const sum = Object.values(trace.bindingDiagnostics).reduce(
        (acc, d) => acc + ((d as unknown as Record<string, number>)[k] ?? 0),
        0,
      );
      if (sum !== v) {
        fails.push(`diagnostics.${k}: expected ${v}, got ${sum}`);
      }
    }
  }

  const pinFinal = expect.finalState as Record<string, unknown> | undefined;
  if (pinFinal !== undefined) {
    const s = trace.finalStates[firstControl];
    if (s === undefined) {
      fails.push(`finalState: no binding '${firstControl}'`);
    } else if (!recMatches(s as unknown as Record<string, unknown>, pinFinal)) {
      fails.push(`finalState: ${JSON.stringify(pinFinal)} vs ${JSON.stringify(s)}`);
    }
  }

  if (expect.linkAttached !== undefined) {
    if (trace.link.attached !== expect.linkAttached) {
      fails.push(`linkAttached: expected ${expect.linkAttached}, got ${trace.link.attached}`);
    }
  }
  const pinLink = expect.linkStats as Record<string, number> | undefined;
  if (pinLink !== undefined) {
    for (const [k, v] of Object.entries(pinLink)) {
      const actual = (trace.link as unknown as Record<string, unknown>)[k];
      if (actual !== v) {
        fails.push(`linkStats.${k}: expected ${v}, got ${String(actual)}`);
      }
    }
  }

  const pinQuery = expect.queryOutcomes as Record<string, string> | undefined;
  if (pinQuery !== undefined) {
    for (const [as, outcome] of Object.entries(pinQuery)) {
      const actual = trace.queryOutcomes[as];
      if (actual !== outcome) {
        fails.push(`queryOutcomes.${as}: expected '${outcome}', got '${actual ?? "<missing>"}'`);
      }
    }
  }

  return fails;
}

/** Replay a `*.v1.jsonl` recording's `in` column as a scenario. */
export function opsFromRecording(records: readonly RecordLine[]): Record<string, unknown>[] {
  const ops: Record<string, unknown>[] = [];
  for (const r of records) {
    if (r.dir !== "in") continue;
    const f = r.fields;
    switch (r.type) {
      case "gesture":
        ops.push({
          op: "gesture",
          control: f.controlId,
          inputs: [
            {
              type: f.inputType,
              gestureId: f.gestureId,
              source: f.source,
            },
          ],
        });
        break;
      case "action":
        ops.push({ op: "action", name: f.name });
        break;
      case "receipt":
        ops.push({
          op: "receipt",
          status: f.status,
          ref: "unknown",
          commandId: f.commandId,
        });
        break;
      case "projection":
        ops.push({
          op: "projection",
          target: f.target,
          revision: f.revision,
        });
        break;
      case "detach":
        ops.push({ op: "detach", reason: f.reason });
        break;
      case "attach":
        ops.push({ op: "attach", windowId: f.windowId });
        break;
      case "closeView":
        ops.push({ op: "closeView" });
        break;
      case "query":
        ops.push({
          op: "query",
          as: f.as,
          sourceId: f.sourceId,
          sourceRevision: f.sourceRevision,
          start: f.start,
          end: f.end,
        });
        break;
      default:
        break;
    }
  }
  return ops;
}

export { parseRecording };
