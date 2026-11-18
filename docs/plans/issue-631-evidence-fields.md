---
id: "PLAN-p7Xv3kNm5Qw8rTs4yHb6Jd"
type: "plan"
status: "approved"
title: "Issue #631 AC4 evidence-fields 完成 — 実装計画"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-631-evidence-fields.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 AC4 evidence-fields 完成 — 実装計画

## スコープ分割

単一 plan — 変更は `crates/ruviform-plugin-dock-test` に閉じ、
4 系統の独立した additive 変更で構成される。依存は
「helper 追加 → case body への適用」の順のみ。

## ファイルマップ

| ファイル | 変更内容 |
| --- | --- |
| `crates/ruviform-plugin-dock-test/src/real_dock.rs` | `RealDockContext` に window identity 保持、session 開始時 digest 計算、`finalize_case` stamp、`capture_sha256` helper、`save_pixel_artifact`/`record_pixel_artifact` の `{path,sha256}` 拡張、`real-capture` 対応、`real-window-mount`/`real-command-receipt` の semantic tree 全量化、unit tests |
| `crates/ruviform-plugin-dock-test/src/scenario.rs` | `SemanticGuiTest` generic の semantic tree helper 追加、headless `command_receipt_body` の `semantic_tree` 全量化 |
| `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-real-scenario.rs` | `--pixel-artifacts` doc に `real-capture` 追記 |
| `crates/ruviform-plugin-dock-test/README.md` | verdict 表修復（dpi/close/command-receipt） |
| `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs` | headless `command-receipt` の semantic tree 全量断言 |
| `mise.toml` | `test-ui-plugin-dock-real` description 修正（引数不変） |
| `evidence/rf631/plugin-dock-real-headless.jsonl` | `--blocked-probe` 再生成 |
| `evidence/rf631/plugin-dock-real-scenario.jsonl` | interactive desktop があれば再生成 |
| `evidence/rf631/verification.md` | slice 記録追記 |
| `docs/{discovery,specs,designs,plans,tasks,test-specs,adr}/issue-631-evidence-fields.md` | doc chain |
| `docs/impl/{ir,exp}/issue-631-evidence-fields.md` (+exp.jsonl) | 実装記録 |

## 依存グラフ

```
window identity helper (canonical 文字列→digest)
  → run_all_with_pixel_artifacts で計算・ctx 保持
  → finalize_case merged extras で environment に stamp

capture_sha256 helper (rgba bytes→hex)
  → real-capture observed + dcomp 3 case の step row に記録

save_pixel_artifact (width,height,rgba)→{path,sha256}
  → record_pixel_artifact → 全 caller row に artifact object
  → real-capture の --pixel-artifacts 対応

semantic_tree helper (scenario.rs, SemanticGuiTest generic)
  → real-window-mount / real-command-receipt (real_dock.rs)
  → command_receipt_body (scenario.rs headless)
```

4 系統は互いに独立 — 順序は任意だが、helper を先に置いて
から case body に適用する。

## タスク分解

1. **T1 window identity** — `RealDockContext` field 追加、
   session 開始時の geometry+id+title 収集、canonical 文字列
   →`Sha256::digest`、`finalize_case` merged extras、
   `#[cfg(test)]` unit test。
2. **T2 capture/artifact digest** — `capture_sha256` helper、
   `real-capture`・dcomp 3 case の全 capture row に記録、
   `save_pixel_artifact` 一般化 + `{path,sha256}`、
   `record_pixel_artifact` 拡張、`real-capture` の
   `--pixel-artifacts` 対応、unit test。
3. **T3 semantic tree 全量化** — `scenario.rs` に
   `SemanticGuiTest` generic helper、real lane 2 case と
   headless `command_receipt_body` に適用、unit/integration
   test。
4. **T4 doc 修復** — README verdict 表、bin doc、
   `mise.toml` description。
5. **T5 evidence 再生成** — headless blocked-probe（必須）、
   real-scenario（interactive desktop あれば）、
   `verification.md` 追記。
6. **T6 検証・レビュー・提出** — 全 mise lane、review-
   protocol、commit/push/PR/worker_done。

## リスク登録

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `window_geometry` session 開始時失敗 | identity が `null` | `sha256: null` で正直記録・捏造しない |
| interactive desktop 不在 | real-scenario JSONL 未再生成 | NotRun として verification.md/PR に明記・既存 JSONL を上書きしない |
| `ComponentSpec` 将来 variant 追加 | walk の match が非網羅 | `validate_spec` と同じ網羅 match でコンパイルエラー検出 |
| `artifact` shape 変更（path→object） | 既存 consumer | 本 slice の evidence consumer は repo 内 test/docs のみ — verification.md に shape 変更を明記 |
| semantic tree walk で observe 失敗 key | `null` 記録 | verdict と分離・捏造しない |

## ロールバック

各系統は additive で独立 — 問題があれば該当 commit を
revert すれば他系統に影響しない。evidence JSONL は git
管理のため revert で復元可能。

## 検証マトリクス

| 変更 | 検証 |
| --- | --- |
| T1 window identity | `mise run test ruviform-plugin-dock-test --features real-dock`（unit test）+ real-scenario JSONL の `environment.window_identities` 確認 |
| T2 capture/artifact | 同上 + opt-in artifacts run で `{path,sha256}` 確認 + canonical run で PNG 非生成確認 |
| T3 semantic tree | `mise run test ruviform-plugin-dock-test`（headless integration）+ `--features real-dock` unit + real JSONL の `semantic_tree` key 確認 |
| T4 doc 修復 | `mise run test-ui-plugin-dock-console`（catalog 整合）+ README 表の目視 |
| T5 evidence | `mise run test-ui-plugin-dock-real-headless`（必須）+ `test-ui-plugin-dock-real`（interactive あれば） |
| 全体 | `fmt-check`/`check`/`lint`/`test`（default+real-dock+lab-gui）/`test-ui-plugin-dock{,-console,-lab}`/`test-ui-scenario-contract`/`check-ui-driver-boundary`/`prek` |

## 前提・参照

- `docs/designs/issue-631-evidence-fields.md` — 設計詳細。
- `docs/specs/issue-631-evidence-fields.md` — 要件・AC。
- `docs/discovery/issue-631-evidence-fields.md` — 探索記録。
- `POLICIES.md` — 検証レベル・fail-closed 規範。
