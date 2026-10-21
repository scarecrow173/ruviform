---
id: "IMPL-034JBSlPrPoY8WlSeBMTkX"
type: "impl"
status: "completed"
title: "PoC 描画バックエンド計測"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/28"
  changes:
    added:
      - type: "file"
        path: "poc/ui/src/exp/render.rs"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/package.json"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/tsconfig.json"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/index.ts"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/workload.ts"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/stats.ts"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/meters.ts"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/ipc.ts"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/runner.ts"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/renderers/types.ts"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/renderers/index.ts"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/renderers/canvas2d.ts"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/renderers/webgl2.ts"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/renderers/webgpu.ts"
      - type: "file"
        path: "poc/ui/frontend/vanilla/package.json"
      - type: "file"
        path: "poc/ui/frontend/vanilla/tsconfig.json"
      - type: "file"
        path: "poc/ui/frontend/vanilla/vite.config.ts"
      - type: "file"
        path: "poc/ui/frontend/vanilla/index.html"
      - type: "file"
        path: "poc/ui/frontend/vanilla/src/main.ts"
      - type: "file"
        path: "docs/impl/exp/ac2-render-canvas2d-webgl2-webgpu.jsonl"
    modified:
      - type: "file"
        path: "poc/ui/frontend/package.json"
      - type: "file"
        path: "poc/ui/frontend/package-lock.json"
      - type: "file"
        path: "poc/ui/frontend/build.mjs"
      - type: "file"
        path: "poc/ui/.gitignore"
      - type: "file"
        path: "poc/ui/README.md"
      - type: "file"
        path: "poc/ui/RESULTS.md"
      - type: "file"
        path: "poc/ui/src/args.rs"
      - type: "file"
        path: "poc/ui/src/assets.rs"
      - type: "file"
        path: "poc/ui/src/host.rs"
      - type: "file"
        path: "poc/ui/src/exp/mod.rs"
      - type: "file"
        path: "poc/ui/src/exp/host.rs"
      - type: "file"
        path: "poc/ui/src/metrics.rs"
      - type: "file"
        path: "docs/tasks/poc-render-backends.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "poc/ui/out/render-*.jsonl"
  implements:
    - "../../tasks/poc-render-backends.md"
  implemented-by: []
  depends-on:
    - "../../plans/issue-28-ui-poc.md"
    - "../../designs/issue-28-ui-poc.md"
    - "poc-ui.md"
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
      - "../exp/ac2-render-canvas2d-webgl2-webgpu.jsonl"
    rejected: []
---

# PoC 描画バックエンド計測

## Summary

task `TASK-034JfGDrzj4yLZMu9A4qHS`（plan `PLAN-034JfDEU8mhCDj4ldYFZUG` の T2）として、AC2「WebView 内描画バックエンド計測」を実装・実測した。`frontend/` を npm workspaces（`bench-shared` + `vanilla`）へ拡張し、FW 非依存のワークロード（ピーク列 ~2,000 点/フレーム + 8ch メータ）・rAF フレーム統計・3 バックエンドレンダラ・IPC ブリッジを整備。`--experiment render --backend canvas2d|webgl2|webgpu [--frames 600]` が `poc://app/vanilla/index.html` のベンチ UI を駆動し、done メッセージ経由の生サンプルを `metrics::summarize` が集約する。実機で 3 バックエンド全て `status:"passed"`（webgpu は `navigator.gpu` 不在により WebGL2 フォールバック + `fallback_to` 記録）。途中で発見した「WebView2 IPC が tao `WaitUntil` を起床させない」問題と「UDF ロック競合」も `host.rs`（adapter 層）で修正済み。全変更は未コミット。

## Context

