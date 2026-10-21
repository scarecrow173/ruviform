---
id: "IMPL-034JE49N0s52vniJdJrTpU"
type: "impl"
status: "completed"
title: "PoC フロントエンド比較と配布検証"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/28"
  changes:
    added:
      - type: "file"
        path: "poc/ui/src/exp/frontend.rs"
      - type: "file"
        path: "poc/ui/src/exp/dist.rs"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/mutations.ts"
      - type: "file"
        path: "poc/ui/frontend/solid/package.json"
      - type: "file"
        path: "poc/ui/frontend/solid/tsconfig.json"
      - type: "file"
        path: "poc/ui/frontend/solid/vite.config.ts"
      - type: "file"
        path: "poc/ui/frontend/solid/index.html"
      - type: "file"
        path: "poc/ui/frontend/solid/src/main.tsx"
      - type: "file"
        path: "poc/ui/frontend/solid/src/meters.tsx"
      - type: "file"
        path: "docs/impl/exp/ac5-frontend-compare.jsonl"
      - type: "file"
        path: "docs/impl/exp/ac6-dist-single-binary.jsonl"
    modified:
      - type: "file"
        path: "poc/ui/Cargo.toml"
      - type: "file"
        path: "poc/ui/Cargo.lock"
      - type: "file"
        path: "poc/ui/src/exp/mod.rs"
      - type: "file"
        path: "poc/ui/src/args.rs"
      - type: "file"
        path: "poc/ui/src/assets.rs"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/index.ts"
      - type: "file"
        path: "poc/ui/frontend/bench-shared/src/runner.ts"
      - type: "file"
        path: "poc/ui/frontend/vanilla/src/main.ts"
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
        path: "docs/tasks/poc-frontend-and-dist.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "poc/ui/out/frontend-*.jsonl"
      - type: "file"
        path: "poc/ui/out/dist-*.jsonl"
  implements:
    - "../../tasks/poc-frontend-and-dist.md"
  implemented-by: []
  depends-on:
    - "../../plans/issue-28-ui-poc.md"
    - "../../designs/issue-28-ui-poc.md"
    - "poc-ui.md"
    - "poc-render-backends.md"
    - "poc-native-surface.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../specs/issue-28-ui-poc.md"
    - "../../adr/ui-tech-selection-poc.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/issue-28-ui-poc.md"
  references:
    - "https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution"
    - "https://blogs.gnome.org/mcatanzaro/2025/04/28/webkitgtk-api-versions/"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "../exp/ac5-frontend-compare.jsonl"
      - "../exp/ac6-dist-single-binary.jsonl"
    rejected: []
---

# PoC フロントエンド比較と配布検証

## Summary

T5（AC5 フロントエンド比較・AC6 配布検証）を実装し、Windows 11 実機で検証完了。

- **AC5**: `frontend/solid/` を新設し（vite-plugin-solid 2.11.14 + solid-js 1.9.17・bench-shared 共有）、`--experiment frontend --variant vanilla|solid` で 2 候補比較を実測。Solid の fine-grained 更新は vanilla と完全に同型の部分適用粒度（mutations/meter 更新=1.0・childList churn ゼロ）を達成し、コスト差は dist raw +8.8KB（+78%）/ gzip-9 +3.5KB・build +32%・glue LOC +2.1 倍に集約された。Svelte 5 は YAGNI 延期（追加容易性を記録）。
- **AC6**: headless `--experiment dist`（GUI 非生成・前提 gating なし）で WebView2 検出（155.0.4283.62）・バイナリサイズ・埋め込み状態・bootstrapper 案内を記録。release 単一バイナリ（8,100,864B）が temp dir 単体で `--check` 全項目・全実験を完走（`embed_mode=embedded-release`）し、portable zip（3,039,116B）も解凍先で動作確認。macOS/Linux は文書評価行として整理し、Linux `libwebkit2gtk-4.1` の依存リスクを API バージョン表で定量化した。

## Context

- 実施タスク: [../../tasks/poc-frontend-and-dist.md](../../tasks/poc-frontend-and-dist.md)（`TASK-034JfHyxKSwvhiQ0sKqjXQ`、plan `PLAN-034JfDEU8mhCDj4ldYFZUG` の T5）
- 上流: [plan](../../plans/issue-28-ui-poc.md)（T5）・[design](../../designs/issue-28-ui-poc.md)（AC5 frontend 実験・AC6 dist 実験の正本）・[spec](../../specs/issue-28-ui-poc.md)（AC5・AC6）・[test-spec](../../test-specs/issue-28-ui-poc.md)（比較項目・文書評価ラベル・リスク定量化の判定規則）・[採否 ADR](../../adr/ui-tech-selection-poc.md)（「フロントエンド FW」「配布」採否条項の対象）
- 前提: [poc-ui.md](poc-ui.md)（T1 ハーネス: host/assets/check/metrics）・[poc-render-backends.md](poc-render-backends.md)（T2: bench-shared・vanilla ベンチ UI。Solid 移植の前提実装）・[poc-message-boundary.md](poc-message-boundary.md)（T3: ipc workspace 追加手順の先例）・[poc-native-surface.md](poc-native-surface.md)（T4: surface 実験の登録先例）
- 前提制約: `poc/ui` は独立ワークスペース（ルート `Cargo.toml`/`Cargo.lock`/`crates/`/`mise.toml` 無変更）。npm 依存は `poc/ui/frontend/`・Rust 依存は `poc/ui/Cargo.toml` に限定。`unsafe` 不使用。コミットしない。

