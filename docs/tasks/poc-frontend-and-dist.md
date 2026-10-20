---
id: "TASK-034JfHyxKSwvhiQ0sKqjXQ"
type: "task"
status: "todo"
title: "PoC フロントエンド比較と配布検証（Solid 移植・単一バイナリ・OS 別方針）"
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
    - "poc-render-backends.md"
  blocks:
    - "poc-adr-finalize.md"
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-28-ui-poc.md"
    - "../designs/issue-28-ui-poc.md"
    - "../adr/ui-tech-selection-poc.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/issue-28-ui-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/issue-28-ui-poc.md"
  references:
    - "https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution"
    - "https://blogs.gnome.org/mcatanzaro/2025/04/28/webkitgtk-api-versions/"
  defers: []
  deferred-by: []
---

# PoC フロントエンド比較と配布検証（Solid 移植・単一バイナリ・OS 別方針）

## Work

plan `PLAN-034JfDEU8mhCDj4ldYFZUG` の T5。design の AC5・AC6 実験設計を正本として、フロントエンド 2 候補比較と Windows 単一バイナリ配布を hands-on 検証し、macOS / Linux は文書評価で整理する。

- [ ] `frontend/solid/`（Vite + vite-plugin-solid）に T2 のベンチ UI（AC2 ワークロード + intent/差分表示部品）を bench-shared 共有で移植する
- [ ] `--experiment frontend --variant vanilla|solid` で比較計測: `dist` 合計バイト（raw / gzip -9・再現コマンド付き）・Vite ビルド時間・glue-code LOC・更新粒度（メータ更新当たりの DOM mutation 数・Solid の fine-grained 更新が部分適用に留まるか）・開発ワークフロー観察（ビルド→埋め込み配線の手順）を `RESULTS.md` に 2 候補比較行として記録（AC5。Svelte 5 は YAGNI 延期だが追加容易である旨を記録）
- [ ] `cargo build --release --manifest-path poc/ui/Cargo.toml` でアセット埋め込み単一バイナリを作り、実機で埋め込みアセットのみで起動・`--check` 全項目を報告することを確認（AC6）
- [ ] `src/exp/dist.rs` に `--experiment dist` を実装: WebView2 レジストリ検出（winreg 経由・HKLM/HKCU `...\Clients\{F3017226-...}` の `pv` 値）・バイナリサイズ・検出失敗時の bootstrapper 案内出力を記録
- [ ] portable zip（`ui-poc.exe` 自己完結）を hands-on で構成して動作確認。NSIS/WiX は参照評価とする（spec の「いずれか」要件を充足）
- [ ] macOS（`.app` + 署名/公証・productbuild/dmg）・Linux（`libwebkit2gtk-4.1` + GTK3 実行時依存・deb 依存・glibc ベースライン・AppImage）を「文書評価」行として `RESULTS.md` に記録し、Linux 実行時依存リスクを依存パッケージ / API バージョン表（4.1/soup3 が唯一の現実経路・4.0/soup2 は WebKitGTK 2.52 で削除済み）で定量化する

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/issue-28-ui-poc.md](../plans/issue-28-ui-poc.md) — `PLAN-034JfDEU8mhCDj4ldYFZUG`（approved）。T5 の正本
- [../designs/issue-28-ui-poc.md](../designs/issue-28-ui-poc.md) — AC5 frontend 実験・AC6 dist 実験の正本
- [../specs/issue-28-ui-poc.md](../specs/issue-28-ui-poc.md) — AC5・AC6 の正本
- [../adr/ui-tech-selection-poc.md](../adr/ui-tech-selection-poc.md) — 「フロントエンド FW」「配布」採否条項の対象
- [../test-specs/issue-28-ui-poc.md](../test-specs/issue-28-ui-poc.md) — 比較項目・文書評価ラベル・リスク定量化の判定規則

## Dependencies

- 前提: `poc-ui-harness-host.md`（host・assets・check・metrics）、`poc-render-backends.md`（bench-shared・vanilla ベンチ UI を Solid 移植する前提）
- 後続をブロック: `poc-adr-finalize.md`（AC5・AC6 エビデンス供給）
- フロントエンド依存は `poc/ui/frontend/` 配下に限定（vite-plugin-solid・solid-js）。`npm ci && npm run build` が標準前提

## Affected Files

- `poc/ui/frontend/solid/` — 新規
- `poc/ui/src/exp/dist.rs`・`poc/ui/src/exp/mod.rs`（レジストリ追加）— 新規/更新
- `poc/ui/src/check.rs` — WebView2 検出の実機確認に伴う調整（必要に応じて）
- `poc/ui/RESULTS.md` — AC5 比較行 + AC6 行（Windows hands-on + macOS/Linux 文書評価 + Linux リスク定量化表）
- `docs/impl/exp/` — 実施ログを追記

## Done When

- [ ] `--experiment frontend --variant vanilla|solid` で 2 候補の比較項目（dist バイト・ビルド時間・LOC・DOM mutation 数・ワークフロー観察）が `RESULTS.md` に記録される
- [ ] release 単一バイナリが埋め込みアセットのみで起動し、`--check` 全項目・WebView2 検出・バイナリサイズ・bootstrapper 案内が記録される
- [ ] portable zip が hands-on で動作確認される
- [ ] macOS / Linux の文書評価行と Linux `libwebkit2gtk-4.1` リスク定量化表が `RESULTS.md` に存在する

## Verification

- [ ] `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment frontend --variant vanilla` / `--variant solid` — `poc-metric.v1` 最終行・`status` 規約どおり
- [ ] `mise exec -- cargo build --release --manifest-path poc/ui/Cargo.toml` 後の `poc/ui/target/release/ui-poc.exe --check` — 埋め込みのみで capability 検出を報告（実機）
- [ ] `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment dist` — WebView2 検出・バイナリサイズ・案内出力を記録
- [ ] `git status --porcelain -- Cargo.toml Cargo.lock crates` — 出力が空（隔離確認）
- [ ] Manual: `poc/ui/RESULTS.md` に 2 候補比較行・AC6 行・「文書評価」ラベル付き macOS/Linux 行・リスク定量化表が存在する（補助確認）