- 実施タスク: [../../tasks/poc-render-backends.md](../../tasks/poc-render-backends.md)（`TASK-034JfGDrzj4yLZMu9A4qHS`）
- 上流: [plan](../../plans/issue-28-ui-poc.md)（T2）・[design](../../designs/issue-28-ui-poc.md)（AC2 実験設計: ワークロード・計測契約・フォールバックの正本）・[spec](../../specs/issue-28-ui-poc.md)（AC2）・[test-spec](../../test-specs/issue-28-ui-poc.md)（≥600 フレーム・`fallback_to`・記録要件は SLA でない）
- 前提: [poc-ui.md](poc-ui.md)（T1 ハーネス。`host.rs`/`assets.rs`/`metrics.rs`/`exp` レジストリを拡張して利用）
- 前提制約: `unsafe` 不使用・外部型を `host.rs`/`assets.rs`/`check.rs` に封じる・ルート `Cargo.toml`/`Cargo.lock`/`crates/`/`mise.toml` 無変更・コミットしない・実験は同時実行しない（タイミング汚染防止）

## Implementation

### Changes

- **`frontend/` npm workspaces 化**: ルート `package.json` に `workspaces:["bench-shared","vanilla"]`。`npm run build` = 全 workspaces ビルド（`--if-present`）+ `build.mjs` による `dist/` 集約（`src/` の host ページ + `vanilla/dist` → `dist/vanilla/` + `.ui-poc-built` マーカー。variant dist 欠落は exit 1 で失敗させる）。`package-lock.json` を再生成して `npm ci` 契約を維持
- **`frontend/bench-shared/`**: FW 非依存の共通部品。`workload.ts`（mulberry32 種固定の決定論的生成: 1,000 列×min/max ≒ 2,000 ピーク + 8ch attack/decay メータ）・`stats.ts`（rAF 差分列 `frame_time_ms`・作業時間 `work_time_ms`・median 推定 vsync による dropped/achieved_fps）・`meters.ts`（DOM transform scaleY 更新の参照メータ群）・`ipc.ts`（`window.ipc.postMessage` ブリッジ + `__poc` 受け口）・`renderers/`（canvas2d パス fill・webgl2 TRIANGLE_STRIP bufferData・webgpu writeBuffer+draw の 3 実装と `createRenderer` 機能検出+フォールバック）・`runner.ts`（warmup 12 + N フレーム rAF ループ → done メッセージ送信）
- **`frontend/vanilla/`**: Vite 7 + TypeScript（`base:"./"` で `poc://app/vanilla/` 相対解決）。`main.ts` は `ready` 通知 → `__poc.start({backend,frames})` 受付 → `done`/`bench_error`/`alive` ハートビート（1s）を IPC 報告する薄い配線のみ
- **`src/exp/render.rs`**: AC2 実験。`--backend` 検証（欠落/不正は `failed` + reason）→ `poc://app/vanilla/index.html` を開き、`ready`/PageLoadFinished で `__poc.start` を eval 送信（`alive` が `state:"idle"` を報告したら最大 3 回再送）→ `done.stats` 受領で `metrics::summarize` 集約（`frame_time_ms`/`work_time_ms` の avg/p50/p95/p99 + `dropped`/`achieved_fps`/`vsync_ms`/`fallback_to`/`capabilities`）。既定 timeout は `frames/15+20`（最低 30s）秒、`--duration` で上書き
- **`src/exp/mod.rs`**: `render` をレジストリ登録。全実験共通の前提ゲーティング（headless/display/WebView2/frontend_built）を `check_gui_prereqs` として集約し `exp/host.rs` も移行（T3–T5 の重複防止）
- **`src/assets.rs`**: `app_asset_exists(rel)` 追加 — `.ui-poc-built` マーカーがあっても `vanilla/index.html` が欠ける部分ビルドを `skipped` で検出するため
- **`src/host.rs`（T2 中に発見した根因修正）**: (1) WebView2 の IPC/ページロード callback は `WaitUntil` スリープを起床させず、ウィンドウイベント途絶区間の IPC は deadline まで滞留する問題へ `EventLoopProxy`（`with_user_event`）で明示起床。(2) WebView2 UDF シングルトンロックの競合（前回プロセスの子残留で初期化ブロック）を避けるため既定 data_dir を `ui-poc-webview2-<pid>` のプロセス固有パスへ変更
- **`src/metrics.rs`**: `summarize`/`percentile` の `#[allow(dead_code)]` を撤去（render 実験が接続）
- **`src/args.rs`**: usage の実験一覧に `render` を追加
- **`poc/ui/.gitignore`**: `*/dist/` → `dist/`（`frontend/vanilla/dist` 等のネスト dist も ignore 対象にするため）
- **`poc/ui/RESULTS.md`**: AC2 行・バックエンド別数値表・所見（navigator.gpu 非公開・vsync 支配の解釈・IPC 非起床修正・UDF 競合）・再現コマンドを追記
- **`poc/ui/README.md`**: workspaces 構成・render 実行手順・新規の既知挙動メモを更新