## Implementation

### Changes

- **Solid フロントエンド新設**: `frontend/solid/`（package.json・tsconfig・vite.config.ts・index.html・`src/main.tsx`・`src/meters.tsx`）。`Meters` コンポーネントで 8ch 独立 `createSignal` → `.meter-fill` の `style.transform` 直接適用という fine-grained 更新経路を構築し、bench-shared の `BenchRunner` に meter host を注入する形で共有ワークロードを再利用。root `frontend/package.json` の workspaces に `solid` を追加し `package-lock.json` を `npm install` で再生成。
- **bench-shared 計装拡張**: `mutations.ts` 新設（`MutationMeter` — MutationObserver で attributes/childList/characterData/added/removed を集計）。`runner.ts` に `BenchElements.metersHost`/`observeEl` を追加し meter driver 選択（`injected` / `createMeterHost`）を `done.stats.meters_driver` として報告。`vanilla/src/main.ts` に MutationMeter 配線を追加（Solid 側と同一計装経路）。
- **ビルド orchestrator**: `build.mjs` を VARIANTS 駆動のワークスペース別ビルドに拡張し、`dist/<variant>/build-meta.json`（variant・build_ms・FS 側 total_bytes・file 内訳）を刻印。
- **`src/exp/frontend.rs`**（AC5）: `--variant vanilla|solid` 対応（欠落/不正値→failed、variant dist 欠落→skipped。`poc://app/<variant>/index.html` で状態機械ベンチを実行し、rust-embed アセット列挙から dist raw/gzip-9（flate2/miniz_oxide・`gzip9_impl` 明記）を算出、build-meta.json で `meta_total_bytes_match` 自己検証、`frontend/<variant>/{index.html,vite.config.ts,src/**}` の glue LOC を計数、DOM mutation 集計を metrics へ記録。
- **`src/exp/dist.rs`**（AC6）: `check_gui_prereqs` を呼ばない headless 実験。WebView2 レジストリ検出（check.rs 経路）・`current_exe()` サイズ・`embedded_ok`/`embed_mode`（fs-debug vs embedded-release）・`frontend_built`・アセット数・bootstrapper 案内（Evergreen fwlink + winget）を記録。
- **レジストリ/CLI**: `exp/mod.rs` に `frontend`/`dist` の module・lookup・known_names 登録、`args.rs` USAGE の実験一覧更新。`poc/ui/Cargo.toml` に `flate2 = "1"` 追加（poc/ui WS 限定・miniz_oxide バックエンド）。
- **ドキュメント**: `poc/ui/RESULTS.md` に AC5 比較行（2 候補全項目）・AC6 行（debug/release/zip + macOS/Linux 文書評価）・Linux リスク定量化表・再現コマンドを追加。`poc/ui/README.md` の実験一覧・構成図を T5 完了に合わせ更新。

### Decisions During Implementation

- **dist 実験の headless 化**: design の「配布前提の検出は GUI と分離する」意図に従い、`check_gui_prereqs` を使わず GUI 非生成で検出値のみ記録する契約とした。`skipped` は前提不足専用で、dist は常に記録して `passed`（WebView2 未検出時も `bootstrapper_required=true` + 案内出力で表現）。
- **gzip-9 の実装**: design 指定どおり Rust 側で計算。`flate2`/`miniz_oxide` の best 圧縮をファイル毎に適用した合計値とし、実装差を `gzip9_impl` メトリック + RESULTS.md の GNU gzip クロスチェック値（−17/−20B）で明示した。
- **DOM mutation 計装の配置**: 計装を variant 側ではなく bench-shared（`MutationMeter` + `observeEl`）に置き、vanilla/solid が同一オブザーバ経路を通るよう統一。`meters_driver` メトリックで「Solid 側が実際に signal 駆動 host を使った」ことを証跡化（比較の同一性担保）。
- **Svelte 5 延期**: AC5 は 2 候補で判定可能（design の要件どおり）。`build.mjs` VARIANTS 追加のみで同一経路に載るため、第 3 候補の追加コストは小さいことを RESULTS.md に記録した。
- **`experiment` フィールドの修正**: `new_experiment_log.js` が生成した seq 1 の `experiment` パスがリネーム前ファイル名を指していたため、audit（`experiment-path-mismatch`）に合わせて正しいパスへ修正した。

