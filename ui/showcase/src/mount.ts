/**
 * Showcase mounter (Issue #263).
 *
 * Builds the theme matrix: one `<rvf-theme>` scope per
 * {@link ShowcaseTheme}, each holding the same control row
 * (Knob/Fader/Seekbar/Numeric/Toggle/Transport/Range), the same viz row
 * (Waveform/Meter/Spectrum), and honest "deferred" cells for the
 * viz rows that have no element yet (Spectrogram, TimeSeries —
 * Issue #262 native-parity lane).
 *
 * Every value control and the transport are wired through
 * `ui/binding`'s DOM adapter into {@link DemoControlGateway} targets —
 * "display + basic operation" is the fixture, not just markup. Each
 * column is a {@link ViewLink} view (`showcase.<theme>`) with its own
 * gateway targets and viz sources, so columns exercise theme/view
 * isolation rather than cross-talk.
 *
 * Receipt routing: `CommandBinding` mints `cmd-N` ids that collide
 * across bindings, so receipts can only route by `intent.controlId`
 * on the received record — the mount wires `gateway.deliver` once so
 * EVERY receipt resolves its owning binding, and the handle's
 * `apply`/`accept`/`reject`/`unsupported` are thin pass-throughs over
 * the gateway's record-taking methods.
 *
 * Two-edge note: `<rvf-range-selector>` emits per-edge control ids
 * (`<id>-lo` / `<id>-hi`), so the range cell gets two bindings and two
 * gateway targets — a composite consumer control maps to two bound
 * parameters, which is exactly what a real consumer does.
 *
 * The same mount runs in the standalone `examples/showcase.html` page
 * and headless under happy-dom — no DOM API is touched beyond what
 * the component/viz/binding packages already require.
 */

import {
  registerRvfElements,
  type RvfTheme,
  type ThemeInput,
  type ValueDescriptor,
} from "../../components/src/index.ts";
import {
  registerRvfVizElements,
  type MemoryWaveformSource,
} from "../../viz/src/index.ts";
import {
  LinkRegistry,
  bindControlElement,
  bindTransportElement,
  type BindingViewState,
  type CommandBinding,
  type DomEventLike,
  type Unbind,
  type ViewLink,
} from "../../binding/src/index.ts";
import type { InteractionEvent } from "../../core/src/index.ts";
import { SHOWCASE_THEMES, showcaseTheme, type ShowcaseTheme } from "./themes.ts";
import {
  makeDemoMeterSource,
  makeDemoSpectrumSource,
  makeDemoWaveformSource,
  type DemoMeterSource,
  type DemoSpectrumSource,
} from "./demo-data.ts";
import { DemoControlGateway, type DemoReceivedCommand } from "./demo-gateway.ts";

/** A control row entry of the showcase matrix. */
export interface ShowcaseControlSpec {
  /** Matrix key used in fixture maps and `data-control` attributes. */
  readonly key: string;
  readonly tag: string;
  readonly label: string;
  /** Typed descriptor assigned at mount (never JSON-in-attribute). */
  readonly descriptor?: ValueDescriptor;
  /** Static host attributes applied at mount. */
  readonly attrs?: Readonly<Record<string, string>>;
}

/** Controls row — the same set under every theme column. */
export const SHOWCASE_CONTROLS: readonly ShowcaseControlSpec[] = [
  {
    key: "knob",
    tag: "rvf-knob",
    label: "Gain",
    descriptor: { min: 0, max: 100, defaultValue: 30, unit: "%" },
  },
  {
    key: "fader",
    tag: "rvf-fader",
    label: "Vol",
    descriptor: { min: -60, max: 6, defaultValue: -12, unit: "dB" },
  },
  {
    key: "seekbar",
    tag: "rvf-seekbar",
    label: "Pos",
    descriptor: { min: 0, max: 1 },
  },
  {
    key: "numeric",
    tag: "rvf-numeric-input",
    label: "dB",
    descriptor: { min: -60, max: 6, unit: "dB" },
  },
  { key: "toggle", tag: "rvf-toggle", label: "Mute" },
  {
    key: "range",
    tag: "rvf-range-selector",
    label: "Sel",
    descriptor: { min: 0, max: 1 },
  },
  {
    key: "transport",
    tag: "rvf-transport",
    label: "Transport",
    attrs: { state: "stopped" },
  },
];

/** Viz row — the renderers that exist (Issue #260 delivered set). */
export const SHOWCASE_VIZ = ["waveform", "meter", "spectrum"] as const;
export type ShowcaseVizKey = (typeof SHOWCASE_VIZ)[number];

/**
 * Matrix rows with no delivered element yet — rendered as honest
 * "deferred" cells, never as empty canvas or fake substitutes.
 */
