---
id: "IMPL-034JxC4eRod3t2ZFdmLrJJ"
type: "impl"
status: "completed"
title: "PoC UI ハーネスホスト"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/28"
  changes:
    added:
      - type: "file"
        path: "poc/ui/Cargo.toml"
      - type: "file"
        path: "poc/ui/Cargo.lock"
      - type: "file"
        path: "poc/ui/.gitignore"
      - type: "file"
        path: "poc/ui/README.md"
      - type: "file"
        path: "poc/ui/RESULTS.md"
      - type: "file"
        path: "poc/ui/build.rs"
      - type: "file"
        path: "poc/ui/src/main.rs"
      - type: "file"
        path: "poc/ui/src/args.rs"
      - type: "file"
        path: "poc/ui/src/check.rs"
      - type: "file"
        path: "poc/ui/src/host.rs"
      - type: "file"
        path: "poc/ui/src/assets.rs"
      - type: "file"
        path: "poc/ui/src/metrics.rs"
      - type: "file"
        path: "poc/ui/src/exp/mod.rs"
      - type: "file"
        path: "poc/ui/src/exp/host.rs"
      - type: "file"
        path: "poc/ui/frontend/package.json"
      - type: "file"
        path: "poc/ui/frontend/package-lock.json"
      - type: "file"
        path: "poc/ui/frontend/build.mjs"
      - type: "file"
        path: "poc/ui/frontend/src/index.html"
      - type: "file"
        path: "docs/impl/exp/ac1-host-experiment-wry-tao-webview.jsonl"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/poc-ui-harness-host.md"
  implemented-by: []
  depends-on:
    - "../../plans/issue-28-ui-poc.md"
    - "../../designs/issue-28-ui-poc.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../specs/issue-28-ui-poc.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/issue-28-ui-poc.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "../exp/ac1-host-experiment-wry-tao-webview.jsonl"
    rejected: []
---

# PoC UI ハーネスホスト

## Summary

task `TASK-034JfGCUwtk0lzJvJPr36B`（plan `PLAN-034JfDEU8mhCDj4ldYFZUG` の T1）として、Issue #28 UI 技術検証の捨て場付きハーネス `poc/ui/` を独立 Cargo ワークスペース + `ui-poc` 単一バイナリで構築した。std-only CLI・capability 検出・`poc-metric.v1` 計測契約・tao/wry ホスト・rust-embed 埋め込みアセット・`--experiment host`（AC1）・AC8 隔離を実装・実測し、実機 Windows 11 + WebView2 155 で `status:"passed"` を 4 連続で確認した。全変更は未コミット（orchestrator 引き渡し）。

## Context

- 実施タスク: [../../tasks/poc-ui-harness-host.md](../../tasks/poc-ui-harness-host.md)（`TASK-034JfGCUwtk0lzJvJPr36B`）
- 上流: [plan](../../plans/issue-28-ui-poc.md)（T1）・[design](../../designs/issue-28-ui-poc.md)（ハーネス構成・`poc-metric.v1`・exit code・自己終了契約の正本）・[spec](../../specs/issue-28-ui-poc.md)（AC1/AC8）・[test-spec](../../test-specs/issue-28-ui-poc.md)（status/exit code・`--check`・記録要件）
- 前提制約: `unsafe` 不使用・windows-rs 直接呼出禁止・外部型を `host.rs`/`assets.rs`/`check.rs` に封じる・ルート `Cargo.toml`/`Cargo.lock`/`crates/`/`mise.toml`/`.gitignore` 無変更・コミットしない

## Implementation

### Changes

