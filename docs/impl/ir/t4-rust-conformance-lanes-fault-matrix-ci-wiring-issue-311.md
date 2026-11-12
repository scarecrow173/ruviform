---
id: "IMPL-034Rxm9vEvzTujyztPoAQk"
type: "impl"
status: "completed"
title: "T4 Rust conformance lanes + fault-matrix CI wiring (Issue #311)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/tests/ipc_conformance.rs"
      - type: "file"
        path: "ui/guitest/test/faultmatrix.test.ts"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-webview/src/scheme.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
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
  implements:
    - "docs/tasks/issue-311-t4-fault-matrix-lanes.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-311-t1-fixture-corpus.md"
    - "docs/tasks/issue-311-t2-ts-wire-validators.md"
    - "docs/tasks/issue-311-t3-recorder-replay-oracle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-311-ipc-conformance-replay.md"
    - "docs/designs/issue-311-ipc-conformance-replay.md"
    - "docs/test-specs/issue-311-ipc-conformance-replay.md"
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
---

# T4 Rust conformance lanes + fault-matrix CI wiring (Issue #311)

## Summary

同一 versioned fixture corpus を消費する Rust 側 conformance lane 2 本（ruviform-ui integration test + ruviform-ui-webview scheme unit lane）と host window-lifecycle scenario runner を実装し、seeded fault-matrix の TS 側 lane・mise タスク・CI `ui-verify` 配線・AGENTS.md 同期を完了。product コードの変更はゼロ（既存 `#[cfg(test)]` と新規 test target のみ）。

## Context

- 実施タスク: docs/tasks/issue-311-t4-fault-matrix-lanes.md
- 上流: design §8（Rust lane 分担）・test-spec（受入ゲート）・AGENTS.md 検証入口規約（mise run 経由）
- 前提: T1–T3 の corpus・validator・replay/oracle

## Implementation

### Changes

- `crates/ruviform-ui/tests/ipc_conformance.rs` — `CARGO_MANIFEST_DIR` 相対で `ui/guitest/fixtures` を読込み:
  - `ipc_intents_fixture_cases`: 全 intent case を実 `parse_intent` に通し accept/code/id/op/params を照合（`bodyGen: pad-intent` の byte 生成も対応）
  - `ipc_events_fixture_cases`: `Event` は Serialize-only + non_exhaustive のため、公開 constructor（result_ok/result_err/job_*等）で生成した wire が `to_json` の `"v":1` envelope + 各 kind 相関 field を pin に一致することを照合。`notify` は Rust 側 constructor 不在のため skip（TS decode lane が担当 — コメント明記）
  - `ipc_data_registry_cases`: `DataRegistry` + `DataRequest`（u64 境界）で range/maxChunk/not_found/out_of_range を照合。`rawQuery` case は `serve_data` の `parse_query` と同じ分割規則で offset/len を抽出。bad_name/bad_params は scheme 層の責務として webview lane へ委譲
  - `host_window_lifecycle_scenario`: `HostState` runner（named window → `WindowId`・liveness で delivery-layer drop を model 化）が `UiRuntime::handle_window`/`drain_jobs_routed`/`has_running_jobs` を駆動し、expect.events を ordered-subsequence で照合。`waitJobs` は bounded poll（timeoutMs 付き）
- `crates/ruviform-ui-webview/src/scheme.rs` `mod tests` 内 `data_fixture_cases`: 同 fixture を実 `serve` 経路に通し、URL 構築（rawQuery/offset+len・`request.host` で `http://rvf.<ns>` rewritten 形）→ status/bodyLen/`X-Rvf-Total` 照合。bad_name・bad_params の scheme 層検証をここで担う
- `crates/ruviform-ui-webview/Cargo.toml` — `[dev-dependencies] serde_json`（fixture loader、test-only 追加・理由をコメント記録）
- `ui/guitest/test/faultmatrix.test.ts` — `genAckStorm`/`genAttachStorm` を複数 seed で実 path + oracle に注入、決定性・terminal 保存・ghost command 非発生を検証（10 tests）
- `mise.toml` — `check-ui-guitest`・`test-ui-guitest` 追加（npm ci → typecheck/test の既存 UI lane 規約踏襲）
- `.github/workflows/ci.yml` — `ui-verify` job に guitest の 2 step 追加
- `AGENTS.md` — mise タスク表に guitest lane 2 行 + `ui-verify` 説明文に lane 追記

### Decisions During Implementation

