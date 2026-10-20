---
id: "TASK-034JfGDrzj4yLZMu9A4qHS"
type: "task"
status: "todo"
title: "PoC 描画バックエンド計測（canvas2d/webgl2/webgpu フレーム統計）"
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
    - "poc-frontend-and-dist.md"
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

# PoC 描画バックエンド計測（canvas2d/webgl2/webgpu フレーム統計）

## Work

plan `PLAN-034JfDEU8mhCDj4ldYFZUG` の T2。design の AC2 実験設計を正本として、同一の高頻度更新ワークロードを Canvas2D / WebGL2 / WebGPU の各 WebView 内バックエンドで実行し、フレーム統計を `poc-metric.v1` で記録する。

- [ ] `poc/ui/frontend/` に npm workspaces 骨格（`package.json` + `package-lock.json` コミット）を構築する
- [ ] `frontend/bench-shared/` に FW 非依存の共通部品を実装: ワークロード生成（スクロール波形・ピーク列 約 2,000 点/フレーム + レベルメータ群・目安 60fps）・rAF フレーム統計（`frame_time_ms`・dropped = 2 vsync 間隔超過・`achieved_fps`）・`window.ipc.postMessage` ブリッジ・done シグナル送信
- [ ] `frontend/vanilla/` に Vite + TS の参照実装 UI（FW ランタイムゼロの下限）を構築し、`npm run build` で `dist/` を生成、`poc://app/` 埋め込み配信に載せる
- [ ] `src/exp/render.rs` に `--experiment render --backend canvas2d|webgl2|webgpu [--frames 600]` を実装: 既定 600 フレームまたは timeout で自己終了し、JS 側統計を done メッセージ経由で `metrics.rs` が集約（`frame_time_ms` `avg/p50/p95/p99`・`dropped`・`achieved_fps`）
- [ ] WebGPU は `navigator.gpu` + adapter 要求の機能検出を実装し、失敗時は WebGL2 へフォールバックして `status:"passed"`・`metrics.fallback_to` に理由を記録する
- [ ] 実機で 3 バックエンドを実行し、バックエンド別数値行を `poc/ui/RESULTS.md`（AC2 行）と `docs/impl/exp/` ログに記録

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/issue-28-ui-poc.md](../plans/issue-28-ui-poc.md) — `PLAN-034JfDEU8mhCDj4ldYFZUG`（approved）。T2 の正本
- [../designs/issue-28-ui-poc.md](../designs/issue-28-ui-poc.md) — AC2 render 実験（ワークロード・メトリック・フォールバック）の正本
- [../specs/issue-28-ui-poc.md](../specs/issue-28-ui-poc.md) — AC2 の正本
- [../test-specs/issue-28-ui-poc.md](../test-specs/issue-28-ui-poc.md) — フレーム統計の記録要件（≥600 フレーム・`fallback_to`・記録要件は SLA でない）判定規則

## Dependencies

- 前提: `poc-ui-harness-host.md`（host・metrics・protocol・`poc://app/` 配信・`poc-metric.v1` が前提）
- 後続をブロック: `poc-frontend-and-dist.md`（本タスクの bench UI を Solid へ移植する前提）、`poc-adr-finalize.md`（AC2 エビデンス供給）
- フロントエンド依存は `poc/ui/frontend/` 配下に限定（Vite・TypeScript）。`npm ci && npm run build` が標準前提で、未ビルドの実験は `status:"skipped"`（reason: frontend not built）

## Affected Files

- `poc/ui/frontend/package.json`・`poc/ui/frontend/package-lock.json`・`poc/ui/frontend/bench-shared/`・`poc/ui/frontend/vanilla/` — 新規
- `poc/ui/src/exp/render.rs`・`poc/ui/src/exp/mod.rs`（レジストリ追加）— 新規/更新
- `poc/ui/src/assets.rs` — `frontend/dist` を埋め込み配信する配線（必要に応じて）
- `poc/ui/RESULTS.md` — AC2 行追加
- `docs/impl/exp/` — 実施ログを追記

## Done When

- [ ] `--experiment render --backend canvas2d|webgl2|webgpu --frames 600` が自己終了し、最終 stdout 行が `poc-metric.v1` で `frame_time_ms` の `avg/p50/p95/p99`・`dropped`・`achieved_fps` を含む
- [ ] WebGPU 機能検出失敗時に WebGL2 フォールバック + `metrics.fallback_to` 記録が確認できる
- [ ] 3 バックエンドの数値行が `RESULTS.md` に環境・再現コマンド付きで存在する
- [ ] ルートワークスペースに差分がない

## Verification

- [ ] `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment render --backend canvas2d --frames 600` — `poc-metric.v1` 最終行・`status` フィールド・exit code 規約どおり
- [ ] `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment render --backend webgl2 --frames 600` — 同上
- [ ] `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment render --backend webgpu --frames 600` — WebGPU 計測または `fallback_to` 付き WebGL2 結果
- [ ] `cd poc/ui/frontend && npm ci && npm run build` — frontend dist が生成される（前提手順）
- [ ] Manual: `poc/ui/RESULTS.md` にバックエンド別の数値行・再現コマンドが存在する（補助確認）