### Decisions During Implementation

- **dist の集約構成**: `dist/index.html` は T1 host ページのまま維持し、ベンチ UI は `dist/<variant>/index.html` に分離（render は `poc://app/vanilla/index.html` を開く）。host 実験の URL・JS エージェントを変えずに AC1 証跡を保全しつつ、T5 で `solid/` が同じ構造で追加できる
- **状態機械は Rust 所有・起動は eval 駆動**: URL クエリではなく `ready` IPC → `__poc.start(cfg)` eval でバックエンド/フレーム数を注入（T1 と同思想。`alive` ハートビートの `state:"idle"` 検出で start 再送も bounded に実装）
- **統計の責務分担**: `frame_time_ms`/`work_time_ms` の生サンプル列（600+601 要素）を done メッセージで送り percentile 集約は `metrics.rs` 側に置く（design の集約契約どおり）。vsync 推定が必要な `dropped`/`achieved_fps`/`vsync_ms` は JS 側で算出して送る
- **IPC 非起床問題の修正場所**: `host.rs`（adapter 層）で `EventLoopProxy` を注入。`exp/*` は無変更で効く（個別実験での workaround にしない）。T1 host 実験の「timeout_hit:true 併記」も同根因であり、本修正で `metrics_complete`・RTT 8.1ms・resize retries:0 に改善したことを再実行で確認
- **`--backend` 欠落/不正は `failed`（exit 1）**: CLI 引数エラー（exit 2）ではなく実験失敗として記録する方針（trait に args 検証フックを増やさない最小整合）
- **WebGPU のフォールバック**: `navigator.gpu` 不在・adapter 要求失敗・device 要求失敗・canvas context 取得失敗をすべて検出して WebGL2 へ（理由文字列を `metrics.fallback_to` に記録）。WebGL2 自体の不在は `bench_error` → `failed` とする

## Related Experiments

### Adopted

- [../exp/ac2-render-canvas2d-webgl2-webgpu.jsonl](../exp/ac2-render-canvas2d-webgl2-webgpu.jsonl) — AC2 render 実験の実施ログ（IPC 非起床の発見→proxy 修正・UDF 競合・navigator.gpu 非公開・3 バックエンド数値・host 回帰改善）。結果は `RESULTS.md` AC2 行に集約済み

### Rejected

None.

## Validation

実施環境: Windows 11 x64 対話デスクトップ・Git Bash・mise `rust = "1.99.0"`・Node v26.10.0 / npm 11.19.1・WebView2 155.0.4283.62・60Hz 表示。

