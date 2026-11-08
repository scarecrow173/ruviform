---
id: "SPEC-3S6R25rgxNv9VL9etjWsH9"
type: "spec"
status: "approved"
title: "Issue #311 RF-GT-02 Rust↔TypeScript IPC Schema Conformance・Deterministic Record/Replay・Fault Matrix"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/311"
    - "https://github.com/scarecrow173/ruviform/issues/257"
    - "https://github.com/scarecrow173/ruviform/issues/261"
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/discovery/issue-311-ipc-conformance-inventory.md"
  changes:
    added:
      - type: "dir"
        path: "ui/guitest"
      - type: "file"
        path: "crates/ruviform-ui/tests/ipc_conformance.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-webview/src/scheme.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-311-ipc-conformance-replay.md"
    - "docs/plans/issue-311-ipc-conformance-replay.md"
  depends-on:
    - "docs/designs/issue-156-gui-host-foundation.md"
    - "docs/designs/issue-181-multi-window.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-181-multi-window.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-311-ipc-conformance-inventory.md"
  derived-by:
    - "docs/designs/issue-311-ipc-conformance-replay.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-311-ipc-conformance-replay.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #311 RF-GT-02 — Rust↔TypeScript IPC Schema Conformance・Deterministic Record/Replay・Fault Matrix

> **歴史/旧方式（legacy）注記**: 本書の `rvf.ipc.v1` schema・versioned
> fixture corpus の契約部分は driver-neutral 経路（`WindowLifecycle::Action(Intent)`）
> でも共有・維持される。一方、`ui/guitest` の TS 実装・`window.ipc.postMessage` /
> `evaluate_script` 輸送・WebView DOM 前提の検証 lane は legacy/preview。
> 物理撤去は Issue #465 P2 の別判断。棚卸し正本:
> [issue-465-legacy-webview-inventory](../discovery/issue-465-legacy-webview-inventory.md)。

GUI Testing v1.0 / RF-GT-02（Issue #257 の合意済み分解、実装未着手）の実装 spec。
既存 `rvf.ipc.v1`・`rvf://data`・Binding 契約を改変せず、Rust/TS 両側で**同一の versioned fixture 集合**を処理し、決定的 record/replay と fault matrix を持つ共有テスト基盤を `ui/guitest` として新設する。

## What Should Be Built

1. **Versioned fixture corpus（単一正本）** — `ui/guitest/fixtures/` 配下に、`rvf.ipc.v1` intent/event envelope、`rvf://data` 要求・応答契約、Binding 契約メッセージ（CommandIntent/CommandReceipt/StateProjection/ViewportQuery/QueryReply）の正例・不正例を versioned JSON で保持する。含めるケース: 未認識 version/field、invalid payload、optional 能力（`job` progress の `detail` additive 拡張など）、bounded payload（`MAX_INTENT_BYTES`/`MAX_OP_LEN`/`max_chunk`/`maxInflight` 境界）。fixture 形式自体も versioned（`"v": 1`）で、形式変更は既存 fixture を書き換えず新バージョンを追加する。

2. **TS 側 decode/validation（`ui/guitest/src/`）** — 手続き型（zod 等非採用・依存ゼロ）の `rvf.ipc.v1` envelope decoder と Binding 契約メッセージ validator を実装し、fixture の全ケースを Rust 側と同一 version/同一意味（受理/拒否 + 安定エラーコード）で処理する。`rvf://data` 側は `dataUrl` 構築と応答ヘッダ契約（`X-Rvf-Total`・bounded body）の検証器を提供する。

3. **Rust 側 conformance** — `crates/ruviform-ui/tests/ipc_conformance.rs` が同一 fixture を読み、`parse_intent`/`Event::to_json`/`DataRegistry::read`/`UiRuntime` の挙動を fixture の期待と照合する。`rvf://data` の wire レベル（URL パース・status・`X-Rvf-Total`）は `crates/ruviform-ui-webview/src/scheme.rs` の既存テストモジュールを fixture 駆動に拡張して検証する（`webview` feature lane）。

