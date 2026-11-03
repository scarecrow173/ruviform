/**
 * Cross-consumer fixtures (Issue #263 — Consumer A / Consumer B).
 *
 * Two consumer-*shaped* mounts using ONLY the public APIs of
 * ui/components + ui/viz + ui/binding — the same calls a real consumer
 * makes. They are characteristic fixtures, not product code:
 *
 * - {@link mountDawFixture} — a DAW-*shaped* control consumer
 *   (Consumer A): a mixer-strip fader bound to `track.vol`, a
 *   transport bound for action intents, and an automation-lane knob
 *   bound to `send.0.level` — all inside one {@link ViewLink} view
 *   (`mixer.strip.1`) so dock moves exercise generation semantics.
 * - {@link mountAnalysisFixture} — an analysis-*shaped* consumer
 *   (Consumer B): a waveform over a bounded-query source, a spectrum
 *   over the snapshot port, and a {@link QueryBinding} for bounded
 *   metadata/feature pulls (`analysis.view`).
 *
 * Receipt routing matches the showcase mount: `gateway.deliver` is
 * wired once to resolve each receipt's owning binding by
 * `intent.controlId`, so no receipt path can bypass or misroute.
 */

import {
  registerRvfElements,
  type RvfTheme,
  type ThemeInput,
} from "../../components/src/index.ts";
import {
  registerRvfVizElements,
  type MemoryWaveformSource,
} from "../../viz/src/index.ts";
import {
  ViewLink,
  bindControlElement,
  bindTransportElement,
  bindViewIntents,
  type CommandBinding,
  type QueryBinding,
  type Unbind,
  type ViewIntentHandlers,
} from "../../binding/src/index.ts";
import {
  makeDemoMeterSource,
  makeDemoSpectrumSource,
  makeDemoWaveformSource,
  type DemoSnapshotSources,
} from "./demo-data.ts";
import {
  DemoAnalysisBackend,
  DemoControlGateway,
  type DemoReceivedCommand,
} from "./demo-gateway.ts";

export interface ConsumerFixtureOptions {
  /** Binding flush hook — inject `flushSync`-style determinism in tests. */
  readonly flushWith?: (fn: () => void) => void;
  /** Window id the view attaches to. */
  readonly windowId?: string;
  /** Theme applied to the fixture's `<rvf-theme>` scope. */
  readonly theme?: ThemeInput;
  /** Set `autopump` on mounted viz elements (default: leave as-is). */
  readonly autopump?: boolean;
}

// ── Consumer A — DAW-shaped control fixture ─────────────────────────

export interface DawFixture {
  readonly root: HTMLElement;
  readonly scope: RvfTheme;
  readonly link: ViewLink;
  readonly gateway: DemoControlGateway;
  readonly fader: HTMLElement;
  readonly transport: HTMLElement;
  readonly automation: HTMLElement;
  readonly faderBinding: CommandBinding;
  readonly automationBinding: CommandBinding;
  readonly transportBinding: CommandBinding;
  apply(rec: DemoReceivedCommand): void;
  accept(rec: DemoReceivedCommand): void;
  reject(rec: DemoReceivedCommand, detail?: string): void;
  unsupported(rec: DemoReceivedCommand): void;
  unmount(): void;
}

/**
 * Consumer A: mixer-strip fader + transport + automation lane inside a
 * `mixer.strip.1` view on a {@link DemoControlGateway} with
 * `track.vol`, `send.0.level`, `transport` targets.
 */
export function mountDawFixture(
  host: HTMLElement,
  opts: ConsumerFixtureOptions = {},
): DawFixture {
  registerRvfElements();
  const gateway = new DemoControlGateway();
  gateway.addTarget("track.vol", -12);
  gateway.addTarget("send.0.level", 0.5);
  gateway.addTarget("transport", 0);

  const scope = document.createElement("rvf-theme") as RvfTheme;
  scope.setAttribute("data-fixture", "daw");
  if (opts.theme !== undefined) scope.theme = opts.theme;
  const root = document.createElement("div");
  root.setAttribute("data-consumer", "daw");
  scope.append(root);
  host.replaceChildren(scope);

  const link = new ViewLink({ viewId: "mixer.strip.1", port: gateway });
  const unbinds: Unbind[] = [];
  const byControl = new Map<string, CommandBinding>();

  const fader = document.createElement("rvf-fader");
  fader.setAttribute("control-id", "mixer.vol");
  fader.setAttribute("label", "Vol");
  fader.setAttribute("data-control", "fader");
  (fader as { descriptor?: unknown }).descriptor = {
    min: -60,
    max: 6,
    defaultValue: -12,
    unit: "dB",
  };
  root.append(fader);

  const automation = document.createElement("rvf-knob");
  automation.setAttribute("control-id", "mixer.send0");
  automation.setAttribute("label", "Send");
  automation.setAttribute("data-control", "automation");
  (automation as { descriptor?: unknown }).descriptor = {
    min: 0,
    max: 1,
    defaultValue: 0.5,
  };
  root.append(automation);

  const transport = document.createElement("rvf-transport");
  transport.setAttribute("control-id", "transport");
  transport.setAttribute("label", "Transport");
  transport.setAttribute("state", "stopped");
  transport.setAttribute("data-control", "transport");
  (transport as { capabilities?: unknown }).capabilities = {
    resume: false,
  };
  root.append(transport);

  const mk = (target: string, controlId: string): CommandBinding => {
    const b = link.createBinding(target, {
      controlId,
      ...(opts.flushWith !== undefined ? { flushWith: opts.flushWith } : {}),
    });
    byControl.set(controlId, b);
    return b;
  };

  const faderBinding = mk("track.vol", "mixer.vol");
  const automationBinding = mk("send.0.level", "mixer.send0");
  const transportBinding = mk("transport", "transport");
  unbinds.push(bindControlElement(fader, faderBinding));
  unbinds.push(bindControlElement(automation, automationBinding));
  unbinds.push(bindTransportElement(transport, transportBinding));

  link.attach(opts.windowId ?? "daw-window");

  // Wire the receipt channel once: every receipt resolves its owning
  // binding by `intent.controlId` — `cmd-N` ids collide across
  // bindings, so a bare commandId could never route safely.
  gateway.deliver = (rec, receipt) => {
    byControl.get(rec.intent.controlId ?? "")?.ack(receipt);
  };
  return {
    root,
    scope,
    link,
    gateway,
    fader,
    transport,
    automation,
    faderBinding,
    automationBinding,
    transportBinding,
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
      link.detach("unmount");
      scope.remove();
    },
  };
}

