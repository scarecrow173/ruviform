---
id: "PLAN-722vst3LifecycleAudit04"
type: "plan"
status: "approved"
title: "Issue #722 production VST3 adapter lifecycle 監査 — 実装計画"
created: "2026-11-19"
updated: "2026-11-19"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-722-vst3-lifecycle-audit.md"
    - "docs/designs/issue-722-vst3-lifecycle-audit.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-722-vst3-lifecycle-audit.md"
  implements:
    - "docs/specs/issue-722-vst3-lifecycle-audit.md"
    - "docs/designs/issue-722-vst3-lifecycle-audit.md"
  implemented-by:
    - "docs/tasks/issue-722-vst3-lifecycle-audit.md"
  depends-on:
    - "docs/specs/issue-722-vst3-lifecycle-audit.md"
    - "docs/designs/issue-722-vst3-lifecycle-audit.md"
  blocks: []
---

# Issue #722 実装計画

## スコープ分割

単一 slice。変更は 3 領域に限定される:

1. `crates/ruviform-plugin-vst3/src/component.rs` — `terminate`
   非終端化・`setupProcessing` 受理拡大・doc 同期・unit test 追加
2. `ci/plugin-abi-validator/run.mjs` — `--vst3-package` 追加と
   bundle 名導出の最小汎用化
3. `evidence/rf722/` + docs chain — 監査報告・validator 証拠・
   文書チェーン

## 依存グラフ

```
discovery (監査完了) → spec (approved) → design (approved)
  → plan (本書) → task → 実装 → test-spec 検証 → impl/ir 記録
```

- task は plan に依存。test-spec は spec/design を検証対象として
  参照し、実装完了後に結果を記録する。
- validator 実走は実装・build 完了後にのみ意味を持つ
  （修正前バイナリでは欠陥が再検出される）。

## ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-plugin-vst3/src/component.rs` | `terminate` リセット・`setupProcessing` 受理集合・doc table/comment・test 2 件追加 |
| `crates/ruviform-plugin-vst3/src/controller.rs` | 変更なし（監査のみ・結論を evidence に記録） |
| `ci/plugin-abi-validator/run.mjs` | `--vst3-package`・bundle 名導出・leg ラベル・artifacts 記録 |
| `ci/plugin-abi-validator/tests/run.test.mjs` | 既存契約維持（必要なら引数導出のテスト追加） |
| `evidence/rf722/**` | 監査報告・validator 実走/NotRun 証拠（新規） |
| `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir}/issue-722-vst3-lifecycle-audit.md` | 文書チェーン（新規） |

## リスクと対策

| リスク | 対策 |
| --- | --- |
| `Initialized` での setup が host の想定外経路を生む | SDK 契約どおりの受理であり、`config`/`prepare` の既存ガードが状態を保護。unit test で全遷移を網羅 |
| terminate リセットが processor 内部状態を残す | fixture と同じく `GainProcessor` の param 状態は保持（processor は COM object と同寿命）。`config=None` で processing 再開は不可 |
| run.mjs 汎用化が fixture lane を壊す | 既定導出が従来名を再現・契約テスト 18 件 + fixture 実走で回帰確認 |
| production validator が環境要因で NotRun | fail-closed のまま証拠に NotRun を記録（Pass と偽らない） |

## 検証マトリクス

| 層 | コマンド | 目的 |
| --- | --- | --- |
| format | `mise run fmt-check` | rustfmt |
| check | `mise run check ruviform-plugin-vst3` | 型検査 |
| lint | `mise run lint ruviform-plugin-vst3` | clippy `-D warnings` |
| unit | `mise run test ruviform-plugin-vst3` | lifecycle 往復・拒否の単体検証 |
| L1 | `mise run test-l1-real-plugin` | production cdylib の host bridge 実走非回帰 |
| runner 契約 | `node --test ci/plugin-abi-validator/tests/` | run.mjs 契約 |
| ABI lane | `node ci/plugin-abi-validator/run.mjs --vst3-dll target/debug/ruviform_plugin_vst3.dll ...` | production bundle の SDK validator 実走 |
| ABI 回帰 | 同上（既定 fixture 対象） | run.mjs 既定動作の非回帰 |

## ロールバック

全変更は単一ブランチの論理コミット。問題時はブランチ破棄で
main 状態に戻る。core・fixture・他 adapter には触れないため
影響範囲は `ruviform-plugin-vst3` と opt-in lane のみ。
