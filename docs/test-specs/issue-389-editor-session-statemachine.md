---
id: "TSPEC-Q5Bwy4OQ5KsQyE0Z4uhWg6"
type: "test-spec"
status: "approved"
title: "Issue #389 Plugin Editor Session fake host driver 状態機械 fixture テスト仕様（terminal-event・stale・timeout・degrade・crash）"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-389-editor-session-headless.md"
    - "docs/test-specs/issue-389-session-hardening.md"
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
    - "docs/specs/issue-389-plugin-editor-session.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 Plugin Editor Session fake host driver 状態機械 fixture テスト仕様

## Purpose

Issue #389 受入条件の「fake host driver で Open/Attach/Reparent/
Detach/Close・stale generation・Double close・Timeout・OS
unsupported・Plugin/GPU Host crash の terminal-event 検証」と
「失敗時 Floating/Generic Editor への明示的 degrade の契約」を、
**driver 側 async protocol を駆動する `FakeHostDriver` fixture**
で固定する。既存 ES-xx/SH-xx/EH-xx は session/ops を test 毎に
手動駆動しており、「dispatch → 後で complete → deadline 監視」
という driver の責務ループそのものを再現する fixture がない。
本仕様が追加する SM-xx はその driver 視点の残存 gap のみを対象
とし、新規契約は定義しない。ここで保証するのは状態機械と語彙
の契約であり、実 Native Dock / 実 plugin editor / 実 OS window
の PASS ではない。

## Feature

Plugin Editor Session の fake host driver fixture — driver が
`begin_*`/`complete_*`/`tick`/`tick_all` を通じて session を
駆動したとき、全失敗経路で terminal event（`Closed`/`Terminated`）
が exactly once で観測され、stale な session id/request id・
double close・timeout・OS unsupported・host crash が状態機械を
破らないこと。

## Fixture contract（`tests/editor_session_driver.rs`）

- `FakeHostDriver` は `PluginEditorSessions` registry・
  `ScriptedOps`（`EditorSessionOps` の script 化 fake）・共有
  `FakeClock`（`SessionDeadlinePolicy` へ注入）・pending
  completion queue・driver 側 `live_editors` 帳簿を所有する。
  public API のみを使い、session 内部状態は覗かない。
- verb（`open`/`recover`/`degrade`/`reparent`/`close`）は
  `begin_*` で request を mint し op を queue に積む。`pump()` が
  ready な op を FIFO で `ops.*` 呼出 → `complete_*` 配送する。
  `hung` 登録された op kind は配送されず残り、`tick`/`tick_all`
  が deadline 超過を `Failed{Timeout}` に収束させる。後の配送は
  `StaleRequest`。
- `reparent` は 2 相: platform detach（driver scripted result）
  → `complete_detach` → Ok なら新 request で `ops.reattach` →
  `complete_attach`。detach 相の失敗では attach 相の op を呼ばない。
- `close` は driver 側 `live_editors` 帳簿で adapter 呼出を制御
  する。帳簿は **adapter 側の真実**を追う: `ops.open` が `Ok`
  を返した時点で真（`complete_attach` の受理とは無関係 —
  stale だが成功した open でも adapter editor は存在し、
  orphan として可視であり続ける）、`ops.close` の `Ok` 配送
  または termination で偽。adapter editor 非生存の session
  は `complete_close` で収束し adapter を呼ばない。
- async `complete_close` は error を運ばない — `ops.close` 拒否は
  配送せず request を Closing のまま残し、deadline timeout で
  `Failed{Timeout}` へ落ちる。収束は後続の `close` 再試行
  （sync/async 経路）が担う — 現行 contract の定義済み挙動。
- session id が registry に存在しない配送は `stale_drops`、
  request 不一致の `complete_*` は `StaleRequest` を返し
  `stale_rejections` に記録 — いずれも state/event を変えない。

## Rules

- **SM-R1 (driver loop / timeout)**: in-flight request は
  deadline までに complete されなければ `tick`/`tick_all` が
  `Failed{Timeout}` + `Failed{Timeout}` event に収束させる。
  deadline 前の `tick` は no-op。後の配送は `StaleRequest` で
  event を増やさない。
- **SM-R2 (terminal once)**: 全 scenario の終端で
  `Closed`（graceful）または `Terminated`（abnormal）が
  exactly once 観測される。duplicate/missing/silent terminal
  はない — event 列を test が全量 assert する。
- **SM-R3 (explicit degrade)**: preferred mode の
  `Unsupported` 拒否に対し、driver が fallback mode を
  **明示的に** `begin_degrade` + `ops.open` で試行した場合のみ
  `Attached{fallback}` + `Degraded{fallback}` event に着地する。
  `Attached`/`Recovered`/`Reattached` event は発火せず、
  degrade は requested-mode attach（Native Dock PASS）と
  誤認されない。
- **SM-R4 (stale/generation safety)**: `end_session` で
  generation が進んだ旧 `PluginEditorSessionId` 宛の配送は
  session 不在で drop され、新 session の state/event/caps に
  影響しない。request id 不一致の `complete_*` は
  `StaleRequest` で拒否され state 不変。
