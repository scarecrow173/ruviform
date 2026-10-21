---
id: "IMPL-034JCy68I3essEUwu3PWnn"
type: "impl"
status: "completed"
title: "PoC メッセージ境界計測"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/28"
  changes:
    added:
      - type: "file"
        path: "poc/ui/src/exp/ipc.rs"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/ipcbench.ts"
      - type: "file"
        path: "poc/ui/frontend/ipc/package.json"
      - type: "file"
        path: "poc/ui/frontend/ipc/tsconfig.json"
      - type: "file"
        path: "poc/ui/frontend/ipc/vite.config.ts"
      - type: "file"
        path: "poc/ui/frontend/ipc/index.html"
      - type: "file"
        path: "poc/ui/frontend/ipc/src/main.ts"
      - type: "file"
        path: "docs/impl/exp/ac4-ipc-json-rtt-pull-evaluate-script-push.jsonl"
    modified:
      - type: "file"
        path: "poc/ui/src/exp/mod.rs"
      - type: "file"
        path: "poc/ui/src/exp/host.rs"
      - type: "file"
        path: "poc/ui/src/exp/render.rs"
      - type: "file"
        path: "poc/ui/src/host.rs"
      - type: "file"
        path: "poc/ui/src/assets.rs"
      - type: "file"
        path: "poc/ui/src/args.rs"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/ipc.ts"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/index.ts"
      - type: "file"
        path: "poc/ui/frontend/package.json"
      - type: "file"
        path: "poc/ui/frontend/package-lock.json"
      - type: "file"
        path: "poc/ui/frontend/build.mjs"
      - type: "file"
        path: "poc/ui/README.md"
      - type: "file"
        path: "poc/ui/RESULTS.md"
      - type: "file"
        path: "docs/tasks/poc-message-boundary.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "poc/ui/out/ipc-*.jsonl"
  implements:
    - "../../tasks/poc-message-boundary.md"
  implemented-by: []
  depends-on:
    - "../../plans/issue-28-ui-poc.md"
    - "../../designs/issue-28-ui-poc.md"
    - "poc-ui.md"
    - "poc-render-backends.md"
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
      - "../exp/ac4-ipc-json-rtt-pull-evaluate-script-push.jsonl"
    rejected: []
---

# PoC メッセージ境界計測

## Summary

task `TASK-034JfH9TmzdaGlsimZsJiq`（plan `PLAN-034JfDEU8mhCDj4ldYFZUG` の T3）として、AC4「メッセージ境界計測」を実装・実測した。Rust ⇄ フロントエンド間の 3 経路 — JSON intent 往復（`window.ipc.postMessage` → `with_ipc_handler` → `evaluate_script` 返送）、カスタムプロトコル バイナリ pull（`fetch("poc://data/blob?bytes=N")` → `arrayBuffer()`）、`evaluate_script` push（バースト + tick 定速）— と併用フェーズを `poc://app/ipc/index.html`（新規 `frontend/ipc` workspace + bench-shared `ipcbench.ts`）で計測する `--experiment ipc [--duration 10]` を実装。`assets.rs` に `poc://data/blob` の合成 `Cow<'static,[u8]>` 応答を、`host.rs` に周期起床 `HostEvent::Tick` と高頻度評価向け `eval_quiet` を追加した。実機 2 連続実行とも `status:"passed"`・`done_signal` 自己終了で、全必須メトリックキーが `poc-metric.v1` に記録された。design Open Question（高頻度 `evaluate_script` の暗黙 coalesce/drop）への実測回答として「ドロップ 0・受信側バッチ化は明確に発生」を記録。全変更は未コミット。

## Context

