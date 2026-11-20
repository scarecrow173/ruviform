# ruviform-plugin-dock-test

Test-only plugin-dock scenario harness (Issue #631 first slice,
RF-VTEST-04). **Not a shipping product** — no production crate may
depend on this crate; it is the consumer-layer composition root that
can see both the plugin side (`ruviform-plugin-*`) and the UI side
(`ruviform-ui-*`), which may not depend on each other.

## What it does

Runs the `rf631.plugin-dock` scenario cases through the
driver-neutral `ruviform-ui-test` contract (`ScenarioSpec` →
`run_scenario` → `ScenarioReport`) against the **real** `test_gain`
VST3 cdylib built by `ruviform-plugin-test-vst3`:

| case | driver | verdict today |
| --- | --- | --- |
| `view-mount` | `Fake` | `Pass` — semantic `NativeSlot` spec mounts (contract conformance only) |
| `command-receipt` | `Fake` | `Pass` — `sv_fader_dock_fixture` gesture mints receipt-expecting `Commit` intents; applied/rejected `Event::Result`s settle `Applied`→`Observed`/`Rejected` and populate `receipts` (contract conformance only — never real-window/pixel/OS evidence) |
| `artifact` | `PluginHostProbe` | `Pass` — resolve + SHA-256 + `Vst3Library::load` + descriptor |
| `host-lifecycle` | `PluginHostProbe` | `Pass` — Scan → Instantiate → Initialize → Activate(48 kHz, 1..=512) → Destroy |
| `editor-attach` | `PluginHostProbe` | `Unsupported` — `createView` null → `HostError::Unsupported` (fixture has no editor) |
| `session-lifecycle` | `PluginHostProbe` | `Pass` — begin → `SessionBusy` → refused `open(Docked)` → adapter-free `close` converge → `end_session` → slot/generation-bumped re-begin → Destroy (Issue #389) |
| `host-terminate` | `PluginHostProbe` | `Pass` — `Destroy` → `notify_terminated(HostLost)` → `Terminated{HostLost}` exactly once, second notify no-op (Issue #389) |
| `open-or-degrade` | `PluginHostProbe` | `Pass` — `open_or_degrade(Docked→Floating)` double refusal → `Failed{Unsupported}`, zero `Degraded`/attach-family events (Issue #389) |
| `dock-pixels` | `PluginHostProbe` | `NotRun` — no surface, no `VisualCapture` driver |

Verdict honesty is the point: only steps that really executed against
the binary may `Pass`; editor attach/dock/pixels stay
`Unsupported`/`NotRun` until a real editor fixture exists. A missing
artifact fails closed as `NotRun`.

## Scenario catalog

The nine cases are declared once in `HEADLESS_MATRIX`
(`scenario::HEADLESS_MATRIX`) — a driver-independent
`ScenarioMatrix`/`ScenarioEntry` table from `ruviform-ui-test`'s
`scenario::catalog` module. `run_all` is the select-all case of
`run_selected`, which sequences each selected entry's `run_entry`
call in catalog order; the lane binary, the integration tests, and
the `plugin-dock-console` binary read the same table. The
`real-dock` lane keeps its own session-scoped catalog
(`real_entries` in `real_dock`) consumed by both the live session
path and the environment-blocked fail-closed path; its metadata-only
projection lives in `real_catalog` (GPUI-free, available in every
build) so the console can name real selectors without compiling the
lane.

`ScenarioMatrix::project` answers "would `run_scenario` pre-flight
pass this entry on this driver" from declared capabilities only — it
executes nothing and upgrades no verdict.

## Artifact resolution

1. `RUVIFORM_PLUGIN_DOCK_TEST_VST3` — explicit file path (env only);
2. `current_exe()`'s `deps/` directory;
3. `current_exe()`'s own directory.

File name: `{DLL_PREFIX}ruviform_plugin_test_vst3{DLL_SUFFIX}`.

## Lane

```sh
mise run build ruviform-plugin-test-vst3   # build the cdylib
mise run test-ui-plugin-dock               # run the lane → evidence/rf631/
```

The lane is manual/opt-in — it is not a CI-required gate. JSONL
evidence lands in `evidence/rf631/plugin-dock-scenario.jsonl`.

## Real-window dock lane (`real-dock` feature, opt-in)

With `--features real-dock` the crate additionally links
`ruviform-ui-gpui` (`real-scenario`) and exposes the
`plugin-dock-real-scenario` binary, which runs the
`rf631.plugin-dock-real` cases through `RealOsGuiTest` on **real OS
windows**:

| case | verdict intent |
| --- | --- |
| `real-window-mount` | `Pass` only when the `plugin_dock_view_spec()` mount and target resolution really ran on a live window |
| `real-native-slot` | `Pass` only for steps that executed: declare → bind → renderer projection → `lease_parent` (real container HWND via `GpuiSurfaceAdapter`) → validate → `container_token` observation → stale/forged lease rejection → release |
| `real-editor-attach` | `Pass` only when `test_editor`'s real `IPlugView` attaches a real child HWND under the leased container (OS-observed: `EnumChildWindows`/`GetParent`/`IsWindowVisible`) and `session.close` destroys it (`IsWindow(child)==0`, events `[Attached{Docked}, Closed]`); observation mismatch → `Fail`, defined refusal → `Unsupported` |
| `real-capture` | window/slot-region pixels only — detail states the capture target is not plugin-editor pixels |
| `real-dcomp-pixel` | `Pass` only when the docked `test_editor` child HWND's **screen-composited** pixels (screen-DC `BitBlt` of its `GetWindowRect` region — what the user sees) show `EDITOR_COLOR` green; lifecycle verified + no green → `Fail` (the known GPUI DComp topmost occlusion regression), pixels unevaluable → `NotRun`, defined refusal → `Unsupported`. `PrintWindow(child)`/`GetDC`+`BitBlt` are recorded as `note`-level diagnostics (the child's own raster/surface, not the on-screen composition), plus query-only occlusion notes (`occlusion_exstyle`/`occlusion_zorder`/`occlusion_hit_test` — root/parent/child styles, sibling z-order, `WindowFromPoint` hit-test). `--pixel-artifacts <dir>` optionally writes the captured frames as PNG |
| `real-dcomp-foreground` | `Pass`/`Fail`/`NotRun`/`Unsupported` — GPUI root の z-order/foreground を段階変異（raise → `SetForegroundWindow` → topmost → restore）し、各段階で hit-test + screen green を再観測して外部 HWND stacking と compositor 内部遮蔽を二分する probe。verdict は probe 実行+状態復元+lifecycle のみ — screen green 出現の有無は二分の結果データであって verdict 入力ではない。`SetForegroundWindow` の foreground-lock 拒否は記録のみ、`SetWindowPos` の拒否・復元検証失敗は `Fail` |
| `real-dcomp-present` | `Pass`/`Fail`/`NotRun`/`Unsupported` — 変異なしの presentation probe（Issue #532 残余候補の判別）: `EnumWindows` top-level inventory（`WindowFromPoint` が skip する hidden/disabled/transparent/layered/cloaked cover を捕捉）・per-HWND presentation attributes（`DWMWA_CLOAKED`/window region/update region/layered）・`PrintWindow` flag 変種（`0`/`PW_CLIENTONLY`/`PW_RENDERFULLCONTENT`/複合）+ window DC・screen-vs-suspect 画素 fingerprint（quantized-bin match fraction）を child ごとに記録。verdict は probe 実行+lifecycle のみ — cover/fingerprint/green の観測値はすべて二分データで verdict 入力ではない |
| `real-dcomp-uncovered` | `Pass`/`Fail`/`NotRun`/`Unsupported` — uncovered-baseline bisect（Issue #532 compositor-internal occlusion 確定）: `osmut` staged mutation（raise → `SetForegroundWindow` → topmost）で遮蔽なし環境を作り、hit-test role 集計で `uncovered` を判定し、`std::thread::sleep` の累積 settle 曲線（200/500/1200ms）で hit-test+screen green+`has_update` を再観測、post-settle に `PrintWindow` flag 変種+window DC を記録して verified restore。自 process top-level window は inventory cap 非依存で全件・真の z_index 付きで記録（`own_windows`）。verdict は probe 実行+状態復元+lifecycle のみ — green/hit-test/uncovered の観測値はすべて二分データで verdict 入力ではない |
| `real-dcomp-bandescape` | `Pass`/`Fail`/`NotRun`/`Unsupported` — band-escape probe（Issue #532 脱出経路二分）: container snapshot（parent・`GWL_EXSTYLE`・screen rect・sibling z-order）を取り、強制 repaint（`RedrawWindow`+`RDW_UPDATENOW` で `update_pending` 解消）→ `WS_EX_LAYERED`+`SWP_FRAMECHANGED`+`SetLayeredWindowAttributes(LWA_ALPHA=255)`+repaint → settle → hit-test+screen green 観測、次に baseline exstyle 復元 → `SetParent(GetDesktopWindow())`+baseline rect を `HWND_TOP` に再適用 → settle → 観測、最後に parent/exstyle/rect を検証付き復元（sibling z-order は best-effort 記録）。verdict は probe 実行+復元+lifecycle のみ — green/hit-test の観測値は脱出方式採否データで verdict 入力ではない |
| `real-multiwindow-focus` | two real OS windows mounted; `Pass` only when `request_focus` → `window_focused` settles observe the A→B→A focus transfer. Focus release is recorded as observation only (the driver may honestly report `Unsupported`) |
| `real-os-input` | `Pass` only when the injection-effect preflight proves effective AND pointer move (physical cursor read-back), click, and `Tab` keystroke all land `Injected` on the focused real window; `EnvironmentBlocked`/`NotRun`/`Unsupported` outcomes map to the same-named verdicts — never a fake pass |
| `real-command-receipt` | `Pass`/`Fail`/`NotRun` — window D mounts the command-minting `sv_fader_dock_fixture`; Begin→Preview→Commit mints receipt-expecting intents whose `command_id`s return through `post_event` as applied/rejected `result` events, settling the binding Applied→Observed / Rejected; decoded `CommandReceipt`s populate `report.receipts` (AC1 link + AC4) |
| `real-window-dpi` | `Pass`/`Fail`/`NotRun` — live `window_geometry` read twice identically (origin, physical px, scale factor) cross-checked against the latest lifecycle `Resized`/`ScaleFactorChanged` within rounding slack; `window_geometry_sha256` + `dpi_change_driven: false` recorded — no DPI-change path is driven (the driver exposes no move/display-scale verb), observation consistency only |
| `real-window-ime` | `Unsupported` at capability preflight — the adapter never claims `Ime` (IME composition stays Issue #389 scope) |
| `real-window-close` | `Pass`/`Fail`/`NotRun` — armed `Veto`→`Defer`→`respond_close(Approve)` on window B and default `Approve` on window C drive real `CloseRequested`→`Destroyed` lifecycle events observed via `window_events`/`windows()` only; window A stays the sentinel (`windows()==[A]` at case end) |
| `real-editor-resize` | non-pixel oracle only (never pixels): `session.set_size(800×600)` through the production `InProcessEditorOps` on the real lease; `Pass` only when the child HWND's `GetWindowRect` reports the requested size (the fixture's `onSize` resizes its own window), the session stays `Attached`, events are `[Attached{Docked}, Closed]`, and close convergence holds; plugin-declined resize → `Unsupported`, accepted op whose geometry did not move → `Fail`, accepted-but-unobservable → `NotRun` |
| `real-editor-redock` | non-pixel oracle only: `session.reattach` under the same live lease through the real slot-ledger redeem; `Pass` only when every pre-reattach child HWND is destroyed (`removed()` → `DestroyWindow`), a new attach generation re-appears under the container (`rvf-test-editor-attach-{n}` title advanced), events are `[Attached{Docked}, Detached, Reattached{Docked}, Closed]`, and close convergence holds; plugin-declined re-parent → session `Failed{Unsupported}` + `[Attached{Docked}, Detached, Failed{..}, Closed]` → `Unsupported`; recreation mismatch → `Fail` |
| `real-session-terminate` | non-pixel oracle only: attach 中の `HostCommand::Deactivate`+`Destroy` → registry `notify_terminated(HostLost)`; `Pass` only when the session converges `Closed`+`concluded` with exactly `[Attached{Docked}, Terminated{HostLost}]`, a repeated notify is a no-op, every recorded child HWND reports `IsWindow(child)==0`, and the host survives (`Scan` ok + destroyed handle `InvalidHandle`); observation mismatch → `Fail`, defined refusal → `Unsupported` (Issue #389) |
| `real-session-degraded` | non-pixel oracle only: `open_or_degrade(Floating→Docked)` — the preferred mode's defined `Unsupported` refusal falls back to a real `OpenGui` docked attach; `Pass` only when `OpenOutcome::Degraded{mode:Docked}` lands, events are exactly `[Degraded{Docked}, Closed]` (never `Attached`), and the child HWND contract + close convergence hold; mismatch → `Fail` (Issue #389) |
| `real-session-recovery` | non-pixel oracle only: `open(Floating)` lands the defined `Failed{Unsupported}` (`editor_live=false`), then `recover(Docked)` attaches the real child HWND; `Pass` only when events are exactly `[Failed{Unsupported}, Recovered{Docked}, Closed]` and the child HWND contract + close convergence hold; mismatch → `Fail` (Issue #389) |
| `real-session-timeout` | non-pixel oracle only: real docked attach → ZERO `SessionDeadlinePolicy` → `begin_detach` → `tick_all` expires the in-flight detach; `Pass` only when the session lands `Failed{Timeout}`, events are exactly `[Attached{Docked}, Failed{Timeout}, Closed]`, and the child HWND tears down on close; mismatch → `Fail` (Issue #389) |
| `real-editor-visibility` | non-pixel oracle only: `session.set_visibility(false)` then `(true)` through the production `InProcessEditorOps` on the real lease; `Pass` only when hide lands `Hidden{Docked}` with every pre-hide child HWND destroyed (`removed()` → `DestroyWindow` — `IPlugView` has no hide verb, so hide IS detach), show lands `Attached{Docked}` with the child contract re-established on a new attach generation, events are `[Attached{Docked}, Hidden, Shown{Docked}, Closed]`, and close convergence holds; plugin-declined hide → `Unsupported` with `Attached{Docked}` preserved; hide/show mismatch → `Fail`, accepted-but-unobservable → `NotRun` (Issue #389) |
| `real-session-crash` | `Unsupported` — the fixture's process-killing crash fault (`test_fault` abort/exit) shares the host process in this lane and no out-of-process plugin host exists; `L1EditorOps` refuses every editor op with the defined `Unsupported`. Recorded inside the real session — an architectural boundary, never a fabricated `NotRun`/`Pass` (Issue #389) |
| `real-session-hang` | `Unsupported` — same boundary for the wedging hang fault (`test_fault` hang never returns); recorded inside the real session (Issue #389) |

The chain recorded is the production one:
`NativeSlot → GpuiSurfaceAdapter (実 container HWND) → lease_parent →
ParentSlotLease → InProcessEditorOps redeem → session.open(Docked)`.
`ParentSlotLease` keeps its opaque `u64` payload; only the canonical
`SharedSurface` ledger redeems it. `ScenarioCapability::NativePluginDock`
is reported only when the surface backend can mint a real container
(`surface::BACKEND_AVAILABLE && !gpui-test`).

Headless/non-interactive hosts fail closed: every case records `NotRun`
with an environment-blocked detail and exit code stays 0 — except a
`Fail` verdict exits 1 by design (e.g. `real-dcomp-pixel` under the
known unresolved GPUI DComp topmost occlusion is the honest
regression signal, not a lane defect). Every real-lane report's
`environment` records the raw `GPUI_DISABLE_DIRECT_COMPOSITION`
value (`null` when unset) so each run self-describes which
compositor path GPUI took; the env var is read once by GPUI at
process start, so set it on the lane invocation itself
(`GPUI_DISABLE_DIRECT_COMPOSITION=1 mise run test-ui-plugin-dock-real`).
The session-scoped AC4 evidence fields land on the same object:
`scale_factor` (the primary window's measured scale factor),
`window_identities` (per-window identity digests — id/title/
geometry/SHA-256), and `display_adapters` (the AC4 'OS and GPU'
GPU portion — the `EnumDisplayDevicesW` display-adapter census:
each adapter's `device_name`/`device_string`/`device_id`/
`device_key`, raw `state_flags`, decoded `primary`/
`attached_to_desktop` bits, and its `EDD_GET_DEVICE_INTERFACE_NAME`
monitor census; the GDI adapter view never claims which device the
renderer composited on). None of these is stamped on
`--blocked-probe`/headless rows — no session exists there to
measure them (fail-closed).
GPUI types never cross this crate's API or evidence schema.

```sh
mise run test-ui-plugin-dock-real   # builds test-plugin cdylib + runs the lane
cargo run -p ruviform-plugin-dock-test --features real-dock \
  --bin plugin-dock-real-scenario -- --out evidence/rf631/plugin-dock-real-scenario.jsonl
# opt-in pixel artifacts (auxiliary evidence, not the canonical lane):
cargo run -p ruviform-plugin-dock-test --features real-dock \
  --bin plugin-dock-real-scenario -- \
  --pixel-artifacts evidence/rf631/dcomp-pixel --out /tmp/real-lane.jsonl
```

Manual/opt-in — not a CI-required gate; requires an interactive
desktop session.

`plugin-dock-real-scenario --blocked-probe` is the deterministic
headless fail-closed proof: it generates the same reports a
`RealOsGuiTest::run` startup failure produces **without opening a
session**, self-checks that every catalog row is `NotRun` +
`observed.environment_blocked` with no `environment_kind=real-window`
claim, and exits 1 on any violation. Its detail states plainly that
no session start was attempted — it evidences the fail-closed
mapping, never an observed headless run. The
`test-ui-plugin-dock-real-headless` mise task writes the proof to
`evidence/rf631/plugin-dock-real-headless.jsonl`.

## Headless scenario console (`plugin-dock-console`)

`plugin-dock-console` is the headless consumer of the merged catalog —
a CLI for enumerating and selectively running the headless cells. It
is not a GUI and never executes real-window cases.

```sh
cargo run -p ruviform-plugin-dock-test --bin plugin-dock-console -- --list
cargo run -p ruviform-plugin-dock-test --bin plugin-dock-console -- \
  --run --case view-mount --out evidence/rf631/plugin-dock-console.jsonl
```

- `--list` — one JSON row per `HEADLESS_MATRIX.project()` cell
  (`type:"catalog"`, `case`, `scenario`, `driver`, `required`,
  `runnable`, `missing`). This is a **capability projection, not
  evidence**: it carries no verdict vocabulary and claims no
  execution.
- `--list --include-real` — appends the twenty-four `rf631.plugin-dock-real`
  rows from `real_catalog` with `session_required:true` and
  `lane:"real-window-dock"` (`runnable`/`missing` are `null`).
  Requires the `real-dock` feature; without it the option is refused
  with exit 2 rather than silently dropping the rows.
- `--run [--case <id>]... [--scenario <id>]... [--driver <name>]...`
  — filters `HEADLESS_MATRIX.cells()` (repeatable flags: OR within a
  flag, AND across flags; `--driver` takes the kebab-case name
  `fake`/`plugin-host-probe`) and executes the selection through
  `scenario::run_selected` — the same `run_entry` catalog path as the
  lane. JSONL output uses run id `rf631-plugin-dock` with lane
  `plugin-dock-console`, so a filtered run is never mistaken for
  full-lane evidence. `--out <path>` writes to a file (stdout
  otherwise).

Fail-closed semantics: unknown selector values, selector combinations
matching zero headless cells, CLI misuse, and any attempt to run a
real-lane case/scenario/driver all exit 2 with nothing executed (real
execution stays with the opt-in `plugin-dock-real-scenario` lane — the
console builds no second executor). A `Fail` verdict exits 1;
`NotRun`/`Unsupported`/`Blocked` are recorded outcomes and exit 0. A
selected case whose `artifact` prerequisite was not part of the same
selection reports `NotRun` — prerequisites are never fabricated.