4. **Recorder（opt-in・allowlist・bounded）** — `BindingPort` 境界を観測する記録器を実装する。記録するのは必要最小限のメタデータのみ: 方向（`in`=注入/`out`=産出）、logical tick、origin/target、correlation（commandId/queryId/gestureId）、sequence、schema version、選択した metadata（kind/phase/status 等の列挙値）。JSONL をテスト交換形式とし、行フォーマットに `v` バージョンを持つ。既定 OFF（未装着時ゼロコスト・副作用ゼロ）、収録対象は allowlist、件数・バイト数の上限を持ち、超過は `dropped` マーカーで明示する。値 payload・実 Media・Prompt/Grant・トークン・巨大 binary・長い自由文字列を収録しない（redaction は「収録しない」で実現する）。

5. **Deterministic replay + 独立 oracle** — 記録/シナリオの**入力列だけ**を実 `InteractionControl`/`CommandBinding`/`QueryBinding`/`ViewLink` へ注入し、産出を**文書化された契約規則のみを符号化した独立 ContractModel**（実装内部とコードを共有しない）と照合する。記録済み応答を再生して PASS としない。同一 seed/clock/入力で 2 回実行して出力が一致する決定性を検証し、seeded fault シナリオ生成（決定的 PRNG）で再現性を担保する。Rust 側は `HostEvent`/`Tick`/`CloseRequested` の入力列を `UiRuntime` に注入し、fixture 内の契約期待（diff レビュー対象）+ 機械的不変条件（intent→一意 result、envelope `v==1`、安定エラーコード集合）で照合する。

6. **Fault matrix** — late/reordered/duplicate ack、view detach/reattach、window destroy、stale source/revision、overlapping gesture、backpressure、timeout、cancel、unsupported、disconnect/crash を網羅するシナリオ群を TS（Binding 層）と Rust（Runtime 層）の両側に整備する。preview coalesce は契約能力の範囲でのみ許可し、Commit/Cancel/Transport/終端通知の silent drop を fixture と replay の両方で検出する。

7. **両 Consumer 向け mock fixture** — DAW 型（Synvelis 系: dispatch/subscribe 中心）と analysis 型（AudioLens 系: query/cancelQuery 中心）の双方で同一 BindingPort 契約を実装するスクリプタブル mock を `ui/guitest` に置き、conformance/replay/fault matrix が両者で再現することを CI で示す。#294 接続用の証拠項目（lane 名・seed・fixture version・Pass/Fail 判定）を記録形式に含める。

## Acceptance Criteria（Issue 本文が正本・トレース対応）

- **AC-1:** Rust/TS の正例・不正例 golden を双方が同一 version/意味で処理し、既存 IPC 互換を維持する。→ 1, 2, 3
- **AC-2:** 同一 seed/clock/入力で replay 決定性が得られ、重複適用や誤った Target/Revision の成功なし。→ 5
- **AC-3:** 遅延・逆順・detach・close・stale・backpressure で truth/receipt を偽装せず、terminal event 欠落なし。→ 5, 6
- **AC-4:** セキュリティレビュー（allowlist、redaction、容量上限、収集 ON/OFF、CI artifact 内容）合格。→ 4 + impl record のセキュリティ節
- **AC-5:** Ruviform 単体と両 Consumer 向け mock fixture が CI で再現、#294 へ接続するための証拠項目を記録。→ 6, 7 + mise/CI レーン

検証規則・判定語彙・fault matrix 個別行は [テスト仕様](../test-specs/issue-311-ipc-conformance-replay.md)、実装順序は [実装計画](../plans/issue-311-ipc-conformance-replay.md)、詳細設計は [設計](../designs/issue-311-ipc-conformance-replay.md) に展開する。

## Non-goals

- `rvf.ipc.v1` への新 kind・第二 IPC 正本・製品独自 Gateway の新設。
- Binding 契約型・`UiRuntime`/`DataRegistry`/`IntentQueue` 等、既存実装の挙動変更（バグを見つけた場合は別途報告し、本 Issue では契約を固定する側に回る）。
- 実 DOM/OS WebView/描画/a11y/実 RT sample accuracy の検証（RF-GT-01/03/04/05 と RT lane の範囲）。
- zod 等のスキーマライブラリの基盤必須採用・新規 npm 依存（validator は依存ゼロの手続き型で実装）。
- 常時稼働の Observability/Event Store。Recorder は opt-in テスト基盤であり、本番パスに配線しない。
- Synvelis/AudioLens 製品側シナリオ（各 Consumer 所有）。mock fixture は契約形状のみを写す。
