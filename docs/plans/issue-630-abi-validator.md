---
id: "PLAN-630abiValidator04"
type: "plan"
status: "approved"
title: "Issue #630 外部 ABI validator slice — 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-abi-validator.md"
    - "docs/designs/issue-630-abi-validator.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-630-abi-validator.md"
  implements:
    - "docs/specs/issue-630-abi-validator.md"
    - "docs/designs/issue-630-abi-validator.md"
  implemented-by:
    - "docs/tasks/issue-630-abi-validator.md"
  depends-on:
    - "docs/specs/issue-630-abi-validator.md"
    - "docs/designs/issue-630-abi-validator.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 外部 ABI validator slice — 実装計画

## フェーズ

1. **validator provisioning 確定**（済）
   - clap-validator 0.4.1 release binary 取得・sha256 照合・配置。
   - VST3 SDK clone（commit `3cdf9ca`）→ CMake configure → MSVC で
     `validator` target build → `validator.exe -version` 確認。
2. **fixture 契約修正**（済・TDD）
   - CLAP: module path / fractional rate / MAX_EVENTS / instrument
     notes / stateful rescan。
   - VST3: terminate 非終端・setupProcessing state・bus info。
   - 各修正に先行する失敗 test を追加し red→green を確認。
3. **runner + contract test**（`ci/plugin-abi-validator/`）
   - `run.mjs`: tool 解決・build・leg 実行・verdict・evidence。
   - `tests/run.test.mjs`: verdict 語彙・解決規則・fail-closed を固定。
4. **canonical 実走 + evidence**
   - `node ci/plugin-abi-validator/run.mjs` を実環境で実行し
     `evidence/rf630/abi-validator/` に run.json + logs を記録。
   - `evidence/rf630/abi-validator-verification.md` に人間可読な
     検証記録（version/license/digest/結果）を作成。
5. **検証・文書・PR**
   - 正規検証コマンド一式を実行し exit code を記録。
   - 本 doc chain 完成 → commit → push → PR（Refs #630）。

## ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-plugin-test-clap/src/{plugin,fault,stateful,instrument,block}.rs` | validator 指摘の契約修正 |
| `crates/ruviform-plugin-test-clap/Cargo.toml` | host handle 依存のコメント更新 |
| `crates/ruviform-plugin-test-vst3/src/component.rs` | lifecycle/setup/bus 修正 |
| `ci/plugin-abi-validator/run.mjs` | 新規 runner |
| `ci/plugin-abi-validator/tests/run.test.mjs` | 新規 contract test |
| `evidence/rf630/abi-validator/*` | 実走 evidence |
| `evidence/rf630/abi-validator-verification.md` | 検証記録 |
| `docs/{discovery,specs,designs,plans,tasks,test-specs,impl}/issue-630-abi-validator*` | doc chain |

## リスク

- validator tool の version 差異 → pin 済み（clap-validator 0.4.1 /
  SDK commit `3cdf9ca`）で再現性を確保。
- `test_fault` の param 除外が「検証の弱体化」に見える → 除外理由・
  unfiltered informational leg・L1 fault lane での網羅を明記。
- docs index 未同期 → Coordinator が統合時に再生成（本 slice では触らない）。