export const SHOWCASE_DEFERRED_VIZ = [
  { key: "spectrogram", deferredTo: "Issue #262 native-parity lane" },
  { key: "timeseries", deferredTo: "Issue #262 native-parity lane" },
] as const;

export interface ShowcaseOptions {
  /** Theme ids to mount (default: every {@link SHOWCASE_THEMES} entry). */
  readonly themes?: readonly string[];
  /** Mount the controls row (default `true`). */
  readonly controls?: boolean;
  /** Mount the viz row (default `true`). */
  readonly viz?: boolean;
  /** Render deferred-viz placeholder cells (default `true`). */
  readonly deferredRows?: boolean;
  /** Shared gateway (default: a fresh {@link DemoControlGateway}). */
  readonly gateway?: DemoControlGateway;
  /** Binding flush hook — inject `flushSync`-style determinism in tests. */
  readonly flushWith?: (fn: () => void) => void;
  /** Target-name prefix for gateway parameters. */
  readonly targetPrefix?: string;
  /** Window id the column views attach to (default `"showcase"`). */
  readonly windowId?: string;
  /** Set `autopump=false` on the viz elements (headless tests do this
   *  so a failed assertion can never leave a pump loop running). */
  readonly autopump?: boolean;
}

export interface ShowcaseVizSources {
  readonly waveform: MemoryWaveformSource;
  readonly meter: DemoMeterSource;
  readonly spectrum: DemoSpectrumSource;
}

export interface ShowcaseColumn {
  readonly theme: ShowcaseTheme;
  /** The `<rvf-theme>` scope element for this column. */
  readonly scope: RvfTheme;
  /** The column's logical view (`showcase.<themeId>`). */
  readonly link: ViewLink;
  /** Control key → mounted element. */
  readonly controls: ReadonlyMap<string, HTMLElement>;
  /** Control key → its CommandBinding (value controls + transport).
   *  The two-edge `range` cell registers `range-lo` / `range-hi`. */
  readonly bindings: ReadonlyMap<string, CommandBinding>;
  /** Viz key → mounted element. */
  readonly viz: ReadonlyMap<string, HTMLElement>;
  /** This column's demo viz sources. */
  readonly sources: ShowcaseVizSources;
}

export interface ShowcaseHandle {
  /** The mounted matrix root (`data-showcase` container). */
  readonly root: HTMLElement;
  readonly columns: readonly ShowcaseColumn[];
  readonly gateway: DemoControlGateway;
  /** Logical-view link registry (`showcase.<theme>` views). */
  readonly links: LinkRegistry;
  /** Look up the binding that owns a `control-id` (range edges use the
   *  `<id>-lo` / `<id>-hi` ids the element emits). */
  bindingFor(controlId: string): CommandBinding | undefined;
  /** Receipt helpers — `gateway.deliver` already routes every receipt
   *  to the record's owning binding; these are thin pass-throughs. */
  apply(rec: DemoReceivedCommand): void;
  accept(rec: DemoReceivedCommand): void;
  reject(rec: DemoReceivedCommand, detail?: string): void;
  unsupported(rec: DemoReceivedCommand): void;
  /** Remove the matrix (element teardown runs on disconnect). */
  unmount(): void;
}

function rowEl(kind: string): HTMLElement {
  const row = document.createElement("section");
  row.className = `sc-row sc-row-${kind}`;
  row.setAttribute("data-row", kind);
  return row;
}

/**
 * Bind a two-edge `<rvf-range-selector>` to its per-edge
 * {@link CommandBinding} pair. The element emits `rvf-gesture` events
 * whose `meta.controlId` carries the edge suffix — each binding's
 * `event()` filter accepts only its own edge, and reflections write
 * `valueLow`/`valueHigh` plus the element's flag attributes. This is
 * the composite-control counterpart of `bindControlElement`: one
 * element, two bound parameters.
 */
