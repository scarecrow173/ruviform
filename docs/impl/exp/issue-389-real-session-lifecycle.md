# Issue #389 rf389-session-lifecycle — 実装実験記録（2026-11-17）

`docs/tasks/issue-389-real-session-lifecycle.md` の実装で試行・観測した
事項の記録。実装結果の正本は
[`docs/impl/ir/issue-389-real-session-lifecycle.md`](../ir/issue-389-real-session-lifecycle.md)、
イベントログは
[`issue-389-real-session-lifecycle-exp.jsonl`](issue-389-real-session-lifecycle-exp.jsonl)
を参照。

## 試行 1: `attempt_real_attach` への case 別 session drive 導入

- **目的**: 既存 case は全て `session.open(Docked)` を直打ちしていたが、
  degraded（`open_or_degrade`）・recovery（`open(Floating)` refused →
  `recover`）・timeout（attach 後 `begin_detach`+deadline 駆動）は
  open 駆動と post-open 観測が case 固有。
- **観測**: `drive_open`/`observe_open` を別引数で足すと
  `clippy::too_many_arguments`（8/7）。`AttachDrive<O, V>` bundle に
  まとめると struct literal 内の bare closure が `FnOnce` signature
  を推論できず `implementation of FnOnce is not general enough`
  （E0599 系）で 3 dcomp case が壊れた。
- **結論**: `AttachDrive` bundle を維持し、dcomp 3 case は
  `pixel_observe`/`foreground_observe`/`present_observe` の named
  `impl FnOnce` wrapper 経由に変更 — return-position `impl Trait`
  の bound が inner closure の引数型推論を導く。`ChildLifecycle`
  （fn pointer 2 本 + case 参照）には `Copy` を derive して
  `needless_pass_by_value` を解消。`conclude_editor_session` は
  `expected` 追加で 8 引数となったため `#[expect(clippy::
  too_many_arguments)]`（`real_entries` の `too_many_lines` と同じ
  先例）。

## 試行 2: recovery の `Failed` 起点の選択

- **観測**: `open_gui` は view 既存で `PluginFailed` を返すため、
  live editor からの `Failed` 遷移では `recover` を試せない。
  `open(Floating)` は `InProcessEditorOps` が非 Docked を
  `Unsupported` で拒否し `Failed{Unsupported}` +
  `editor_live=false` に落ちる — これが recoverable な起点。
- **結論**: refused-open 起点の recovery を採用。実走で events
  `[Failed{Unsupported}, Recovered{Docked}, Closed]` を確認。

## 試行 3: timeout の deterministic 駆動

- **観測**: `SessionDeadlinePolicy{timeout: Duration::ZERO}` は
  fail-fast として有効。`begin_detach` が in-flight `Detaching`
  request を arm し、`tick_all()` が 1 件を expire して
  `Failed{Timeout}` に収束させる。
- **結論**: `drive_detach_timeout` helper に分離（deadline policy
  install → begin_detach → request_deadline → tick_all → state
  assert）。実走で `timed_out=1`・events
  `[Attached{Docked}, Failed{Timeout}, Closed]` を確認。

## 試行 4: crash/hang の honest `Unsupported`

- **観測**: fixture の crash fault は hosting process を殺し、
  hang fault は lane を永久に塞ぐ — in-process lane では実行不能。
  out-of-process plugin host は存在しない（L1 は全 op 拒否）。
- **結論**: `required: &[NativePluginDock]` で executable row として
  catalog に残し（task spec 指定 — capability preflight を通して
  body が実 session 内に `Unsupported` を記録する形）、body が
  architectural boundary を説明して `Unsupported` を返す。
  `NotRun` 偽装・catalog 欠落・fault 実行のいずれも行わない。

## 試行 5: `real-dcomp-pixel` fail の切り分け

- **観測**: real lane 実走で `real-dcomp-pixel` が `fail`
  （`screen_pixels` `green_pixels=0/1019844`・`green_fraction=0.0`）。
  本 slice の変更は observe 駆動の配管のみで pixel 観測ロジックは
  不変だが、regression でないことを確認するため過去 evidence を
  調査 — `plugin-dock-real-scenario-baseline{,2}`・
  `-dcomp-disabled` の 3 実走全てで同一 fail（同一 rect・同一
  `green_pixels=0`）。
- **結論**: 既知の GPUI DComp topmost occlusion（#532 決裁待ち）
  であり本 slice 起因ではない。前 slice の `evidence/rf389/
  verification.md` も同じ結論を記録済み。verdict を緩和せず
  fail のまま正直に記録。