- **SM-R5 (crash/window 区別)**: `notify_terminated`（instance
  destroy・Plugin/GPU host crash・window loss の語彙）は live
  session を `Closed` + `Terminated{cause}` exactly once に
  収束させ、in-flight request・後続配送は stale として無視
  される。graceful `close` の `Closed` event と区別され、
  window close は DSP instance を終了しない（session 帳簿の
  みが閉じる）。
- **SM-R6 (double close)**: concluded session への再 `close`
  は `InvalidState` で拒否され、`ops.close` は再 dispatch
  されず `Closed` event は重複しない。
- **SM-R7 (boundary 記録)**: focus verb は session port に
  存在しない（EH-R5 と同じ境界 — #388 slot 層の責務）。
  「Generic Editor」は host 側 fallback UI の語彙で session
  `AttachMode` には存在しない — 本 fixture が固定する degrade
  は Floating への明示遷移まで。
- **SM-R8 (非致命 refusal)**: `set_size`/`set_visibility` 等の
  同期 op が `Err(Ops(_))` を返しても session state は不変で
  event を発火せず、error は呼出側（driver）にそのまま返る
  — plugin-side preferred size/resize 拒否は defined rejection
  であり crash ではない。

## Examples

SM-xx case は `crates/ruviform-plugin-host/tests/editor_session_driver.rs`
の test 名に対応する。

| ID | Rule | 手順 | 期待 |
| --- | --- | --- | --- |
| SM-01 | SM-R1/R2 | open(Docked)→pump → hide → show → resize → reparent→pump×2 → close→pump | events `[Attached{Docked}, Hidden, Shown{Docked}, Detached, Reattached{Docked}, Closed]`。ops 呼出列 open→set_visibility×2→set_size→reattach→close。最終 `Closed` |
| SM-02 | SM-R1/R2 | Attached → `script_detach(Err(Plugin))` → reparent → pump | `Failed{PluginFailed}` + `Failed` event のみ（`Detached` なし）。`ops.reattach` 未呼出（2 相 abort）。`recover` → `Recovered` で rollback |
| SM-03 | SM-R1/R2 | Attached → reparent（detach Ok）→ pump → `ops.reattach=Err(LeaseRejected)` → pump | `Failed{LeaseInvalid}`。`Detached` は発火済み・`Reattached` なし。editor 生存のため `close` は `ops.close` を dispatch → `Closed` |
| SM-04 | SM-R4 | idA Attached → reparent（Detach pending 保持）→ crash(HostLost) → `end_session` → `begin_instance` で idB（同 slot・generation+1）→ open(idB)→pump | 旧 idA 宛 Detach 配送は `stale_drops`。idB は `Attached{Docked}` に正常着地し event は自身のもののみ。`session(idA)==None` |
| SM-05 | SM-R4 | open → state `Creating{request:r}` を読み `inject_attach_completion(id, r+1, Ok)` → pump | 注入配送は `StaleRequest`・`stale_rejections` 計数・state は `Creating{r}` 不変・event なし。pump で本来の request が `Attached` に着地 |
| SM-06 | SM-R6 | open→Attached → close→pump→`Closed` → 再度 `close` | 2 回目は `Err(InvalidState)`。`ops.close_calls==1`。events 末尾は `Closed` 1 件のみ |
| SM-07 | SM-R1/R2 | `hang(Open)` → open → deadline 前 `tick_all`/`pump`（no-op/deferral）→ `advance(>deadline)` → `tick_all` → `unhang` → pump → `close` → pump | deadline 前 tick は no-op・hung 中 pump は dispatch せず request は in-flight。`Failed{Timeout}` + `Failed{Timeout}` event。遅延配送は `StaleRequest` で event 不変だが `ops.open` `Ok` は adapter orphan を記録（`is_live` 真）— 後の `close` が `ops.close` を dispatch し `Closed` に収束 |
| SM-08 | SM-R1/R2 | Attached → `hang(Close)` → close → `advance` → `tick_all` → `unhang` → pump → `close` 再試行 | 1 回目 timeout で `Failed{Timeout}`（editor live 維持）。遅延 `ops.close` `Ok` 配送は ledger を偽にし `complete_close` は `StaleRequest` — adapter editor は既に閉じているため再 `close` は adapter 非呼出で `Closed` に収束。`Closed` event は最後の 1 件のみ |
| SM-09a | SM-R1/R2 | Attached → `hang(Detach)` → reparent → `advance` → `tick_all` | `Failed{Timeout}`。`Detached` event なし。`ops.reattach` 未呼出 |
| SM-09b | SM-R1/R2 | Attached → reparent → `hang(Reattach)` → pump（detach 配送・`Detached` 発火・Reattach は queue 残留）→ `advance` → `tick_all` | `Detached` 発火済み・`Reattached` なしで `Failed{Timeout}`（re-attach request の deadline）。遅延配送は `StaleRequest` |
| SM-10 | SM-R3 | `open_by_mode`: Docked=`Unsupported`・Floating=`Ok` → open(Docked)→pump → `degrade(Floating)`→pump | `Failed{Unsupported}` → `Attached{Floating}` + events `[Failed{Unsupported}, Degraded{Floating}]`。`Attached`/`Recovered` event なし — degrade は requested-mode attach と区別される |
| SM-11 | SM-R2/R3 | 全 lifecycle op `Unsupported`・caps 全 `Unsupported` の defined-refusal adapter → probe(caps) → open→pump → close | caps は全 `Unsupported`（`Ok(caps)` で返る — 実 `L1EditorOps` は caps も `Err` を返す点で別形）。open は `Failed{Unsupported}`。editor 非生存の close は adapter 非呼出しで `Closed` 収束 — defined refusal が fake success に化けない |
| SM-12 | SM-R5 | idA open→Attached → reparent（Detach pending 保持）→ `crash(HostLost)` → pump（遅延配送は stale）→ 再 `crash` | `Terminated{HostLost}` exactly once + `Closed` state。遅延配送は stale。2 回目 crash は event を増やさない |
| SM-13 | SM-R5 | `hang(Open)` → open → `crash(GraphicsLost)` → `unhang`→pump | in-flight open 中の crash で `Terminated{GraphicsLost}` 1 件 → `Closed`。遅延 attach 配送は `StaleRequest` で二重終端なし |
| SM-14 | SM-R5 | instance A: open→Attached→`close`→pump（`Closed`）。instance B: open→`crash(B)` | A は `Closed` event・`session_for_instance(A)` は `end_session` まで残存。B は `Terminated{HostLost}`・binding 即時解放 — window close と instance destroy が別終端として区別される |
| SM-15 | SM-R4 | open→Attached→close→pump→`end_session`→`begin_instance`→open→pump | 新 id は同 slot・generation+1。`Attached{Docked}` + event は新 session の `[Attached]` のみ。旧 id 宛 op は `session()==None` で拒否 |
| SM-16 | SM-R8 | Attached → `ops.size_result=Err(Plugin)` → `resize` | `Err(Ops(Plugin))` を返すが state は `Attached` 不変・event なし（resize 拒否は非致命 — plugin-side preferred size/resize 拒否の契約） |
| SM-17 | SM-R1/R2 | Attached → `ops.close_result=Err(Plugin)` → close → pump（dispatch されるが refusal で completion 保留）→ `advance` → `tick_all` → `close_result=Ok` → close 再試行 → pump | `ops.close` refusal は error channel 不在のため配送されず `Closing` に残り deadline で `Failed{Timeout}`（`is_live` 真のまま）。再 `close` が `ops.close` を再 dispatch し `Closed` に収束 — `Closed` event は最後の 1 件のみ |