function bindRangeElement(
  el: HTMLElement,
  lo: CommandBinding,
  hi: CommandBinding,
): Unbind {
  const flags = el as unknown as {
    pending: boolean;
    invalid: boolean;
    unsupported: boolean;
    valueLow: number;
    valueHigh: number;
  };
  const listener = (e: DomEventLike): void => {
    const d = e.detail as InteractionEvent;
    lo.event(d);
    hi.event(d);
  };
  el.addEventListener("rvf-gesture", listener);

  const plain = (p: { value: unknown }): number | undefined => {
    const v = p.value;
    if (typeof v === "number") return Number.isFinite(v) ? v : undefined;
    if (v !== null && typeof v === "object" && "plain" in v) {
      const pv = (v as { plain: unknown }).plain;
      if (typeof pv === "number" && Number.isFinite(pv)) return pv;
    }
    return undefined;
  };
  const seen = { lo: -1, hi: -1 };
  const reflectFlags = (): void => {
    const phases = [lo.state.phase, hi.state.phase];
    flags.pending = phases.some(
      (p) => p === "pending" || p === "accepted" || p === "applied",
    );
    flags.unsupported = phases.includes("unsupported");
    flags.invalid = phases.includes("rejected") || phases.includes("timeout");
  };
  const reflectEdge = (
    edge: "lo" | "hi",
    s: BindingViewState<unknown>,
  ): void => {
    const failed =
      s.phase === "rejected" ||
      s.phase === "timeout" ||
      s.phase === "unsupported";
    const fresh = s.observedSeq !== seen[edge];
    if (!fresh && !failed) return;
    if (fresh) seen[edge] = s.observedSeq;
    if (s.lastObserved !== undefined) {
      const v = plain(s.lastObserved);
      if (v !== undefined) {
        if (edge === "lo") flags.valueLow = v;
        else flags.valueHigh = v;
      }
    }
  };
  const offLo = lo.addStateListener((s) => {
    reflectEdge("lo", s);
    reflectFlags();
  });
  const offHi = hi.addStateListener((s) => {
    reflectEdge("hi", s);
    reflectFlags();
  });
  reflectEdge("lo", lo.state);
  reflectEdge("hi", hi.state);
  reflectFlags();

  let live = true;
  return () => {
    if (!live) return;
    live = false;
    el.removeEventListener("rvf-gesture", listener);
    offLo();
    offHi();
    lo.detach("unmount");
    hi.detach("unmount");
  };
}

/**
 * Mount the showcase matrix under `host` (clears `host` first).
 * Registers the Ruviform elements idempotently — the same explicit
 * mount boundary every consumer uses.
 */
