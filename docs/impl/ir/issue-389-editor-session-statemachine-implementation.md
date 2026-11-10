---
id: "IMPL-9fK2mQxT7vBnRwYsJhGtUi"
type: "impl"
status: "completed"
title: "Issue #389 Plugin Editor Session fake host driver 状態機械 fixture 実装記録（driver async protocol fixture + SM-xx terminal-event 検証）"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/editor_session_driver.rs"
      - type: "file"
        path: "docs/plans/implement-issue-389-editor-session-statemachine.md"
      - type: "file"
        path: "docs/tasks/issue-389-editor-session-statemachine.md"
      - type: "file"
        path: "docs/test-specs/issue-389-editor-session-statemachine.md"
      - type: "file"
        path: "docs/impl/ir/issue-389-editor-session-statemachine-implementation.md"
    modified:
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-389-editor-session-statemachine.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-389-editor-session-statemachine.md"
    - "docs/plans/implement-issue-389-editor-session-statemachine.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-session-hardening.md"
    - "docs/plans/implement-issue-389-editor-session-statemachine.md"
    - "docs/test-specs/issue-389-editor-session-statemachine.md"
    - "docs/impl/ir/issue-389-editor-session-headless-implementation.md"
    - "docs/specs/issue-389-plugin-editor-session.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
  issues: [389]
---

# Issue #389 Plugin Editor Session fake host driver 状態機械 fixture 実装記録

計画・タスク・テスト仕様:
`docs/plans|tasks|test-specs/issue-389-editor-session-statemachine.md`。

## Summary

Issue #389 受入条件の「fake host driver で Open/Attach/Reparent/
Detach/Close・stale generation・Double close・Timeout・OS
unsupported・Plugin/GPU Host crash の terminal-event 検証」と
「失敗時 Floating/Generic Editor への明示的 degrade の契約」を、
driver 側 async protocol（`begin_*`/`complete_*`/`tick`/`tick_all`）
を実際に駆動する再利用可能な **`FakeHostDriver` fixture** として
固定する slice。既存 49 test（`tests/editor_session.rs`）は
session/ops を test 毎に手動で駆動しており、driver が担うべき
「dispatch → 後で complete → deadline 監視」のループを再現する
fixture がなかった — 本 slice はその fixture を
`tests/editor_session_driver.rs` に追加し、SM-01..16 の 17 case
で状態機械の terminal-event 不変条件を driver 視点で検証する。
contract 本体（`src/`）の変更はゼロ。

## Context

contract 層は approved design
（`docs/designs/issue-389-plugin-editor-session.md` 改訂 1・
`docs/designs/issue-389-session-hardening.md` D1–D5）どおり main に
存在し、hardening design の Data flow 節が定義する async driver
contract（`ops.*` を非同期 dispatch → `complete_*(request,
result)` → 定期的に `tick`/`tick_all`）を実際に回す fixture が
残存 gap だった。本 slice はその driver loop のみを対象とし、
新規契約・API 変更は行わない。

## Implementation

### Changes

- `crates/ruviform-plugin-host/tests/editor_session_driver.rs` —
  新設（約 1,400 行・test のみ）:
  - `FakeClock` — `SessionClock` の手動進行 clock。全 session の
    `SessionDeadlinePolicy` に注入し、wall-clock 非依存で
    deadline を検証する。
  - `ScriptedOps` — `EditorSessionOps` の script 化 fake。
    per-mode open 結果・close/reattach/size/visibility 結果・
    capabilities・`hung` op kind 集合・call log
    （`OpCall` 列で dispatch 順序を assert）を保持する。
    `unsupported()` は全 op `Unsupported` + caps 全
    `Unsupported`（L1 相当の defined refusal）。
  - `FakeHostDriver` — `PluginEditorSessions` registry・
    `ScriptedOps`・共有 `FakeClock`・pending completion queue
    （`PendingKind::{Open,Detach,Reattach,Close}`）・driver 側
    `instances`/`live_editors` 帳簿・`stale_drops`/
    `stale_rejections` 計数を所有する。verb は `begin_*` で
    request を mint して queue に積み、`pump()` が ready な op を
    FIFO で `ops.*` 呼出 → `complete_*` 配送する（snapshot
    semantics — 配送中に積まれた re-attach は次回 pump）。
    `advance()` + `tick_all()` が deadline を評価する。
    public session API のみを使い、session 内部状態は覗かない。
  - SM-01..17 — open/hide/show/resize/2 相 reparent/close の
    全 lifecycle（ops 呼出列 + event 列を全量 assert）、detach
    相 abort → `Failed{PluginFailed}` → `recover` rollback、
    re-attach 相 abort → `Failed{LeaseInvalid}`、generation
    recycle 後の stale session id drop（SM-04）・stale request
    id `StaleRequest` 拒否（SM-05）・double close 拒否（SM-06）、
    open/close/detach/reattach 各 request の timeout →
    `Failed{Timeout}` と遅延配送の `StaleRequest`（SM-07..09）、
    preferred mode `Unsupported` → `begin_degrade` で Floating へ
    明示 degrade（`Degraded` event のみ・`Attached` 系 event
    なし、SM-10）、全 op `Unsupported` adapter の defined
    refusal（SM-11）、Plugin host crash `HostLost` /
    GPU host crash `GraphicsLost` の `Terminated` exactly once
    （SM-12/13）、window close `Closed` と instance destroy
    `Terminated` の区別（SM-14）、close + `end_session` 後の
    generation+1 recreate（SM-15）、plugin resize 拒否の非致命
    `Err(Ops(Plugin))`（SM-16）、`ops.close` refusal の
    completion 保留 → timeout → retry 収束（SM-17）。