- 実施タスク: [../../tasks/poc-message-boundary.md](../../tasks/poc-message-boundary.md)（`TASK-034JfH9TmzdaGlsimZsJiq`）
- 上流: [plan](../../plans/issue-28-ui-poc.md)（T3）・[design](../../designs/issue-28-ui-poc.md)（AC4 実験設計: 3 経路・ペイロード系列・メトリックキーの正本）・[spec](../../specs/issue-28-ui-poc.md)（AC4）・[test-spec](../../test-specs/issue-28-ui-poc.md)（記録要件・status/exit code 規約）
- 前提: [poc-ui.md](poc-ui.md)（T1 ハーネス。`with_ipc_handler`・`with_custom_protocol` の `poc://data/` 名前空間・metrics）・[poc-render-backends.md](poc-render-backends.md)（T2 パターン: 状態機械は Rust 所有・done シグナル集約・EventLoopProxy 起床）
- 前提制約: `unsafe` 不使用・外部型を `host.rs`/`assets.rs`/`check.rs` に封じる・ルート `Cargo.toml`/`Cargo.lock`/`crates/`/`mise.toml` 無変更・コミットしない・実験は同時実行しない（タイミング汚染防止）

## Implementation

### Changes

- **`src/assets.rs` — `poc://data/blob?bytes=N` 実装**: T1 で予約だった `serve_data` を実装。`OnceLock<Vec<u8>>` で 32MiB 合成バッファを初回要求時に遅延初期化し、先頭 `bytes` バイトを `Cow::Borrowed` で返す（要求毎の確保/充填を避け、計測対象をプロトコル転送そのものに絞る）。`Access-Control-Allow-Origin: *` + `Cache-Control: no-store` を付与（`poc://app` → `poc://data` はオリジン横断のため fetch 受理に必要）。`bytes` 欠落・非数値・範囲外（1..=32MiB）は 400。
- **`src/host.rs` — `tick` + `eval_quiet`**: `HostConfig.tick: Option<Duration>` を追加し、`WaitUntil(min(next_tick, deadline))` で周期起床 `HostEvent::Tick` を配信。`ResumeTimeReached` は期限を区別しないため `now >= deadline` で Timeout/Tick を振り分け、遅延分は追い越しで飛ばす（過去分連続発火によるループ占有を防止）。`eval_quiet` は `eval` と同じ `evaluate_script` 呼出だが stderr ログを出さない — 高頻度経路（定速 push・RTT echo 返送）で `eprintln!` のコンソール出力コスト自体が計測を汚染するため。
- **`src/exp/ipc.rs` — AC4 実験本体**: `poc://app/ipc/index.html` を開き、状態機械は Rust 所有（`ready`/PageLoadFinished → `__poc.start(cfg)` で反復回数・サイズ系列を注入 → `done` で集約・`bench_error` で失敗・`alive` idle 検出で bounded start 再送、すべて render 実験と同じ規約）。`rtt_ping` 受信で `eval_quiet` `pong(seq)` 即返送、`ingest` は Rust 側到達窓（first→last Instant）で `ipc_ingest_msg_s` を計測、`push_begin` で 200 件バースト送信後 `Pace{next_seq:201,total:160}` を tick 駆動、`coexist_begin` で seq 1001 起点の 96 件定速。送り切りで `pushDone()` eval。既定 timeout 30s（`--duration` 上書き）、全フェーズ実測 ~4s。
- **`frontend/bench-shared/src/ipcbench.ts` — 測定ロジック（FW 非依存）**: `IpcBench` クラスがフェーズ逐次実行 — (1) `measureRtt`（サイズ別 pad 付き JSON を逐次 ping-pong、2s タイムアウトで missed 計数）(2) ingest 500 件フラッド (3) `measurePull`（`fetch` → `arrayBuffer` 完走・バイト数検証・`dataBase()` が `poc:`/`http://poc.data` を切替）(4) push 観測（`onPush` が seq・受信時刻・macrotask スタンプを記録、`waitPushDone` は pushDone/ストール/上限で解放）(5) coexist（定速 push 中に rtt ×8 + pull ×6 を並行）。`installApi` は `Record<string, unknown>` に拡張し `pong`/`push`/`pushDone` 受け口を追加。
- **`frontend/ipc/` — AC4 ページ（npm workspace）**: vanilla と同構成（Vite 7 + TS・`base:"./"`・薄い main.ts 配線: ready/alive/done/bench_error + `__poc` 受け口）。root `package.json` workspaces + `build.mjs` `VARIANTS` に `ipc` を追加、`package-lock.json` を `npm install` で再生成。
- **`src/exp/mod.rs`**: `ipc` をレジストリ登録・`known_names()` 追加。`src/args.rs` usage 追記。`exp/host.rs`/`exp/render.rs` は `HostEvent::Tick` 腕を noop 追加 + `tick: None`（列挙網羅性）。
- **`poc/ui/RESULTS.md`**: AC4 行・経路別数値表・所見（RTT/ingest/pull/push/coalesce/併用）・再現コマンドを追記。`poc/ui/README.md` も workspaces・構成・既知挙動メモ（初回 pull コールドパス・coalesce・tick 粒度）を更新。