| Check | コマンド | 結果 |
| --- | --- | --- |
| frontend ビルド | `cd poc/ui/frontend && npm ci && npm run build` | workspaces ビルド + `dist/`（index.html・vanilla/index.html・assets・`.ui-poc-built`）生成。esbuild postinstall は npm 11 allowScripts でブロックされるが optional dep の `@esbuild/win32-x64` が機能 |
| PoC ビルド | `mise exec -- cargo build --manifest-path poc/ui/Cargo.toml` | clean・0 warnings |
| fmt | `cargo fmt --manifest-path poc/ui/Cargo.toml -- --check` | 差分なし |
| clippy | `cargo clippy --manifest-path poc/ui/Cargo.toml -- -D warnings` | clean・0 warnings |
| AC2 canvas2d | `ui-poc --experiment render --backend canvas2d --frames 600` | `status:"passed"`・done_signal・600 frames・avg 16.667/p50 16.70/p95 16.80/p99 16.80・dropped 0・achieved 60.00（out/render-1792563717.jsonl） |
| AC2 webgl2 | `ui-poc --experiment render --backend webgl2 --frames 600` | `passed`・avg 16.666/p99 16.80・dropped 0・60.00（out/render-1792563728.jsonl） |
| AC2 webgpu | `ui-poc --experiment render --backend webgpu --frames 600` | `passed`・`fallback_to={to:"webgl2",reason:"navigator.gpu unavailable"}`・backend_used=webgl2・avg 16.666・dropped 0・60.00（out/render-1792563740.jsonl） |
| WebGPU 有効化試行 | `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--enable-unsafe-webgpu` 等 2 試行 | `navigator_gpu=false` のまま（有効化不可を記録） |
| skipped 契約 | `UI_POC_HEADLESS=1 ui-poc --experiment render --backend webgl2` | `status:"skipped"` + `reason:"UI_POC_HEADLESS=1"`・exit 0 |
| failed 契約 | `--experiment render`（backend 欠落）/ `--backend foo` | `status:"failed"` + reason・exit 1 |
| host 回帰 | `ui-poc --experiment host --duration 15` | `status:"passed"`・`metrics_complete`・`timeout_hit:false`・ipc_echo RTT 8.1ms・resize retries:0（T1 より改善） |
| 隔離 | `git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml` | 無出力 |
| ルート検証 | `mise run check` / `mise run lint` / `mise run test` | 全 PASS |
| exp 監査 | `node .agents/skills/impl-doc/scripts/audit_experiment_log.js --json` | findings 0 |
| IR 監査 | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | findings 0 |

## Risks

- **WebGPU は実計測不能**: WebView2 155.0.4283.62 では `navigator.gpu` が公開されず、browser args 2 試行でも有効化不可。WebGPU の実フレーム統計は未取得であり、`backend_used:"webgpu"` の計測値は存在しない。他環境（ランタイム更新・他 OS）では取れる可能性がある — `capabilities` フィールドがその判定証跡
- **`frame_time_ms` は vsync 支配**: 60Hz 環境では全バックエンドが vsync ロックとなり、バックエンド間の実力差は `frame_time_ms` には現れない。`work_time_ms`（avg 0.4-0.5ms）は主証拠だが本ワークロードでは差が小さい。より重いワークロード（点密度・メータ数倍増）や高リフレッシュ環境での計測は後続課題
- **vsync 推定の前提**: `dropped` は median 推定の vsync を使うため、表示レート非同期・大幅可変の環境では判定が揺れうる（本機 60Hz では stable に観測）
- **temp UDF の蓄積**: `ui-poc-webview2-<pid>` は実行ごとに残る（OS temp 清掃に委ねる。PoC では実害なし）

## Follow-ups

- T3（`--experiment ipc`）: `poc://data/` バイナリ応答の実装と 3 経路計測。T2 で修正した proxy 起床経路が RTT/push 計測の即時配送にも効く（T1 の 255-940ms RTT は非起床の影響を多分に含んでいた可能性があり、AC4 計測値は本修正後の値で評価すること）
- T5（`--experiment frontend`）: `frontend/solid/` を workspaces に追加し `build.mjs` の `VARIANTS` に列追加。AC5 の variant 比較は `dist/<variant>/` 単位で測れる構造
- WebGPU-in-WebView2: SDK/opt-in API 経路（`ICoreWebView2` 系の機能許可等）の有無を discovery へフィードバック。実計測は runtime 更新時に再試行
- host 実験メトリックの再取得: proxy 修正で ipc_echo/resize_follow の証跡値が改善したため、AC1 行の更新再実行は任意（T1 記録は当時コードの実測として有効）
