---
id: "PLAN-732clapLifecycleAudit04"
type: "plan"
status: "approved"
title: "Issue #732 production CLAP adapter lifecycle 監査 — 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-732-clap-lifecycle-audit.md"
    - "docs/designs/issue-732-clap-lifecycle-audit.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-732-clap-lifecycle-audit.md"
  implements:
    - "docs/specs/issue-732-clap-lifecycle-audit.md"
    - "docs/designs/issue-732-clap-lifecycle-audit.md"
  implemented-by:
    - "docs/tasks/issue-732-clap-lifecycle-audit.md"
  depends-on:
    - "docs/specs/issue-732-clap-lifecycle-audit.md"
    - "docs/designs/issue-732-clap-lifecycle-audit.md"
  blocks: []
---

# Issue #732 実装計画

## スコープ分割

単一 slice。監査（read-only 照合）→ 必要時の欠陥修正（TDD）→
validator lane 拡張 → evidence/docs → 検証 → PR の順。

## ファイルマップ

| 区分 | パス | 変更 |
| --- | --- | --- |
| 監査対象（原則 read-only） | `crates/ruviform-plugin-clap/src/{plugin,adapt,gain,host,lib}.rs` | 欠陥発見時のみ修正 |
| 共有機械（条件付き lease） | `crates/ruviform-plugin-core/src/lifecycle.rs` | core 自体の欠陥確定時のみ |
| validator lane | `ci/plugin-abi-validator/run.mjs`・`tests/run.test.mjs` | production CLAP leg 追加 |
| evidence | `evidence/rf732/` | 監査報告 + validator 出力 |
| docs chain | `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir}/issue-732-*.md` | 新規 |

## リスクと対策

- **clack binding の未精査リスク**: 生存期間型が不正呼び出しを弾く
  範囲を crate ソースで確認してから判定する（推測で「欠陥なし」と
  しない）。
- **clap-validator 0.4.1 panic（rf#726）**: production plugin は
  `PluginParams` を宣言するため param-fuzz 系 leg が走り得る。
  異常終了時は leg 単位で Unsupported/NotRun に落とし、他 leg の
  verdict と分離して記録する。
- **run.mjs 共有 file リスク**: rf#630/#726 と共有。構造変更を避け、
  target ループ化は既定経路の leg 順・出力を不変に保つ最小差分に
  留める。`node --test` の contract test で pin する。
- **docs index 競合**: `docs/**/README.md` は編集・再生成しない
  （Coordinator が統合時に再生成）。

## 検証マトリクス

| 層 | コマンド | 目的 |
| --- | --- | --- |
| format | `mise run fmt-check` | rustfmt |
| check/lint/test | `mise run check|lint|test ruviform-plugin-clap` | adapter crate |
| L1 real plugin | `mise run check|lint|test-l1-real-plugin` | production cdylib の host bridge 実走（非回帰） |
| validator contract | `node --test ci/plugin-abi-validator/tests/` | run.mjs 変更の pin |
| 外部 validator | `mise run test-plugin-abi-validator` + `node run.mjs --clap-production …` | clap-validator 実走（opt-in） |
| hooks | `mise exec -- prek run --all-files` | pre-commit 相当 |

## ロールバック

run.mjs 拡張は opt-in flag で既定挙動不変のため、問題時は flag を
使わなければ fixture lane は従来どおり。adapter 修正が発生した場合は
該当 commit を revert する単位で分離する。