// ── Consumer B — analysis-shaped fixture ────────────────────────────

export interface AnalysisFixture {
  readonly root: HTMLElement;
  readonly scope: RvfTheme;
  readonly link: ViewLink;
  readonly backend: DemoAnalysisBackend;
  readonly query: QueryBinding;
  readonly waveform: HTMLElement;
  readonly spectrum: HTMLElement;
  readonly meter: HTMLElement;
  readonly sources: {
    readonly waveform: MemoryWaveformSource;
    readonly snapshots: DemoSnapshotSources;
  };
  /** Recorded view-intent details (`rvf-viewport`/`rvf-select`/`rvf-backend`). */
  readonly viewIntents: {
    readonly viewport: unknown[];
    readonly select: unknown[];
    readonly backend: unknown[];
  };
  unmount(): void;
}

/**
 * Consumer B: waveform over a bounded-query source + spectrum/meter
 * over the snapshot port + a {@link QueryBinding} for bounded
 * metadata/feature pulls, inside the `analysis.view` view on a
 * {@link DemoAnalysisBackend}.
 */
export function mountAnalysisFixture(
  host: HTMLElement,
  opts: ConsumerFixtureOptions = {},
): AnalysisFixture {
  registerRvfElements();
  registerRvfVizElements();
  const backend = new DemoAnalysisBackend();

  const scope = document.createElement("rvf-theme") as RvfTheme;
  scope.setAttribute("data-fixture", "analysis");
  if (opts.theme !== undefined) scope.theme = opts.theme;
  const root = document.createElement("div");
  root.setAttribute("data-consumer", "analysis");
  scope.append(root);
  host.replaceChildren(scope);

  const link = new ViewLink({ viewId: "analysis.view", port: backend });
  const unbinds: Unbind[] = [];

  const wfSource = makeDemoWaveformSource("analysis.wf");
  const snapshots: DemoSnapshotSources = {
    meter: makeDemoMeterSource({ id: "analysis.meter" }),
    spectrum: makeDemoSpectrumSource({ id: "analysis.spectrum" }),
  };

  const wf = document.createElement("rvf-waveform");
  wf.setAttribute("label", "Waveform");
  wf.setAttribute("data-viz", "waveform");
  (wf as { channelLabels?: readonly string[] }).channelLabels = ["L", "R"];
  (wf as { source?: unknown }).source = wfSource;
  root.append(wf);

  const meter = document.createElement("rvf-meter");
  meter.setAttribute("label", "Level");
  meter.setAttribute("data-viz", "meter");
  (meter as { source?: unknown }).source = snapshots.meter;
  root.append(meter);

  const sp = document.createElement("rvf-spectrum");
  sp.setAttribute("label", "Spectrum");
  sp.setAttribute("data-viz", "spectrum");
  (sp as { freq?: unknown }).freq = { scale: "log", minHz: 20, maxHz: 20000 };
  (sp as { db?: unknown }).db = { minDb: -90, maxDb: 0, reference: 1 };
  (sp as { source?: unknown }).source = snapshots.spectrum;
  root.append(sp);

  if (opts.autopump !== undefined) {
    for (const el of [wf, meter, sp]) {
      (el as { autopump?: boolean }).autopump = opts.autopump;
    }
  }

  const viewIntents = {
    viewport: [] as unknown[],
    select: [] as unknown[],
    backend: [] as unknown[],
  };
  const handlers: ViewIntentHandlers = {
    onViewport: (d) => viewIntents.viewport.push(d),
    onSelect: (d) => viewIntents.select.push(d),
    onBackend: (d) => viewIntents.backend.push(d),
  };
  unbinds.push(bindViewIntents(wf, handlers));

  const query = link.createQuery();
  link.attach(opts.windowId ?? "analysis-window");

  return {
    root,
    scope,
    link,
    backend,
    query,
    waveform: wf,
    spectrum: sp,
    meter,
    sources: { waveform: wfSource, snapshots },
    viewIntents,
    unmount() {
      for (const u of unbinds.splice(0)) u();
      link.detach("unmount");
      scope.remove();
    },
  };
}
