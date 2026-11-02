---
id: "TSPEC-ajLVDeHdxTEsvm0JMWptbM"
type: "test-spec"
status: "approved"
title: "Issue #181 Multi-Window Host — test spec"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-181-multi-window.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-181-multi-window.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-181-multi-window.md"
    - "docs/designs/issue-181-multi-window.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  gate: "M7"
  issue: 181
---

# Issue #181 Multi-Window Host — test spec

## Feature: Window 識別とイベント envelope（R1, R2, R3, R4）

### Rule: `WindowId` は非ゼロ・一意・単調・再利用しない安定 ID

- **Example**: `WindowId::new(0)` は `None`、非ゼロ値で生成した ID は `get()` で元値を返し、比較・Hash キー・（逆）シリアライズが可能。
- **Example**: FakeHost が採番した `w1`/`w2`/`w3` は全て異なり、破棄した Window の ID が後続生成に再利用されない。

### Rule: `WindowEvent` envelope が発信元を保持する

- **Example**: `WindowEvent::Windowed{window: w1, event}` の `window` が発信元 `w1` を保持する。`Tick` は `WindowId` を持たない。

### Rule: `HostEvent` 追加 variant は additive

- **Example**: `Moved{x,y}`/`Destroyed` が構築・match 可能で、既存 variant の挙動は不変（既存 runtime テストが green のまま）。

## Feature: IPC 応答の宛先 routing（R7, R10）

### Rule: intent の応答は発信 Window へのみ routing される

- **Example**: `rt.handle_window(w1, Message(intent_i1))` の `result` event は `RoutedEvent{Some(w1)}` のみ。`w2` 宛イベントは発生しない。

### Rule: 複数 WebView の `id` 衝突は宛先で分離される

- **Example**: `w1` と `w2` が同じ `id: "i-1"` の intent を発行 → `w1` の応答は `Some(w1)`、`w2` の応答は `Some(w2)`。両方の Window が各々の相関 id の応答を正しく受け取る。

### Rule: wire 形式不変

- **Example**: `handle_window` 経路で生成される `Event` の `to_json()` は `handle` 経路と同じ schema（`v:1` envelope）— `WindowId` は JSON に出現しない。

## Feature: CloseRequested / Destroy 分離（R5, R6）

### Rule: `Approve` は即破棄し `Destroyed` が続く

- **Example**: `CloseRequested` → `respond_close(Approve)` → Window は宛先リストから消え、`Destroyed` が記録される。ループ（FakeHost）は継続。

### Rule: `Veto` は Window を生存させる

- **Example**: `CloseRequested` → `respond_close(Veto)` → Window は `windows()` に残り、以後の `eval`/`resize`/`Message` が通常通り動作。

### Rule: `Defer` は close-pending で生存させ後続破棄を許す

- **Example**: `CloseRequested` → `respond_close(Defer)` → Window は残存し close-pending。後で `destroy_window` または次回 `CloseRequested`+`Approve` で終端。

### Rule: 未回答 `CloseRequested` は既定 `Approve`（その Window のみ）

- **Example**: `CloseRequested` に回答しない → FakeHost が既定 Approve を適用しその Window のみ `Destroyed`、他 Window とループは継続。

### Rule: `request_close` は veto-able 経路を通る

- **Example**: `request_close(w)` → `CloseRequested` が発行され、`Veto` で Window が残る（即破棄ではない）。

### Rule: アプリ終了・最終 Window ポリシーは consumer 所有

- **Example**: 全 Window 破棄後も FakeHost は `exit()` まで継続する（auto-exit しない）。

## Feature: 失敗フォールバック（R11）

### Rule: 破棄済み/不明宛先への配送はプロセスを壊さない

- **Example**: 破棄済み `w` への `eval` は silent drop（panic なし）、`resize`/`focus`/`respond_close`/`destroy_window` は `Err(UiError)`。

### Rule: not-ready 宛先の eval は bounded 保留

- **Example**: `Ready` 前の `eval` は pending へ積まれ、`Ready` で順序通り flush される。上限超過は drop-oldest。

### Rule: `create_window` 失敗は `Err` で Window を残さない

- **Example**: 失敗注入した FakeHost の `create_window` は `Err` を返し `windows()`/`Destroyed` に変化なし。

## Feature: Job / Window 寿命分離（R7, R8）

### Rule: Window Close で Job は自動取消されない

- **Example**: `w1` 発の Job 稼働中に `w1` の `CloseRequested` → `closing` event は `Some(w1)` へ届くが `has_running_jobs()` は true のまま。Job は後続 drain で完了する。

### Rule: Job event は owner Window へ routing される

- **Example**: `w1` 発 Job の progress/finished は `RoutedEvent{Some(w1)}`、`w2` 発 Job は `Some(w2)`、`dispatch_op` 発 Job は `None`（unrouted）。

### Rule: owner Window 破棄後の Job event は安全に drop される

- **Example**: `w1` 破棄後に Job 完了 → `RoutedEvent{Some(w1)}` が生成されるが配送層（FakeHost.eval）が silent drop、プロセスは継続。

### Rule: `cancel_window_jobs` は opt-in の Window-scope 取消

- **Example**: `cancel_window_jobs(w1)` は `w1` 所有 Job のみ cancel（`w2` 所有は継続）し、cancelled event は `Some(w1)` 宛。

## Feature: 単一 Window baseline 非回帰（R14）

### Rule: `handle`/`run` 経路の既存セマンティクスは不変

- **Example**: 既存 `close_requested_cancels_jobs_and_emits_lifecycle` を含む `tests/runtime.rs`/`tests/data_registry.rs` が非回帰 green。
- **Example**: `handle(Tick)` は owner に関係なく全 Job event を `Vec<Event>` で返す（unrouted 扱い）。

## 実施形式

- `crates/ruviform-ui/tests/multi_window.rs` — FakeHost（`WindowHandle` 実装 + close 決定機構 + bounded pending）駆動の headless 契約テスト。
- `crates/ruviform-ui/tests/runtime.rs` — `handle_window`/`drain_jobs_routed` の直接単体検証（routing/owner/closing 差異）。
- `mise run fmt-check`/`check`/`lint`/`test` green、`check-ui-webview`/`lint-ui-webview`/`test-ui-webview` green（`webview` feature lane）。
- `demo_multi` の実機実行は実施範囲のみ記録（CI 対象外、未検証は明示）。
