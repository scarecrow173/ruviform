---
id: "PLAN-630abiValidatorRerun04"
type: "plan"
status: "approved"
title: "Issue #630 外部 ABI validator 再実走 slice — 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-abi-validator-rerun.md"
    - "docs/designs/issue-630-abi-validator-rerun.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-630-abi-validator-rerun.md"
  implements:
    - "docs/specs/issue-630-abi-validator-rerun.md"
    - "docs/designs/issue-630-abi-validator-rerun.md"
  implemented-by:
    - "docs/tasks/issue-630-abi-validator-rerun.md"
  depends-on:
    - "docs/specs/issue-630-abi-validator-rerun.md"
    - "docs/designs/issue-630-abi-validator-rerun.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 外部 ABI validator 再実走 slice — 実装計画

## フェーズ

1. **前提確認**（済）
   - `origin/main`（`1d18e297`）から
     `feature/630-abi-validator-rerun` を作成。
   - validator pin binary の存在・version・SHA-256 を確認
     （`%LOCALAPPDATA%/ruviform/plugin-abi-validator/`）。
   - post-#735 fixture の ABI surface を read-only で確認
     （`crates/` は編集しない）。
2. **実走プローブ**（済）
   - 旧 runner のまま CLAP/VST3 を実走し、post-#735 surface での
     validator 挙動（spawn_fail の failed 署名・preset-discovery
     deadlock・VST3 setup 失敗行）を確認。orphan 化した
     clap-validator sandbox プロセスを終了。
3. **runner 更新 + contract test**（`ci/plugin-abi-validator/`）
   - `CLAP_PLUGINS` に `test_spawn_fail` + `expectCreateDecline`。
   - `CLAP_FIXTURE_LIBRARY_EXCLUDES` + `clapLegExclusions`。
   - `VST3_DECLARED_UNCREATABLE` + `applyVst3DeclaredUncreatable`。
   - schema /2・`exclusions` 複数形フィールド。
   - `tests/run.test.mjs` に新契約 test を追加。
4. **canonical 再実走 + evidence**
   - `mise run test-plugin-abi-validator` で実走し
     `evidence/rf630/abi-validator/` の run.json・raw txt を更新。
   - `evidence/rf630/abi-validator-rerun-verification.md` に
     検証記録を作成。
5. **非回帰検証**
   - `mise run fmt-check`、両 fixture crate の
     `check`/`lint`/`test`、`check-l1-real-plugin` 系、
     `test-l1-real-plugin`、`test-rt`、`prek run --all-files`。
   - `node --test ci/plugin-abi-validator/tests/`。
   - `git diff` で禁止パス（`crates/`・`docs/**/README.md`・
     `Cargo.lock`・`mise.toml`・`.github/**`・既存 doc chain）が
     空であること。
6. **doc chain + review + PR**
   - slug `issue-630-abi-validator-rerun` の文書チェーン完成。
   - review-protocol レビュー → commit → push → PR（Refs #630）。

## ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `ci/plugin-abi-validator/run.mjs` | declared-expectation・library exclusion・VST3 uncreatable・schema /2 |
| `ci/plugin-abi-validator/tests/run.test.mjs` | 新契約の contract test |
| `evidence/rf630/abi-validator/*` | 再実走 evidence（run.json schema /2 + raw txt） |
| `evidence/rf630/abi-validator-rerun-verification.md` | 検証記録（新規） |
| `docs/{discovery,specs,designs,plans,tasks,test-specs,impl}/issue-630-abi-validator-rerun*` | doc chain（新規） |

`crates/`・`Cargo.lock`・`mise.toml`・`.github/**`・既存 doc chain・
docs index は変更しない。

## リスク

- preset-discovery crawl/load の deadlock が環境依存で再現しない
  可能性 → 除外は宣言的であり、再現した場合のみ timeout コストが
  発生。除外自体は upstream issue 根拠つきで正当化済み。
- `test_spawn_fail` の decline signature が validator version で
  変わる可能性 → pattern は pin version 条件。pin 更新時に
  contract test と併せて再評価。
- VST3 `Result:` 集計と setup 失敗行の対応が SDK version で変わる
  可能性 → occurrences=0 は Failed に倒れるため fail-closed。
- `test-l1-real-plugin` の instrument 系 test が高負荷時に
  flaky（本 slice 無関係の既存 lane）→ 失敗時は再実行して
  切り分け、最終結果を正直に記録する。