## Guarantees

- driver が `begin_*`/`complete_*`/`tick`/`tick_all` を通じて
  駆動する全経路で、terminal event（`Closed`/`Terminated`）が
  exactly once 観測され、duplicate/missing/silent terminal が
  ないことが event 列の全量 assert で検証される。
- stale session id（generation recycle 後）・stale request id・
  double close・hung op の timeout・OS `Unsupported`・
  Plugin/GPU host crash の各経路で、状態機械が Failed/Closed
  の定義済み終端に収束し、後続 callback が状態を破壊しない。
- degrade は `Degraded{mode}` event と driver 記録で明示され、
  requested-mode attach（Native Dock PASS）と区別される。
- 既存 contract test（ES-01..24・SH-01..18・EH-01..05・
  editor_session 49 件）は挙動不変で green を維持する
  （新規 test file 追加のみ・API 不変・既存 file 無変更）。

## Non-goals

- 実 plugin editor・実 OS window・GPUI driver・GPU/実機での
  検証（`NotRun` — 実機受入は #388 gate・Synvelis #176・別 slice）。
- L1 out-of-process での session ops（`L1EditorOps` は全 op
  `Unsupported` の defined refusal — SM-11 はその語彙を driver
  経由で確認するのみで実装変更なし）。
- focus verb の session port への追加・Generic Editor fallback
  UI の実装（SM-R7 の境界記録のみ）。
- async `complete_close` への error channel 追加など contract
  変更（SM-08 は現行の timeout 経由収束を固定するのみ）。
- RT thread からの session 呼出（型設計上不可能 — 既存 EH
  non-goal と同じ）。
- GUI hang/フリーズが DAW/RT を巻き込まない実機検証
  （`NotRun` — Issue #389 残 DoD）。

## Risk

- fixture が実 driver の責務を越えて session 内部を推測する
  退行 → driver は public API のみを使い `editor_live` 相当は
  driver 側帳簿として明示（plan の Design Inputs 参照）。
- fixture green を実 Native Dock PASS と誤読 → test-spec/IR/PR
  で NotRun・非 PASS を明記し、fake driver の結果を実機受入と
  混同しない。
- pending queue の hung 項目が pump を無限にブロックする退行
  → hung は配送 skip のみで queue は前進し、`tick` で必ず
  timeout 収束することを SM-07..09 が固定。