- `docs/` チェーン新設:
  `docs/plans/implement-issue-389-editor-session-statemachine.md`・
  `docs/tasks/issue-389-editor-session-statemachine.md`・
  `docs/test-specs/issue-389-editor-session-statemachine.md`・本記録。
  test-spec の SM-R7 として **focus/Generic Editor 境界**を記録 —
  focus verb は session port に存在せず（EH-R5 と同じ境界・
  #388 slot 層の責務）、「Generic Editor」は host 側 fallback UI
  の語彙で session `AttachMode` に存在しないため、本 slice が
  固定する degrade は Floating への明示遷移まで。
- 各 dir の README index 更新（plans/tasks/test-specs は
  `doc_maintenance.js apply` で managed index 再生成、
  impl/ir は手動行追加）。

### Decisions During Implementation

- **fixture は既存 contract のみを pin**: SM-xx は
  `begin_*`/`complete_*`/`tick`/`tick_all`/`notify_terminated`/
  `end_session` の public API 経路のみを駆動し、src の契約変更・
  新規 API・依存追加は一切行わない（`git status` で `src/`・
  `Cargo.toml`/`Cargo.lock`/`mise.toml` 差分ゼロを確認）。
- **detach 相は driver/platform 責務として script 化**:
  `EditorSessionOps` に `detach` 動詞は存在せず、reparent の
  detach 相は `complete_detach(request, result)` に script 済み
  result を配送する形でモデル化した（`script_detach` で
  `Err(Plugin)` 等を注入可能）。
- **driver 側 `live_editors` 帳簿は adapter 側の真実を追う**:
  `ops.open` が `Ok` を返した時点で真・`ops.close` の `Ok`
  配送/termination で偽 — `complete_attach` の受理とは無関係。
  review で「attach 受理時のみ真」という当初モデルが、stale
  だが成功した `ops.open` が残す adapter orphan を不可視にする
  ことが確認されたため、帳簿を adapter 真実に修正した。これに
  より SM-07 は timeout 後の orphan が後続 `close` で
  `ops.close` dispatch される経路を、SM-08 は stale な
  `ops.close` `Ok` が帳簿を偽にして再 dispatch を省く経路を
  固定する。
- **pending 配送は session 存在確認を ops 呼出より先に行う**:
  registry から消えた session 宛の op は adapter を呼ばず
  `stale_drops` に落とす（実 driver が dead session に仕事を
  出さない規律に合わせる — review 指摘対応）。
- **`hung` は dispatch 保留としてモデル化**: `pump` が hung
  kind を skip して queue に残し、deadline が request を収束
  させる。SM-07 が deadline 前 `tick` no-op・hung 中 `pump`
  deferral の両方を driver 経由で固定する。
- **async `complete_close` の error channel 不在を timeout 経由
  収束として固定**: `ops.close` 拒否時に `complete_close` を
  配送しないと request は `Closing` のまま deadline で
  `Failed{Timeout}` に落ち、後続 `close` が adapter 経路で
  再収束する — 現行 contract の定義済み挙動であり、SM-08
  （hang 経由）と SM-17（refusal 経由）の両経路を fixture で
  固定する（contract 変更はしない）。
- **`Err(_) => {}` arm は stale 以外の完了 error を握り潰すが、
  fail-closed**: `complete_*` の `Err(Ops(_))` は既に
  `mark_failed` 済みの session 側結果であり、driver が追加で
  state を変える術がない — event/state は session が emit した
  ものを test が全量 assert するため、握り潰しは観測を曇らせない。

## Validation

canonical（mise）— 全て実行済み:

- `mise run fmt-check` → exit 0
- `mise run check`（`cargo check --workspace --all-targets`）→
  exit 0