### Decisions During Implementation

- **定速 push はホスト側周期起床 `HostEvent::Tick` で駆動**: `evaluate_script` の定速送信に汎用の「実験駆動 tick」として `host.rs` に実装（実験ローカルの workaround にしない・T4 でも再利用可能）。送信側でバッチ化しない設計（1 tick 1 件）にして、「間隔を空けた送信が受信側でまとめて届くか」という coalesce 観察を汚さない。
- **計測パラメータの単一源泉は Rust**: `__poc.start(cfg)` で rtt サイズ系列・反復回数・push 件数を注入（JS 側は欠落時のみ既定値で防御）。計測契約の正本を Rust 側に置く render と同じ分担。
- **ingest の msg/s は Rust 側到達窓で測定**: postMessage フラッドはキュー滞留を伴うため、実測値は「バックログ排水レート」となる（連続高レート配信の定常値ではない）。この性質は RESULTS.md で明示した。
- **`poc://data/blob` は遅延確保の借用応答**: 32MiB バッファを `OnceLock` 遅延初期化 + `Cow::Borrowed` 先頭スライス返却。要求毎の確保を除き転送のみを計測対象にする一方、初回リクエストのコールドパス（~330ms）が観測されたためこれも RESULTS.md に記録。
- **coalesce 検出は macrotask スタンプ**: JS 側 `setInterval(0)` タグで「同一イベントループターン着弾」を計数（`coalesced`/`max_batch`）。seq 欠番を `dropped` として計数 — 実測は dropped=0・coalesce 明確、という結果になった。
- **coexist フェーズを必須メトリックの後に配置**: rtt/ingest/pull/push の主計測を先に完走させてから併用フェーズを行い、部分失敗時にも必須キーが記録される構成とした。

## Related Experiments

### Adopted

- [../exp/ac4-ipc-json-rtt-pull-evaluate-script-push.jsonl](../exp/ac4-ipc-json-rtt-pull-evaluate-script-push.jsonl) — AC4 ipc 実験の実施ログ（実装方針・2 連続実行の数値・coalesce/tick 粒度/コールドパスの観察・併用結論）。結果は `RESULTS.md` AC4 行に集約済み

### Rejected

None.

## Validation

実施環境: Windows 11 x64 対話デスクトップ・Git Bash・mise `rust = "1.99.0"`・Node v26.10.0 / npm 11.19.1・WebView2 155.0.4283.62・60Hz 表示。

