---
id: "PLAN-726clapValidatorPanic04"
type: "plan"
status: "approved"
title: "Issue #726 clap-validator param-fuzz panic — 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-726-clap-validator-panic.md"
    - "docs/designs/issue-726-clap-validator-panic.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-726-clap-validator-panic.md"
  implements:
    - "docs/specs/issue-726-clap-validator-panic.md"
    - "docs/designs/issue-726-clap-validator-panic.md"
  implemented-by:
    - "docs/tasks/issue-726-clap-validator-panic.md"
  depends-on:
    - "docs/specs/issue-726-clap-validator-panic.md"
    - "docs/designs/issue-726-clap-validator-panic.md"
  blocks: []
---

# Issue #726 実装計画

## スコープ分割

単一 slice。調査（filter 絞り込み + upstream 照合）→ 判断記録 →
evidence/docs → 検証 → PR の順。runner 変更は契約違反が確認された
場合のみ — lane 実走で pipe stdio の ETIMEDOUT 誤記録を実測確認し、
`spawnCapture`（file-backed stdio）への限定修正を実施した
（leg 定義・verdict 語彙は無変更）。

## ファイルマップ

| 区分 | パス | 変更 |
| --- | --- | --- |
| runner | `ci/plugin-abi-validator/run.mjs`・`tests/run.test.mjs` | `spawnCapture` 追加 + `runClapLeg` 適用 + contract test 2 件（lane 実走で確認した ETIMEDOUT 誤記録の修正。leg 定義は無変更） |
| fixture（read-only） | `crates/ruviform-plugin-test-clap/src/fault.rs` | 変更しない（rf#630 占有） |
| upstream（read-only） | `free-audio/clap-validator` source | 変更しない。報告文ドラフトのみ |
| evidence | `evidence/rf726/` | investigation.md・upstream-report-draft.md・filter-runs/・abi-validator/（修正前・ETIMEDOUT 誤記録の証拠）・abi-validator-fixed/（修正後の実走） |
| docs chain | `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-726-*.md` | 新規 |

## リスクと対策

- **panic が非決定的に見えるリスク**: fuzzer の PRNG はテストごとに
  固定シードで決定的だが、「どの fatal mode を最初に踏むか」は
  テストごとに異なる。filter 実行で各テストを個別に切り分けて
  決定性を確認済み（#1-#3 は毎回 panic、#4-#7 は毎回 JSON 出力）。
- **upstream master が動くリスク**: shallow clone の HEAD `5bc2ef3`
  時点で buggy code の残存を source 照合で確認。報告時に最新
  master で再確認するよう報告文に version を明記。
- **orphan sandbox child が build を阻害するリスク**: preset-discovery
  timeout の child が kill されず DLL を lock する事象を実測。
  lane 実行前に `tasklist`/`taskkill` で残存 `clap-validator.exe` を
  確認・除去してから build する運用で対処（evidence に記録）。
- **docs index 競合**: `docs/**/README.md` は編集・再生成しない
  （Coordinator が統合時に再生成）。

## 検証マトリクス

| 層 | コマンド | 目的 |
| --- | --- | --- |
| runner contract | `node --test ci/plugin-abi-validator/tests/` | spawnCapture 契約 pin + 全 contract test |
| opt-in lane | `node ci/plugin-abi-validator/run.mjs --clap-validator "$LOCALAPPDATA/ruviform/plugin-abi-validator/clap-validator.exe" --clap-production --evidence-dir evidence/rf726/abi-validator-fixed --no-build` | 修正後 runner の実走記録（`--no-build` は DLL 再生成済みのため） |
| panic 絞り込み | `clap-validator validate --json --plugin-id org.ruviform-test.clap-test-fault --include <regex> target/debug/ruviform_plugin_test_clap.dll` | 最小再現の raw 証拠 |
| hooks | `mise exec -- prek run --all-files` | pre-commit 相当 |

Rust 変更が発生しないため `mise run check|lint|fmt-check` は不要
（Issue 本文の検証方法どおり）。

## ロールバック

runner 変更は `spawnCapture` 追加と `runClapLeg` の stdio 差し替えに
限定され、leg 定義・verdict ロジックは無変更。問題時は当該 commit を
revert すれば pipe stdio 実装に戻る（ETIMEDOUT 誤記録の既知劣化を
伴う）。evidence/docs は記録物であり revert 不要。