- `mise run lint`（clippy `-D warnings`）→ exit 0
  （初回 `doc_markdown`/`needless_pass_by_value`/`match_same_arms`/
  `single_match_else`/`assert_is_empty`/`single_match` 指摘を
  修正後に green）
- `mise run test`（`cargo test --workspace`）→ exit 0
  （`editor_session_driver` 18 passed・既存
  `editor_session` 49 passed を含む全 suite 非回帰）
- `mise run test-rt`（`cargo test --workspace rt_` filter）→
  exit 0
- `test-msrv` 相当の汎用 task は `mise.toml` に存在しない
  （`test-ofx-msrv`/`test-ui-test-msrv`/`test-observability-msrv`/
  `test-io-msrv` のみ・plugin-host 対象外のため canonical 未実行）

補助証拠（canonical = mise）:

- `cargo test -p ruviform-plugin-host --test editor_session_driver`
  → 18 passed / 0 failed
- `cargo clippy -p ruviform-plugin-host --tests -- -D warnings`
  → exit 0
- `mise exec rust@1.89.0 -- cargo test -p ruviform-plugin-host
  --test editor_session_driver` → 17 passed / 0 failed
  （MSRV 1.89.0 toolchain・review 修正前の補助証拠 —
  修正後は pinned toolchain の `mise run test` が canonical）

review-protocol: Reviewer A（cooperative）/ Reviewer B
（adversarial）の 2 系統レビューを各 1 回実施し、Issue
Candidate を統合・Validator 相当の source 照合で分類、Judge
で severity 判定した。P0/P1 の CONFIRMED issue はゼロ。
CONFIRMED P2/P3 のうち本 slice の成果物に関わるもの
（`live_editors` 帳簿の adapter 真実化・ops 呼出前の session
存在確認・hung/`unsupported()` の doc 精度・SM-16 の rule
対応付け・close refusal withhold 経路の未カバー・deadline
前 tick no-op の driver 経由未固定）は全て修正済み
（SM-17 追加・SM-07/08 強化・SM-R8 追加・docs 整合）。
残る指摘は P3 の観測メモ（`instances` 帳簿の prune なし・
pump の deferred 順序・既存 suite との一部重複 — いずれも
fixture 設計上許容）のみで blocker なし。

## Risks

- fixture green を実 Native Dock/実 plugin PASS と誤読される
  リスク → 本記録・test-spec・PR で NotRun/Unsupported を明記。
  fake driver の結果は contract-level の証拠であり、実機受入
  ではない。
- fixture が実 driver の責務を越えて session 内部状態を推測する
  退行 → driver は public API のみを使い、`editor_live` 相当は
  driver 側帳簿として明示的に保持する設計を維持する。
- hung op が queue を無限にブロックする退行 → hung は配送 skip
  のみで queue は前進し、`tick`/`tick_all` が必ず timeout 収束
  させることを SM-07..09 が固定。

## NotRun / Unsupported（正直な記録）

- 実 plugin editor（実 VST3/CLAP/AU binary）での
  open/attach/reparent/detach/close — **NotRun**（本 slice は
  headless fake driver fixture のみ）。
- 実 OS window・実 GPU・pixel compositing・実 Native Dock での
  受入 — **NotRun**（#388 gate・Synvelis #176・別 slice）。
- L1 out-of-process session ops の実経路 — `L1EditorOps` は全 op
  `Unsupported` の defined refusal（SM-11 はその語彙を driver
  経由で確認するのみ・実装変更なし）。
- focus verb — session port に存在しない（EH-R5/SM-R7 の境界
  記録のみ・#388 slot 層の責務）。
- 「Generic Editor」fallback UI — host 側 consumer/#388 の別 slice
  （本 slice が固定するのは Floating への明示 degrade まで）。
- GUI hang/フリーズが DAW/RT を巻き込まない実機検証 —
  **NotRun**（Issue #389 残 DoD）。
- `test-msrv` 汎用 task — `mise.toml` に存在しないため canonical
  未実行（上記の通り rust@1.89.0 補助証拠のみ）。

## Follow-ups

- 実 driver（GPUI/composition root）からの `tick`/`tick_all` 定期
  呼出接続と `FakeHostDriver` 相当の production 側参照実装
  （hardening の deferred・実 driver 側作業）。
- 「Generic Editor」fallback UI の consumer 側実装（#388 slot/
  window 層・別 slice）。
- async `complete_close` への error channel 追加の要否
  （現行は timeout 経由収束 — contract 変更を伴う別 Issue）。
- Issue #389 残 DoD: raw `GuiParentToken` の public API 除去、
  実 VST3/CLAP/AU・L0/L1 実機受入、macOS/Linux・multi-plugin
  sweep、GUI hang 非波及の実機検証。
