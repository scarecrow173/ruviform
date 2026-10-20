---
id: "TASK-034JfH9TmzdaGlsimZsJiq"
type: "task"
status: "todo"
title: "PoC メッセージ境界計測（JSON RTT・バイナリ pull・evaluate_script push）"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/28"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/issue-28-ui-poc.md"
  implemented-by: []
  depends-on:
    - "../plans/issue-28-ui-poc.md"
    - "poc-ui-harness-host.md"
  blocks:
    - "poc-adr-finalize.md"
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-28-ui-poc.md"
    - "../designs/issue-28-ui-poc.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/issue-28-ui-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/issue-28-ui-poc.md"
  references: []
  defers: []
  deferred-by: []
---

# PoC メッセージ境界計測（JSON RTT・バイナリ pull・evaluate_script push）

## Work

plan `PLAN-034JfDEU8mhCDj4ldYFZUG` の T3。design の AC4 実験設計を正本として、Rust ⇄ フロントエンド間の 3 経路（JSON intent・カスタムプロトコル バイナリ pull・`evaluate_script` push）を計測し、併用の実現性を記録する。

- [ ] `src/exp/ipc.rs` に `--experiment ipc [--duration 10]` を実装（既定時間またはイテレーション完了で自己終了）
- [ ] **JSON RTT 経路**: JS が `{seq,t0}` を `window.ipc.postMessage` → Rust が `with_ipc_handler` で受け `evaluate_script` で seq を返送 → JS が `t1−t0` を計測。ペイロード 64B / 1KiB / 16KiB / 64KiB × 既定反復で `ipc_rtt_ms{avg,p50,p99}` と単方向 `ipc_ingest_msg_s`（msg/s）を記録
- [ ] **バイナリ pull 経路**: `poc://data/blob?bytes=N` に対する `fetch()` 反復で、`ArrayBuffer` 取得までの時間から `protocol_pull_MBps` とリクエスト毎レイテンシを記録（64KiB / 1MiB / 16MiB。Rust 側は合成 `Cow<'static,[u8]>` バッファを `Response` で返す）
- [ ] **push 経路**: Rust が `evaluate_script` で定速送信し、JS 側の受信間隔・ドロップ数・coalesce 挙動を `push_msg_s` + 観察として記録（wry/WebView2 が高頻度 `evaluate_script` を暗黙に coalesce/drop するかは観察項目 — design Open Questions）
- [ ] 3 経路併用の実現性所見（pull 型の間隔・coalesce 制御をフロント側で設計する必要の有無を含む）を `poc/ui/RESULTS.md`（AC4 行）と `docs/impl/exp/` ログに記録

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/issue-28-ui-poc.md](../plans/issue-28-ui-poc.md) — `PLAN-034JfDEU8mhCDj4ldYFZUG`（approved）。T3 の正本
- [../designs/issue-28-ui-poc.md](../designs/issue-28-ui-poc.md) — AC4 ipc 実験（3 経路・ペイロード系列・メトリックキー）の正本
- [../specs/issue-28-ui-poc.md](../specs/issue-28-ui-poc.md) — AC4 の正本
- [../test-specs/issue-28-ui-poc.md](../test-specs/issue-28-ui-poc.md) — 3 経路の記録要件・`status`/exit code 判定規則

## Dependencies

- 前提: `poc-ui-harness-host.md`（host・`with_ipc_handler`・`with_custom_protocol` の `poc://data/` 名前空間・metrics が前提）
- 後続をブロック: `poc-adr-finalize.md`（AC4 エビデンス供給）
- T2（render）・T4（surface）とは並行可能（別実験・プロセス分離で実行）

## Affected Files

- `poc/ui/src/exp/ipc.rs`・`poc/ui/src/exp/mod.rs`（レジストリ追加）— 新規/更新
- `poc/ui/src/assets.rs` — `poc://data/` の計測バイナリ応答ハンドラ（必要に応じて）
- `poc/ui/frontend/bench-shared/` — fetch タイミング・IPC シーケンス番号ブリッジ（T2 と共有部品）
- `poc/ui/RESULTS.md` — AC4 行追加
- `docs/impl/exp/` — 実施ログを追記

## Done When

- [ ] `--experiment ipc` が自己終了し、`ipc_rtt_ms{avg,p50,p99}`（64B/1KiB/16KiB/64KiB）・`ipc_ingest_msg_s`・`protocol_pull_MBps`（64KiB/1MiB/16MiB）・`push_msg_s` が `poc-metric.v1` で記録される
- [ ] `fetch()`/`ArrayBuffer` によるバイナリ pull が動作し、往復・スループット数値が記録される
- [ ] `evaluate_script` push の受信間隔・ドロップ・coalesce 観察が記録される
- [ ] 3 経路併用の実現性所見が `RESULTS.md` に記録される

## Verification

- [ ] `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment ipc --duration 10` — `poc-metric.v1` 最終行に `ipc_rtt_ms`・`ipc_ingest_msg_s`・`protocol_pull_MBps`・`push_msg_s` が含まれ `status` 規約どおり
- [ ] `grep -c "ipc_rtt_ms\|protocol_pull_MBps\|push_msg_s" poc/ui/RESULTS.md` — AC4 行にメトリックキーが存在する（実施後）
- [ ] Manual: `poc/ui/RESULTS.md` に 3 経路の数値行・観察・再現コマンドが存在する（補助確認）