- **`Event` は producer lane のみ**: Serialize-only + `#[non_exhaustive]` で deserialize 不可のため、Rust 側は「constructor が pin wire shape を出す」検証に限定。decode/拒否面は TS lane が担う — 設計時の lane 分担どおり
- **u64/usize 境界は `try_from`**: `DataRequest.offset`/`len`・registry `max_chunk` は u64、Vec 長は usize。`as` cast を使わず `usize::try_from`/`u8::try_from` で clippy `cast_possible_truncation` 準拠
- **destroyed window の event は delivery-layer drop として model 化**: `drain_jobs_routed` は destroyed 宛 event を生産し続ける（error ではない）。runner は `alive` set で drop を再現し、実 wire 到達意味のみを trace に残す
- **clippy `too_many_lines`（101/100）回避**: host runner を `HostState` struct（`new`/`collect`/`run_op`）に分割 — `#[allow]` 不使用で構造的に解消
- **fault-matrix は専用 test ファイルとして実装**: task doc の計画どおり `test/faultmatrix.test.ts`。seeded generator で静的 corpus 外の fault 組合せを検証

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

実施環境: Windows 11 x64・Git Bash・Rust 1.99.0（mise pin）・Node 26.12.1。

| Check | コマンド | 結果 |
| --- | --- | --- |
| Rust conformance | `cargo test -p ruviform-ui --test ipc_conformance`（mise run test 経由相当） | PASS — 4 tests（intents/events/data/host scenario） |
| webview scheme lane | `mise run test-ui-webview` | PASS — 8 tests・`data_fixture_cases` 含む |
| TS 全 lane | `mise run test-ui-guitest` | PASS — 135 tests |
| fmt | `mise run fmt-check` | PASS・差分なし |
| workspace check | `mise run check` | PASS・0 warnings |
| workspace lint | `mise run lint` | PASS・0 warnings（clippy -D warnings） |
| feature lint | `mise run lint-ui-webview` | PASS・0 warnings |
| workspace test | `mise run test` | PASS（実行済み・全クレート） |

### 未実施（理由付き）

- `rt-verify`・`icc-verify`・`bench-smoke`・`l1-real-verify`: 本 Issue のスコープ外 lane（RT/icc/bench/実プラグイン lane は変更対象なし）
- 実 WebView/描画検証: RF-GT-01/04 の範囲（本 Issue は headless 契約層のみ）
- `.github/workflows/ci.yml` の CI 実実行: ローカル検証のみで GitHub Actions 上の実行は未確認（`ui-verify` job 構成は既存 lane と同一形式）

## Security Review（Issue 受入: allowlist・redaction・容量上限・収集 ON/OFF）

- **allowlist**: `jsonl.ts` の record 語彙は明示 allowlist（`v`/`tick`/`dir`/`type` + field 白名単）。未知 field は parse 時 reject
- **redaction**: recorder は文字列 128 文字 truncate・機微 field（media/prompt/grant/token/巨大 binary）は denylist で構造的に収集しない
- **容量上限**: `maxEntries`/`maxBytes` 双 bound、最初の overflow で `sys/dropped` marker + 以後 drop 計数（entries cap 時も marker 枠確保済み）
- **収集 ON/OFF**: 既定 `enabled:false`、off 時は recorder が no-op 透過（テストで検証済み）
- **CI artifact**: 録画生成は opt-in、CI で artifact 化する経路なし（CI は replay 判定のみ実行）

## #294 接続証拠項目

- 同一 versioned fixture 群（`ui/guitest/fixtures`）を Rust serialize lane・TS decode lane・scheme lane の 3 経路で検証済み — 両側同一 version/同一意味の golden 証跡
- replay 決定性: scenario×seed 全組合せで 2 回実行 bit 同一 trace
- fault 保存: delay/reorder/duplicate/detach/close/stale/backpressure で truth/receipt 偽装なし・terminal event 欠落なし（oracle 照合で検証）
- Ruviform 単体 + DAW/analysis 両 consumer mock 形で CI 再現可能

## Risks

- `ipc_conformance.rs` の `waitJobs` は実時間 poll（job 実行系の性質上 logical clock 化不可）。timeoutMs で bounded、flaky 耐性は subsequence 照合で担保
- `data_fixture_cases` は `serve` が `pub(crate)` のため unit test 内実装 — crate 内配置の制約上 integration test 化不可

## Follow-ups

- origin/main merge 後の最終検証・コミット（worker spec 手順どおり）
- fixture 変更時は `v` バンプ + diff レビュー（Issue 受入）
