---
id: "TASK-034JfGCUwtk0lzJvJPr36B"
type: "task"
status: "todo"
title: "PoC UI ハーネスホスト（独立ワークスペース・wry+tao・計測契約）"
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
  blocks:
    - "poc-render-backends.md"
    - "poc-message-boundary.md"
    - "poc-native-surface.md"
    - "poc-frontend-and-dist.md"
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

# PoC UI ハーネスホスト（独立ワークスペース・wry+tao・計測契約）

## Work

plan `PLAN-034JfDEU8mhCDj4ldYFZUG` の T1。design `DESIGN-034JcpvK9cZEfkeEbBLBoz` のハーネス構成を正本として、全実験の前提となる `poc/ui/` 独立ワークスペースと `ui-poc` バイナリ骨格・計測契約を構築し、`--experiment host`（AC1）と隔離（AC8）を検証する。

- [ ] `poc/ui/` を `[workspace]` + `[package]`（`ui-poc` 単一 bin）の独立 cargo ワークスペースとして作成（ルート `Cargo.toml` の `members` は明示リストで glob なしのため `exclude` 不要。ルート `Cargo.toml`/`Cargo.lock`/`crates/`/`mise.toml` は無変更）
- [ ] `poc/ui/.gitignore`（`node_modules/`・`*/dist/`・`out/`）・`poc/ui/README.md`（前提 `npm ci && npm run build`・ビルド/実行/再現手順・wgpu optional feature 化の逃げ道注記）を作成
- [ ] `src/args.rs`: std-only 引数パース（`--experiment <name>`・`--frames N`・`--duration SEC`・`--backend`・`--mode`・`--variant`・`--out`・`--check`）
- [ ] `src/check.rs`: capability 検出（WebView2 レジストリ `pv` 値・wgpu アダプタ・OS/toolchain）と `--check` の JSON 出力（GUI を作らず exit 0）
- [ ] `src/main.rs`: dispatch と exit code 規約（passed/skipped → 0、failed → 1、CLI/内部エラー → 2）
- [ ] `src/host.rs`: tao `Window` + wry `WebView`（`build_as_child` または `build` + `set_bounds`）・自己終了タイマ（N フレーム/timeout）・done シグナル受信。外部型（wry/tao/winreg）はこの層に封じ、`exp/*`・`metrics.rs` は内部 host I/F にのみ依存させる
- [ ] `src/assets.rs`: `rust-embed` + `with_custom_protocol` で `poc://app/` 埋め込みアセット配信（debug=FS・release=埋め込み）と `poc://data/` の名前空間分離
- [ ] `src/metrics.rs`: `poc-metric.v1` スキーマ（`{schema, experiment, ts, env:{os,arch,webview2_version,wgpu_adapter,rustc}, params, metrics, status, reason?}`）・パーセンタイル集計・最終 stdout 行出力 + `out/*.jsonl` 追記
- [ ] `src/exp/mod.rs`: `Experiment` trait（setup/run/teardown）と実験レジストリ
- [ ] `src/exp/` に `--experiment host`（AC1）を実装: `poc://app/index.html` の埋め込みロード・`window.ipc.postMessage` JSON intent → `with_ipc_handler` → `evaluate_script` echo・`set_bounds` リサイズ追従・フォーカス遷移ログ。メトリック `protocol_load`/`ipc_echo`/`resize_follow`/`focus_transition`/`webview2_version` を記録
- [ ] `poc/ui/RESULTS.md` スケルトン（AC → 実験 → エビデンス行対応表・環境欄・再現コマンド欄・「文書評価」ラベル欄）を作成
- [ ] 実機で `--experiment host` を実行し結果を `RESULTS.md` AC1 行と `docs/impl/exp/` ログに記録（ヘッドレス環境では `status:"skipped"` 経路の確認に留める）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/issue-28-ui-poc.md](../plans/issue-28-ui-poc.md) — `PLAN-034JfDEU8mhCDj4ldYFZUG`（approved）。T1 の正本
- [../designs/issue-28-ui-poc.md](../designs/issue-28-ui-poc.md) — `DESIGN-034JcpvK9cZEfkeEbBLBoz`（approved）。ハーネス構成・`poc-metric.v1`・`--check`/exit code/自己終了契約の正本
- [../specs/issue-28-ui-poc.md](../specs/issue-28-ui-poc.md) — AC1（WebView ホスト）・AC8（隔離）の正本
- [../test-specs/issue-28-ui-poc.md](../test-specs/issue-28-ui-poc.md) — `status`/exit code・`--check`・記録要件・文書評価ラベルの判定規則

## Dependencies

- 前提: plan のみ（design は approved 済み）。本タスクは全実験タスク（T2–T5）の共通前提
- 後続をブロック: `poc-render-backends.md`・`poc-message-boundary.md`・`poc-native-surface.md`・`poc-frontend-and-dist.md`（host・metrics・protocol・check が前提）
- 依存クレートは `poc/ui` ワークスペース内に限定（wry/tao/rust-embed/winreg/serde_json 等）。`unsafe` は使わない（本タスクの windows-rs 直接呼出は不要）

## Affected Files

- `poc/ui/Cargo.toml`・`poc/ui/Cargo.lock`・`poc/ui/.gitignore`・`poc/ui/README.md`・`poc/ui/RESULTS.md` — 新規
- `poc/ui/src/{main,args,check,host,assets,metrics}.rs`・`poc/ui/src/exp/{mod,host}.rs` — 新規
- `poc/ui/frontend/`（最小骨格: `package.json` + `package-lock.json`・bench 用 index.html/TS の足場）— 新規（詳細は T2）
- `docs/impl/exp/` — 実施ログを追記
- ルート `Cargo.toml`/`Cargo.lock`/`crates/`/`mise.toml`/`.gitignore` — **変更不要・変更禁止**

## Done When

- [ ] `poc/ui` が独立ワークスペースとして `mise exec -- cargo build --manifest-path poc/ui/Cargo.toml` でビルドできる
- [ ] `--check` が GUI を作らず capability 検出 JSON を出して exit 0 する
- [ ] 実機で `--experiment host` が `status:"passed"`（または前提不足なら `status:"skipped"` + 非空 `reason`）で終了し、`poc-metric.v1` 最終行 JSON と `out/*.jsonl` が出る
- [ ] AC1 項目（埋め込みアセットロード・IPC echo・リサイズ・フォーカス）が `RESULTS.md` に記録される
- [ ] ルート `Cargo.toml`/`Cargo.lock`/`crates/`/`mise.toml` に差分がない（AC8）

## Verification

- [ ] `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --check` — capability 検出 JSON を出して exit 0（ヘッドレスでも動作）
- [ ] `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment host` — 最終 stdout 行が `poc-metric.v1` で `status` が `passed`/`skipped`/`failed` のいずれか（実機では `passed`・skipped 時は非空 `reason`）
- [ ] `git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml` — 出力が空（隔離確認）
- [ ] `mise run check` && `mise run lint` && `mise run test` — ルート検証が PoC 前と同一に成功
- [ ] Manual: `poc/ui/RESULTS.md` に AC → 実験 → エビデンス対応表のスケルトンと AC1 行が存在する（補助確認）