export function mountShowcase(
  host: HTMLElement,
  opts: ShowcaseOptions = {},
): ShowcaseHandle {
  registerRvfElements();
  registerRvfVizElements();

  const gateway = opts.gateway ?? new DemoControlGateway();
  const links = new LinkRegistry();
  const unbinds: Unbind[] = [];
  const columns: ShowcaseColumn[] = [];
  const byControl = new Map<string, CommandBinding>();

  const root = document.createElement("div");
  root.setAttribute("data-showcase", "");

  const themeIds = opts.themes ?? SHOWCASE_THEMES.map((t) => t.id);
  const targetPrefix = opts.targetPrefix ?? "demo";
  if (new Set(themeIds).size !== themeIds.length) {
    throw new Error("mountShowcase: duplicate theme ids in opts.themes");
  }

  for (const themeId of themeIds) {
    const theme = showcaseTheme(themeId);
    const scope = document.createElement("rvf-theme") as RvfTheme;
    scope.setAttribute("data-theme", theme.id);
    // Clone so the element's shallow theme copy can never alias the
    // module-level SHOWCASE_THEMES constants.
    scope.theme = {
      ...theme.theme,
      ...(theme.theme.tokens !== undefined
        ? { tokens: { ...theme.theme.tokens } }
        : {}),
    } satisfies ThemeInput;

    const col = document.createElement("div");
    col.className = "sc-col";
    col.setAttribute("data-col", theme.id);
    const title = document.createElement("h2");
    title.className = "sc-col-title";
    title.textContent = theme.label;
    col.append(title);
    scope.append(col);

    const link = links.link(`showcase.${theme.id}`, gateway);
    const controls = new Map<string, HTMLElement>();
    const bindings = new Map<string, CommandBinding>();
    const viz = new Map<string, HTMLElement>();
    const sources: ShowcaseVizSources = {
      waveform: makeDemoWaveformSource(`demo-wf.${theme.id}`),
      meter: makeDemoMeterSource({ id: `demo-meter.${theme.id}` }),
      spectrum: makeDemoSpectrumSource({ id: `demo-sp.${theme.id}` }),
    };

    if (opts.controls !== false) {
      const row = rowEl("controls");
      for (const spec of SHOWCASE_CONTROLS) {
        const el = document.createElement(spec.tag);
        const controlId = `${theme.id}.${spec.key}`;
        el.setAttribute("control-id", controlId);
        el.setAttribute("label", spec.label);
        el.setAttribute("data-control", spec.key);
        for (const [k, v] of Object.entries(spec.attrs ?? {})) {
          el.setAttribute(k, v);
        }
        if (spec.descriptor !== undefined) {
          (el as { descriptor?: ValueDescriptor }).descriptor = {
            ...spec.descriptor,
          };
        }
        row.append(el);
        controls.set(spec.key, el);

        const target = `${targetPrefix}.${theme.id}.${spec.key}`;
        const flush = {
          ...(opts.flushWith !== undefined
            ? { flushWith: opts.flushWith }
            : {}),
        };
        if (spec.key === "range") {
          // Two-edge control: one binding + one gateway target per
          // edge — the element's `<id>-lo`/`-hi` control ids route
          // events and receipts to the correct edge binding.
          const loB = link.createBinding(`${target}-lo`, {
            controlId: `${controlId}-lo`,
            ...flush,
          });
          const hiB = link.createBinding(`${target}-hi`, {
            controlId: `${controlId}-hi`,
            ...flush,
          });
          gateway.addTarget(`${target}-lo`, spec.descriptor?.min ?? 0);
          gateway.addTarget(`${target}-hi`, spec.descriptor?.max ?? 1);
          bindings.set("range-lo", loB);
          bindings.set("range-hi", hiB);
          byControl.set(`${controlId}-lo`, loB);
          byControl.set(`${controlId}-hi`, hiB);
          unbinds.push(bindRangeElement(el, loB, hiB));
          continue;
        }
        gateway.addTarget(target, spec.descriptor?.defaultValue ?? 0);
        const binding = link.createBinding(target, {
          controlId,
          ...flush,
        });
        bindings.set(spec.key, binding);
        byControl.set(controlId, binding);
        unbinds.push(
          spec.key === "transport"
            ? bindTransportElement(el, binding)
            : bindControlElement(el, binding),
        );
      }
      col.append(row);
    }

    if (opts.viz !== false) {
      const row = rowEl("viz");
      const wf = document.createElement("rvf-waveform");
      wf.setAttribute("label", "Waveform");
      wf.setAttribute("data-viz", "waveform");
      (wf as { channelLabels?: readonly string[] }).channelLabels = [
        "L",
        "R",
      ];
      (wf as { source?: unknown }).source = sources.waveform;
      const meter = document.createElement("rvf-meter");
      meter.setAttribute("label", "Level");
      meter.setAttribute("data-viz", "meter");
      (meter as { source?: unknown }).source = sources.meter;
      const sp = document.createElement("rvf-spectrum");
      sp.setAttribute("label", "Spectrum");
      sp.setAttribute("data-viz", "spectrum");
      (sp as { freq?: unknown }).freq = {
        scale: "log",
        minHz: 20,
        maxHz: 20000,
      };
      (sp as { db?: unknown }).db = { minDb: -90, maxDb: 0, reference: 1 };
      (sp as { source?: unknown }).source = sources.spectrum;
      for (const [key, el] of [
        ["waveform", wf],
        ["meter", meter],
        ["spectrum", sp],
      ] as const) {
        if (opts.autopump !== undefined) {
          (el as { autopump?: boolean }).autopump = opts.autopump;
        }
        row.append(el);
        viz.set(key, el);
      }
      col.append(row);
    }

    if (opts.deferredRows !== false) {
      const row = rowEl("deferred");
      for (const d of SHOWCASE_DEFERRED_VIZ) {
        const cell = document.createElement("div");
        cell.className = "sc-deferred";
        cell.setAttribute("data-deferred", d.key);
        cell.textContent = `${d.key} — deferred (${d.deferredTo})`;
        row.append(cell);
      }
      col.append(row);
    }

    root.append(scope);
    columns.push({ theme, scope, link, controls, bindings, viz, sources });
  }

  host.replaceChildren(root);
  // Attach the views only once the matrix is live — subscriptions then
  // deliver initial projections to already-connected elements, matching
  // the real mount order a consumer sees.
  for (const col of columns) {
    col.link.attach(opts.windowId ?? "showcase");
  }

  const bindingFor = (controlId: string): CommandBinding | undefined =>
    byControl.get(controlId);
  // Wire the receipt channel ONCE — every receipt resolves its owning
  // binding by `intent.controlId`, so no receipt path can bypass the
  // routing or land on a colliding `cmd-N` of another binding.
  gateway.deliver = (rec, receipt) => {
    bindingFor(rec.intent.controlId ?? "")?.ack(receipt);
  };
  return {
    root,
    columns,
    gateway,
    links,
    bindingFor,
    apply(rec) {
      gateway.apply(rec);
    },
    accept(rec) {
      gateway.accept(rec);
    },
    reject(rec, detail = "rejected by gateway") {
      gateway.reject(rec, detail);
    },
    unsupported(rec) {
      gateway.unsupported(rec);
    },
    unmount() {
      for (const u of unbinds.splice(0)) u();
      for (const col of columns) col.link.detach("unmount");
      root.remove();
    },
  };
}
