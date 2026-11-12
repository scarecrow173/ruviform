---
id: "IMPL-jvB1bonFvoIksy2Ruk6Oui"
type: "impl"
status: "completed"
title: "Decide plugin binding method ADR"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added:
      - type: "file"
        path: "docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md"
      - type: "file"
        path: "docs/impl/ir/decide-plugin-binding-method-adr.md"
    modified:
      - type: "file"
        path: "docs/adr/README.md"
      - type: "file"
        path: "docs/specs/implement-plugin-foundation.md"
      - type: "file"
        path: "docs/discovery/evaluate-plugin-binding-options.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/decide-plugin-binding-method-adr.md"
  implemented-by: []
  depends-on:
    - "poc-clap-binding-adapter.md"
    - "poc-vst3-binding-adapter.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-plugin-foundation.md"
    - "../../specs/implement-plugin-foundation.md"
    - "../../adr/plugin-architecture-clap-vst3.md"
    - "../../discovery/evaluate-plugin-binding-options.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/plugin-foundation.md"
  references:
    - "https://github.com/prokopyl/clack"
    - "https://github.com/coupler-rs/vst3-rs"
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Decide plugin binding method ADR

## Summary

task `TASK-034JgsHEkIUa3P4jzOizoM`（plan `PLAN-034JcvpHIlt5JmqvgOquvI` T5）として、T3/T4 の PoC 実測を入力とする binding 方式 ADR `ADR-034JDzseXORnckswW1oYKh`（[../../adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md](../../adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md)、accepted）を作成した。決定は **CLAP = `clack` 0.2.0 系（exact pin）・VST3 = `vst3`（coupler-rs）0.3.0 採用**で、不採用側の退路（`clap-sys` 薄ラッパー・自前 COM 層）の見積りも併記した。

## Context

- 実施タスク: [../../tasks/decide-plugin-binding-method-adr.md](../../tasks/decide-plugin-binding-method-adr.md)
- 上流: [plan](../../plans/implement-plugin-foundation.md) T5、plugin ADR（binding 方式を本評価へ保留した上位決定）、discovery `DISC-034JccH31pBpKOSW5fst0g`（PoC Findings が実測根拠の正本）
- 入力 PoC: [T3 IR](poc-clap-binding-adapter.md)・[T4 IR](poc-vst3-binding-adapter.md)

## Implementation

### Changes

- `docs/adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md` — 新規方式 ADR（MADR 4.0.0・adr-doc skill の `new_adr.js` で骨格生成後、PoC 実測で全節を記入）
- `docs/adr/README.md` — 索引登録（accepted）
- `docs/specs/implement-plugin-foundation.md` — `relations.derives-from` に方式 ADR リンク追加
- `docs/discovery/evaluate-plugin-binding-options.md` — `derived-by` に方式 ADR 追加・status `draft`→`promoted`・Promotion Candidates の adr-doc 項目を完了に更新

### Decisions During Implementation

- 規格ごとの構築方式: CLAP=`clack`、VST3=`vst3`（規格単位の方式差異を許容する方針どおり別々に採否）
- in-place 対応: CLAP 側は `clack` `ChannelPair::InPlace` 分類の専用分岐、VST3 側はエイリアスチャンネルのみ activate/`setupProcessing` 時事前確保の per-instance bounce 領域へ逃がすスクラッチ経路。`static`/`thread_local` 共有スクラッチ禁止
- エイリアス検出粒度: per-channel ポインタ対比較（部分エイリアス `out[i]==in[j]`・出力重複も対象）
- VST3 エクスポート条件: macOS `bundleEntry`/`bundleExit` 必須・Linux `ModuleEntry`/`ModuleExit` 必須・Windows `InitDll`/`ExitDll` 任意（upstream example の `BundleEntry`/`BundleExit` 命名ではなく spec 名を採用する注意を維持）
- pin 管理: `clack-*` `=0.2.0` exact pin 維持（0.x churn 対策）、`vst3` `0.3.0`（緩和は MSRV 実測後に再評価）

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- 構造検証: `node .agents/skills/doc-status/scripts/doc_status.js audit --type adr --json` → 0 findings、`lint --rule broken-link --json` → 0 findings
- 正本突合: ADR の決定記録は discovery PoC Findings 節（CLAP T3・VST3 T4 実測）と整合 — ゼロコピー・unsafe 量と局所性・alloc・ビルド要件・ライセンス・拡張カバレッジを表形式で比較
- T6 への確認残件を ADR に明記: `vst3` MSRV 1.85 単独ビルド・macOS/Linux エントリのクロスビルド・イベント整列安定化・生成バインディング SDK 出自のライセンス継続調査

## Risks

- `clack-*` は `=0.2.0` exact pin で 0.x churn を抑えているが、upstream 修正の取り込みには pin 更新の再評価が必要
- ADR 作成時点で `vst3` 0.3.0 の MSRV 1.85 単独ビルドは未実測だった（後続 T6 で `cargo +1.85 check` 実測・解消済み — [implement-plugin-adapters.md](implement-plugin-adapters.md) 参照）
- `vst3` 生成バインディングの SDK 出自（ライセンス系統）の継続調査が未完了

## Follow-ups

- T6（implement-plugin-adapters）: 上記 4 残件の確認・方式 ADR どおりの実装整合
- Issue #22: 実際の alloc 検証レーン待ち（現状は `rt_` 命名テスト＋構造レビューのみ）
