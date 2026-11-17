---
id: "SPEC-630abiValidatorRerun02"
type: "spec"
status: "approved"
title: "Issue #630 外部 ABI validator 再実走 slice — post-#735 declared-expectation + run.json/evidence 更新 — spec"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-abi-validator-rerun.md"
    - "docs/specs/issue-630-abi-validator.md"
    - "docs/specs/issue-630-preset-spawn-fault.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-630-abi-validator-rerun.md"
  implements:
    - "docs/discovery/issue-630-abi-validator-rerun.md"
  implemented-by:
    - "docs/designs/issue-630-abi-validator-rerun.md"
  depends-on:
    - "docs/specs/issue-630-abi-validator.md"
    - "docs/specs/issue-630-preset-spawn-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 外部 ABI validator 再実走 slice — spec

## 目的

Issue #630 AC3 の残件（PR#735 で「別 slice・NotRun」と deferral
された preset-discovery / IUnitInfo 系 / `test_spawn_fail` の
外部権威検証）を回収し、stale になった `run.json`
（schema `rf630-abi-validator-run/1`、pre-#735 fixture の記録）を
post-#735 surface の実走結果で更新する。本 slice は runner の
宣言的機構拡張と evidence 更新のみを行い、`crates/` 配下・
production adapter は一切変更しない。

## 要求

1. **post-#735 surface の再実走**: CLAP 6 plugin
   （`test_spawn_fail` を含む）+ VST3 bundle（`test_spawn_fail`
   class を含む 11 class）を pin 済み外部 validator
   （clap-validator 0.4.1 / Steinberg VST3 SDK validator 3.8.1）
   で実走し、結果を `evidence/rf630/abi-validator/` に記録する。
2. **意図的 create 拒否の declared-expectation**: `test_spawn_fail`
   は enumerable だが create を意図的に拒否する fixture 契約。
   validator が報告する create-decline signature の failed case を
   宣言的に期待値として扱い、理由・件数を run.json に記録する。
   予期しない failed/crashed case・crash・decline 未観測は
   従来どおり Failed とする（fail-closed 維持）。
3. **tool 側既知問題の宣言的除外**: clap-validator 0.4.1 の
   preset-discovery crawl/load deadlock（upstream issue #41）は
   library-level case として除外し、pattern・理由・upstream 参照を
   leg.exclusions に記録する。除外は検証の省略ではなく pin version
   条件付きの宣言的 scoping であり、descriptor-consistency case と
   fixture in-process ABI test が代替証拠となる。
4. **VST3 decline の可視化**: suite validator が `Result:` 集計に
   含めない意図的 create 拒否を `leg.declaredUncreatable` として
   出現回数つきで記録し、decline 証拠 0 件なら leg を Failed に
   倒す。
5. **schema /2**: 新規 leg フィールド（`expectation`/
   `declaredUncreatable`/`exclusions`）に対応して run.json schema を
   `rf630-abi-validator-run/2` に bump する。
6. **contract test 更新**: `node --test ci/plugin-abi-validator/
   tests/` に declared-expectation・除外合成・VST3 uncreatable の
   契約を固定する test を追加・更新する。
7. **正直な verdict**: 実走していない・tool が扱えない surface は
   Passed と書かず、NotRun/Unsupported と理由を記録する。
   informational leg は canonical verdict に含めない既存規約を維持。

## 受入条件（本 slice）

- [ ] `mise run test-plugin-abi-validator` が実環境で exit 0 を返し、
  `evidence/rf630/abi-validator/run.json`（schema /2）に 7 canonical
  leg 全て Passed・exclusion/expectation/uncreatable 記録・
  tool/artifact SHA-256・git SHA を記録する。
- [ ] CLAP `test_spawn_fail` leg が declared expectation met で
  Passed（declined 件数 > 0、unexpected 0、crashed 0）。
- [ ] VST3 leg が `235 passed, 0 failed` を維持しつつ
  `test_spawn_fail` の decline 証拠（occurrences > 0）を記録する。
- [ ] preset-discovery crawl/load 除外が全 fixture CLAP leg の
  `leg.exclusions` に理由つきで記録される。
- [ ] `node --test ci/plugin-abi-validator/tests/` が全 pass。
- [ ] 非回帰: fmt-check / check / lint / test（両 fixture crate）/
  check-l1-real-plugin 系 / test-rt / prek が実 exit code つきで
  記録される。
- [ ] doc chain（slug `issue-630-abi-validator-rerun`）と検証 md を
  作成し、docs index は触らない。

## 非目標

- `crates/` 配下の一切の編集（fixture・production adapter —
  validator 指摘が契約違反を示した場合は `ask` で占有拡張）。
- preset-discovery crawl/load の外部検証実施（upstream 修正版
  validator への pin 更新は後続 slice）。
- CI 必須化・`.github/**` 変更・`mise.toml` 変更（opt-in 維持）。
- AC4（実 Window/DComp）・AC7（consumer verdict）の gate。
- docs index（`docs/**/README.md`）の再生成 — Coordinator が統合時に
  実施。
- PR は `Refs #630` のみ — Issue は close しない（tracker）。