## Related Experiments

### Adopted

- [../exp/ac5-frontend-compare.jsonl](../exp/ac5-frontend-compare.jsonl) — AC5 2 候補比較（実装過程・計測値・判定の記録）
- [../exp/ac6-dist-single-binary.jsonl](../exp/ac6-dist-single-binary.jsonl) — AC6 配布検証（release 単体・portable zip・文書評価整理の記録）

### Rejected

なし。

## Validation

実機（Windows 11 x64・WebView2 155.0.4283.62・wgpu `NVIDIA GeForce RTX 4070 SUPER (Vulkan)`・rustc 1.99.0・Node v26.10.0/npm 11.19.1）で実施:

- `mise exec -- cargo fmt --manifest-path poc/ui/Cargo.toml -- --check` → exit 0
- `mise exec -- cargo clippy --manifest-path poc/ui/Cargo.toml --all-targets -- -D warnings` → 警告なし
- `mise exec -- cargo build --manifest-path poc/ui/Cargo.toml` → 成功
- `cd poc/ui/frontend && npm ci && npm run build` → 成功（Vite 7.3.7・dist/ 集約 + build-meta.json 生成）
- `--experiment frontend --variant vanilla` → `status:"passed"`（証跡 `poc/ui/out/frontend-vanilla-4.jsonl`。dist 11,371B/5,152B・build 1,417ms・glue 91 行・mutations/meter=1.0・driver=createMeterHost・fps 60.0/dropped=0）
- `--experiment frontend --variant solid` → `status:"passed"`（`frontend-solid-4.jsonl`。dist 20,194B/8,682B・build 2,035ms・glue 188 行・mutations/meter=1.0・driver=injected・fps 60.0/dropped=0）
- `--experiment dist`（debug）→ `status:"passed"`（`dist-manual-1.jsonl`。binary 18,052,096B・embedded_ok・fs-debug・webview2 155.0.4283.62）
- `cargo build --release` → 成功（`ui-poc.exe` 8,100,864B・1m25s）
- release 単体 temp dir 検証 → `--check` 全項目・`--experiment dist` passed（embedded-release）・`--experiment frontend --variant vanilla` passed（埋め込みのみで完走）
- portable zip（`Compress-Archive` 3,039,116B → `Expand-Archive` → 別 temp dir）→ `--check` 全項目・`--experiment dist` passed
- `audit_experiment_log.js --dir docs/impl/exp --json` → `findings: []`
- `git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml` → 空（隔離維持）

## Risks

- **gzip-9 の実装差**: miniz_oxide と GNU gzip で −17/−20B の差。比較判定には影響しないが、他実装との突合時は `gzip9_impl` を参照すること。
- **WebView2 検出経路の Windows 限定性**: `check.rs` のレジストリ検出は Windows 専用。macOS/Linux 対応時は dist 実験の検出部を OS 別実装へ拡張する必要がある（現状は Windows 前提の PoC）。
- **Linux 文書評価の実機未検証**: `libwebkit2gtk-4.1` 前提・deb depends・glibc ベースライン・AppImage は調査ベース。実配布判断前に Ubuntu LTS/Debian stable でのビルド・実行検証が必要。
- **build-meta.json の FS↔埋め込み一致**: `meta_total_bytes_match` で自己検証しているが、部分的な手動編集（dist 内ファイルのみ改変）は検出しない。CI で再ビルド→実験の順序を守る運用が前提。
- **solid variant の bench-shared 依存度**: meter host 注入は `BenchElements` の optional 契約のため、将来の第 3 候補追加時も同じ注入 I/F を使うこと（runner 側の変更不要）。

## Follow-ups

- AC7 採否 ADR 確定（`poc-adr-finalize`）: AC5/AC6 の本記録を「フロントエンド FW」「配布」採否条項の入力として使用。Solid 採用可否は ADR での判断事項（本 PoC は比較エビデンスの供給まで）。
- NSIS/WiX インストーラ評価: portable zip で hands-on 要件は充足したため、製品配布時に必要なら別タスクで比較検討。
- macOS/Linux 実機検証: `.app`+署名/公証・`libwebkit2gtk-4.1`+GTK3・deb/AppImage の実機確認は本 PoC 範囲外。配布対象 OS 確定時に実施。
- Svelte 5 variant: 必要になった場合、`build.mjs` VARIANTS + `frontend/svelte/` 追加のみで同一計装経路に乗せられる（追加容易性を RESULTS.md に記録済み）。