| Check | コマンド | 結果 |
| --- | --- | --- |
| frontend ビルド | `cd poc/ui/frontend && npm ci && npm run build` | workspaces（bench-shared + vanilla + ipc）tsc/vite 全 clean・`dist/`（index.html・vanilla/index.html・ipc/index.html・`.ui-poc-built`）生成 |
| PoC ビルド | `mise exec -- cargo build --manifest-path poc/ui/Cargo.toml` | clean・0 warnings |
| fmt | `cargo fmt --manifest-path poc/ui/Cargo.toml -- --check` | 差分なし |
| clippy | `cargo clippy --manifest-path poc/ui/Cargo.toml -- -D warnings` | clean・0 warnings |
| AC4 実行 | `ui-poc --experiment ipc --duration 10`（2 連続） | 両方 `status:"passed"`・`exit_reason:"done_signal"`・`timeout_hit:false`（out/ipc-1792564680.jsonl・out/ipc-1792564737.jsonl） |
| JSON RTT | 同上 | `ipc_rtt_ms`: 64B avg 0.57-1.06/p50 0.4/p99 ≤3.3・1KiB avg 0.46/p50 0.4/p99 1.2・16KiB avg 2.11/p50 1.6/p99 7.1・64KiB avg 4.33-4.42/p50 4.3/p99 ≤5.8（ms・各 32 往復・missed=0） |
| ingest | 同上 | `ipc_ingest_msg_s` ≒ 40,396-44,113 msg/s（500/500 到達・Rust 窓 ~11-12ms） |
| pull | 同上 | `protocol_pull_MBps`: 64KiB 2.86-2.95・1MiB 94.3-115.1・16MiB 88.4-146.1（64KiB 初回 ~330ms コールドパス・2 回目以降 p50 ~2ms） |
| push | 同上 | `push_msg_s` 192.6-218.2 msg/s・送信 456 件で dropped=0・`coalesced` 231-276/360・`max_batch` 105-109・interval 分布 ~0ms と ~15.6ms に二峰化 |
| tick 粒度 | 同上 | 名目 4ms → `tick_avg_ms` ~11.1-11.2ms（Windows タイマ分解能支配・paced 実効 ~90 msg/s） |
| 併用 | 同上 | 定速 push 中 rtt avg 0.71-0.94ms・pull 64KiB avg 3.3ms/1MiB avg 10ms・coexist push 96/96 受信・dropped=0 |
| 隔離 | `git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml` | 無出力（変更は `poc/`・`docs/` のみ） |
| ルート検証 | `mise run check` / `mise run lint` / `mise run test` | 全 PASS |
| exp 監査 | `node .agents/skills/impl-doc/scripts/audit_experiment_log.js --json` | findings 0 |
| IR 監査 | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | findings 0 |

## Risks

- **ingest msg/s は排水レート**: postMessage フラッドは WebView2 側でキューイングされ、Rust 側は滞留分を ~22µs/msg で消化する。連続高レート配信時の「配送定常レート」は本計測では分離できない（定速配信側の計測が必要なら JS 側に送信間隔を設ける別設計が要る）
- **`ipc_rtt_ms` は echo 返送込みの往復値**: JS→Rust→JS の往復を含むため、単方向 intent 配送の絶対遅延はこの半分以下と見積もれるが、分解計測はしていない
- **pull 初回コールドパスは環境依存を含みうる**: ~330ms の内訳（遅延バッファ初期化 vs カスタムスキーム経路ウォームアップ）は未分解。2 回目以降は安定するため実用上の影響は初回 1 要求のみ
- **push coalesce のタグ検出は近似**: `setInterval(0)` スタンプはクランプ粒度（~4ms）で、「同一タスク着弾」の厳密な区別ではなくバッチ化傾向の指標。`max_batch` 105-109 はバースト着弾を正確に捉えていると判断
- **tick 駆動の定速は ~11ms 分解能**: 本機 WaitUntil の実効粒度。より高速な定速 push が要る場合は 1 tick 複数送信へ設計変更する（送信側バッチ化を観測対象に混入させるトレードオフあり）

## Follow-ups

- T4（`--experiment surface`）: `HostConfig.tick` の周期起床・`eval_quiet` はそのまま再利用可能。`build_as_child` bounds 追従問題は T1/T2 の引継ぎ事項どおり検証論点
- T5（`--experiment frontend`/`dist`）: `frontend/ipc` と同じ手順で `solid` workspace を追加（`build.mjs` `VARIANTS` に列追加のみで dist/<variant>/ が増える構造は検証済み）
- 採否 ADR 連携: AC4 行の「evaluate_script はドロップしないが coalesce される」「高頻度データは pull 型 + seq 付き差分 push（latest-wins）+ 低レート JSON intent の棲み分け」をメッセージ境界条項の根拠として cite する
