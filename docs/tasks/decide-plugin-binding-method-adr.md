---
id: "TASK-034JgsHEkIUa3P4jzOizoM"
type: "task"
status: "todo"
title: "PoC 実測に基づく binding 方式 ADR の決定"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-plugin-foundation.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-plugin-foundation.md"
    - "poc-clap-binding-adapter.md"
    - "poc-vst3-binding-adapter.md"
  blocks:
    - "implement-plugin-adapters.md"
    - "sync-docs-and-verify-plugin-foundation.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../adr/plugin-architecture-clap-vst3.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-plugin-foundation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/plugin-foundation.md"
  references:
    - "https://github.com/prokopyl/clack"
    - "https://github.com/coupler-rs/vst3-rs"
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
---

# PoC 実測に基づく binding 方式 ADR の決定

## Work

plan `PLAN-034JcvpHIlt5JmqvgOquvI` の T5（Task Breakdown の `record-plugin-binding-method-adr` 仮称行に対応）。T3/T4 の PoC 実測に基づき、規格ごとの binding 構築方式を決定する方式 ADR を `docs/adr/` に作成する（adr-doc skill の規約に従う）。方式 ADR の入力は両規格の PoC 実測結果のみであり、アダプタ実装結果は不要（plan 依存グラフ・spec 受け入れ基準）。

- [ ] `docs/adr/` に binding 方式 ADR を新規作成。記録内容:
  - 規格ごとの構築方式決定（CLAP=`clack` 採否、VST3=`vst3` 採否 — 規格単位の方式差異を許容）
  - 比較根拠（T3/T4 の発見記録: ゼロコピー可否・アダプタ側 unsafe の量と局所性・process 経路の alloc 有無・ビルド要件・MSRV/ライセンス適合・拡張カバレッジ・保守状況・依存木サイズ）
  - 不採用側の退路（`clap-sys` 直上の自前薄ラッパー / 自前 COM 層の範囲見積り）
- [ ] in-place 処理の対応方式を本 ADR で決定記録する（T2 の評価結果を入力）: 専用 in-place view 分岐 vs スクラッチ経路。スクラッチ経路を採る場合、データバウンス用スクラッチもチャンネルリスト用スクラッチと同様に activate/prepare 時に事前確保する per-instance 状態とする規則を併記する（`static`/`thread_local` 共有は禁止 — spec Requirements 参照）
- [ ] エイリアス検出の粒度を **per-channel ポインタ対比較**として決定記録する（バッファ全体/`in_place_pair` の等価判定のみに留めない。部分エイリアス `out[i]==in[j]` や出力チャンネルポインタの重複も検出対象 — spec Requirements 参照）
- [ ] VST3 エクスポートシンボルのプラットフォーム条件を決定記録する（macOS: `bundleEntry`/`bundleExit` 必須、Linux: `ModuleEntry`/`ModuleExit` 必須、Windows: `InitDll`/`ExitDll` 任意。アダプタ側の検証基準に反映）
- [ ] spec `SPEC-034JcgdAKPNeh1YCFyprzh` の front matter relations に方式 ADR へのリンクを追加し、`docs/adr/README.md` の索引に新規 ADR を登録する（discovery `evaluate-plugin-binding-options.md` への PoC 結果・方式 ADR `derived-by` リンク・status 同期は任意の追記）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-plugin-foundation.md](../plans/implement-plugin-foundation.md) — `PLAN-034JcvpHIlt5JmqvgOquvI`（approved）。T5 チェックリスト・V-5 の正本
- [../specs/implement-plugin-foundation.md](../specs/implement-plugin-foundation.md) — `SPEC-034JcgdAKPNeh1YCFyprzh`。方式 ADR の受け入れ基準（両規格 PoC 結果に基づく決定記録・spec からのリンク）
- [../discovery/evaluate-plugin-binding-options.md](../discovery/evaluate-plugin-binding-options.md) — `DISC-034JccH31pBpKOSW5fst0g`（draft）。候補比較・退路・既知未確定事項の正本
- [../adr/plugin-architecture-clap-vst3.md](../adr/plugin-architecture-clap-vst3.md) — plugin ADR（accepted）。binding 方式を後続評価に保留した決定の上位文書
- [../adr/README.md](../adr/README.md) — ADR 索引（登録先）
- [../test-specs/plugin-foundation.md](../test-specs/plugin-foundation.md) — `TSPEC-034Jg6YSP6CLjJHnFGG5oL`。方式採否自体は ADR が保持する不変条件の範囲

## Dependencies

- 前提タスク（PoC 実測が入力）:
  - [poc-clap-binding-adapter.md](poc-clap-binding-adapter.md)（`TASK-034JgsGOtyuy2MkLRLZQI5`）
  - [poc-vst3-binding-adapter.md](poc-vst3-binding-adapter.md)（`TASK-034JgsHeZt7Bm40uBMo9wg`）
- 後続をブロック: [implement-plugin-adapters.md](implement-plugin-adapters.md) — アダプタの構築方式は方式 ADR が決定ゲート（binding 不適判明時の手戻りを防ぐため決定記録を先行）

## Affected Files

- `docs/adr/<binding-method-adr>.md` — 新規（ファイル名は adr-doc skill の命名規約に従う）
- `docs/adr/README.md` — 索引への新規 ADR 登録
- `docs/specs/implement-plugin-foundation.md` — 方式 ADR への relation リンク追加 + status 同期
- `docs/discovery/evaluate-plugin-binding-options.md` — PoC 結果・方式 ADR `derived-by` リンク・status 同期（任意の追記）

## Done When

- [ ] `docs/adr/` に方式 ADR が作成され、規格ごとの構築方式決定・比較根拠・不採用側の退路が記録されている
- [ ] in-place 対応方式（専用 in-place view 分岐 vs スクラッチ経路）・per-channel ポインタ対比較の検出粒度・データバウンススクラッチの activate/prepare 時事前確保（per-instance）・VST3 プラットフォーム条件付きエクスポートシンボル要件が ADR に記録されている
- [ ] spec front matter relations に方式 ADR リンクが追加され、`docs/adr/README.md` の索引が更新されている

## Verification

- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --type adr --json` で 0 findings
- [ ] `node .agents/skills/doc-status/scripts/doc_status.js lint --rule broken-link --json` で 0 findings
- [ ] `docs/adr/README.md` に新規 ADR 行があり、spec front matter に方式 ADR リンクが存在することを確認（V-5）