- **`poc/ui/` 独立ワークスペース**: `[workspace]`+`[package]`（`ui-poc` bin）+ resolver "2"。ルート `members` は明示リストのため `exclude` 不要で吸収されない（AC8）。`Cargo.lock`/`package-lock.json` はコミット対象、`.gitignore` は `node_modules/`・`*/dist/`・`out/`
- **`src/args.rs`**: std-only パース（`--experiment`/`--frames`/`--duration`/`--backend`/`--mode`/`--variant`/`--out`/`--check`/`-h`）。不正は `ArgsError`→exit 2、`--help` は usage stdout + exit 0
- **`src/check.rs`**: WebView2 レジストリ `pv`（HKLM 64/32 → HKCU）・wgpu adapter 先頭（`enumerate_adapters` を `pollster::block_on` で同期化、GUI 非生成）・OS/arch/rustc（`build.rs` 焼き込み `UI_POC_RUSTC`）を 1 回検出。`--check` は `poc-check.v1` JSON 出力・常に exit 0
- **`src/host.rs`**: tao `Window` + wry `WebView`（`WebViewBuilder::build(&window)` + `with_bounds` + 明示 `set_bounds`）。`run_return` + `WaitUntil(deadline)` で自己終了（done/timeout/close 何れでも確実に抜ける）。`Host` I/F（eval/resize/refresh_bounds/focus/finish）だけを実験へ露出し、`HostEvent` で正規化転送
- **`src/assets.rs`**: `rust-embed`（debug=FS・release=埋め込み）+ `with_custom_protocol("poc")`。`poc://app/` 配信（パス安全性・MIME）、`poc://data/` は T3/AC4 予約として 404。Windows では要求が `http://poc.app/...` に書き換わるため両形式受理
- **`src/metrics.rs`**: `poc-metric.v1`（schema/experiment/ts/env/params/metrics/status/reason?）。`passed|skipped|failed` と exit code 対応を集約、最終 stdout 行 + `out/*.jsonl` 追記、`summarize`（p50/p95/p99）を先行実装
- **`src/exp/`**: `Experiment` trait（setup/run/teardown）+ レジストリ。`host` 実験は Rust 所有のフェーズ状態機械（AwaitLoad→AwaitPing→AwaitEcho→AwaitResize→AwaitFocus→Done）で AC1 全メトリックを収集
- **`frontend/`**: `package.json`+lock・`build.mjs`（src→dist コピー + `.ui-poc-built` マーカー）・`src/index.html`（JS 観測エージェント: hello/ping/echo/RO/resize/focus リスナ/viewport ハートビート/focusCycle プローブ）
- **`RESULTS.md`**: AC→実験→証跡行対応表・環境・再現コマンド・文書評価欄 + AC1 実測行と AC8 隔離行
- **`README.md`**: 前提・`npm ci && npm run build`・ビルド/実行/再現・`--check`・wgpu optional feature 逃げ道・既知挙動メモ

### Decisions During Implementation

- **`build_as_child` → `build(&window)`**: 子コンテナ経路では `set_bounds` 後も JS viewport が 900x600 に留まる実測（本機 WebView2 155）。`build` 経路は wry 親 subclass が WM_SIZE→`controller.SetBounds` を直結し追従するため採用。子ビュー bounds 追従の非対称は T4/AC3 へ引継ぎ
- **resize の冪等リトライ**: `controller.SetBounds` は非同期反映で同一値再送が合体/破棄される実測（RO が初期値のみ観測するフレーク）。viewport ハートビートを駆動源に ±1px 交互ジッタ付き `set_bounds` 再適用（最大4回・±4px 許容）を実装し、再現実行では retries:4 で追従
- **focus の観測設計**: OS フォーカスはバックグラウンド起動では取得不能（`SetForegroundWindow`+SendInput も失効、tao `Focused` は `is_active&&is_focused` 両立が必要で未発火）。実機では `focus_parent()`→`SetFocus(parent)`→wry subclass `WM_SETFOCUS`→`MoveFocus` の連鎖が DOM window blur→focus ペアを発生させる経路を発見し `mechanism:"window"` で検証。OS 非依存の DOM 要素プローブ（`mechanism:"dom_element"`）はフォールバックとして残置
- **証跡の診断フィールド**: `resize_samples`/`viewport`（hasFocus・visibility）/`api`/`exit_reason`/`timeout_hit`/`done_signal` をメトリックに残す設計とし、失敗時の切り分けと PoC 知見の転記を両立
- **`--help` は exit 0**（usage を stdout。不正引数のみ exit 2）

## Related Experiments

### Adopted

- [../exp/ac1-host-experiment-wry-tao-webview.jsonl](../exp/ac1-host-experiment-wry-tao-webview.jsonl) — AC1 host 実験の実施ログ（build_as_child 仮説→棄却・build 採用・resize リトライ・focus 経路発見・4 連続 passed）。結果は `RESULTS.md` AC1 行に集約済み

### Rejected

None.

## Validation

実施環境: Windows 11 x64 対話デスクトップ・Git Bash・mise `rust = "1.99.0"`。

| Check | コマンド | 結果 |
| --- | --- | --- |
| PoC ビルド | `mise exec -- cargo build --manifest-path poc/ui/Cargo.toml` | clean・0 warnings |
| fmt | `cargo fmt --manifest-path poc/ui/Cargo.toml -- --check` | 差分なし |
| clippy | `cargo clippy --manifest-path poc/ui/Cargo.toml -- -D warnings` | clean・0 warnings |
| `--check` | `ui-poc --check` | `poc-check.v1` JSON（WebView2 155.0.4283.62 / RTX 4070 SUPER (Vulkan) / rustc 1.99.0）・GUI 非生成・exit 0 |
| AC1 実機 | `ui-poc --experiment host` | `status:"passed"` × 4 連続（host-1792554016/4106/4329/4549）。全 AC1 メトリック収集、`focus_transition.mechanism:"window"`、`exit_reason:"metrics_complete"` |
| exit code | `ui-poc --experiment unknown` / 引数不正 | exit 2（usage を stderr） |
| 隔離 | `git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml` | 無出力 |
| ルート検証 | `mise run check` / `mise run lint` / `mise run test` | 全 PASS（変更前後で同一） |
| exp 監査 | `node .agents/skills/impl-doc/scripts/audit_experiment_log.js --json` | findings 0 |
| IR 監査 | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | findings 0 |

## Risks

- **resize/focus の環境依存**: WebView2 `SetBounds` 非同期反映と foreground activation 制約は実機依存。リトライ（4回）と DOM フォールバックで本機は安定させたが、他環境で `failed`/`skipped` になりうる。OS フォーカス前提の AC3 検査はこの制約を前提条件として設計する必要がある
- **`build_as_child` の viewport 非追従**: T4 隣接構成（sibling 子 HWND + WebView 子配置）で同様の問題に当たる可能性が高い。AC3 では子コンテナの親 HWND 管理と SetBounds 伝播を早期に確認すること
- **完了時刻の揺れ**: 完了直前の IPC イベントがデッドラインバッチに乗ると `timeout_hit:true` + `exit_reason:"metrics_complete"` が併記される（観測は成立・実時間 ~20s）。`--duration` の余裕は確保されている
- **`innerWidth` vs RO `contentRect` の非対称**: `innerWidth` はリサイズ後も 900 を報告する観測。ハートビートの w/h は診断用途に限定し、判定は RO `resize_observed` にのみ依存する設計とした

## Follow-ups

- T2–T5: `render`/`surface`/`ipc`/`frontend`/`dist` 実験と `metrics.summarize` の接続、`poc://data/` バイナリ応答の実装（AC4）
- T4: `build_as_child` / sibling 子 HWND 構成での bounds 伝播・フォーカス検査を `build` 経路の観測と対比する
- フォーカス検査の再定義検討: `mechanism:"window"` が観測できた経路（`focus_parent`→subclass `MoveFocus`）を正本として AC3 のフォーカス項目を定義するか、対話セッション前提の OS フォーカス検査を別途用意するかを T4 で判断
- wgpu MSRV 衝突（1.87 vs 宣言 1.85）の事実は `RESULTS.md` に記録済み。ルート採否時は宣言値見直しタスクへ連携
