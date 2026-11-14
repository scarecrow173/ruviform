---
id: "DISC-034XBVWijUovcdqO2rYXzx"
type: "discovery"
status: "resolved"
title: "Issue #294 virtual-first evidence schema — consumer-verify Case schema v2 現状監査"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/627"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-294-virtual-first-evidence-schema.md"
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
  related:
    - "docs/discovery/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-627-policies-revision-proposal.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
  verifies: []
  verified-by: []
  references:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-627-policies-revision-proposal.md"
    - "ci/consumer-verify/evidence.mjs"
    - ".github/workflows/consumer-verify.yml"
    - "ci/daily/decide.mjs"
    - "ci/tracker/tests/issue-tracker-workflows-contract.test.mjs"
  defers: []
  deferred-by: []
---

# Issue #294 virtual-first evidence schema — consumer-verify Case schema v2 現状監査

## Summary

Issue #294 first slice の対象である `ci/consumer-verify/` 証拠契約を、着地済み virtual-first 検証契約（`scarecrow173/ruviform#627`、POLICIES Effective 2026-11-15）の Case schema へ additive に拡張するための現状監査。lane record は現在 `{name, command, feature, os, outcome, reason}` の6項目で、Case schema の `case_id`/`level`/`environment_kind`/`requirement`/独立 Oracle/fixture 版/resolved Ruviform SHA を保持しない。結論: schemaVersion を 2 へ bump し `contract` は `consumer-verify-tuple/v1` のまま維持する拡張で実現可能 — downstream（`ci/daily/decide.mjs`）は `schemaVersion` を検証せず `lanes` を読まず、`outcome` 語彙は `ci/tracker` の contract test が5値固定で pin しているため結果軸名は変更できない。確度: high（コード実測に基づく）。

## Exploration Goal

Issue #294 の first slice「virtual-first Case schema への証拠契約拡張（schemaVersion 1→2）」について、(a) 現行 lane/case record と evidence JSON の実形状、(b) 拡張が downstream consumer（daily tuple recovery・tracker contract test・workflow summary）を壊さないか、(c) workflow の静的 lane 行へ level/environment_kind/requirement を正直に宣言する方法、を確定する。

## Key Issues and Assumptions

- **結果軸の項目名**: 受入条件は record が `result` を保持すると読めるが、実装指針は「結果軸（現行 `outcome`）」と明記し、`ci/tracker/tests/issue-tracker-workflows-contract.test.mjs` が `deriveStatus`/`OUTCOMES` を import して lane object の `outcome` キーと5値語彙を pin している（編集範囲外）。→ 結果軸は `outcome` を維持し、`requirement` を直交する別項目として追加するのが唯一の整合解。confirmed。
- **`contract` 文字列の固定**: `ci/daily/decide.mjs`（~line 1424）が `DAILY_TUPLE_CONTRACT` 既定 `consumer-verify-tuple/v1` を pin し、`classifyRunEvidence`（~lines 406-447）は `kind`/`contract`/`run.runId`/`consumer.*`/`status`/`tuple.*` を検証するが `schemaVersion` と `lanes` を読まない。→ `schemaVersion` のみ bump、`contract` は不変で daily 回復経路は壊れない。confirmed。
- **fail-closed 検証の波及**: `readLanes` が新必須項目を検証するようになると、workflow の全 lane 行（`rec()` 経由・extra task 直接 `lane` 呼出し双方）が新項目を供給しなければ finalize が失敗し最小証拠 fail になる。→ workflow 側の全記録経路を同時に更新する必要がある。confirmed。
- **静的 lane 行の正直な分類**: 既存の `unsupported`/`not_run` 固定行（GUI visual/OS・PLUGIN-NATIVE-DOCK・macOS/実 OS・GUI-CONTRACT consumer-side）に level/environment_kind/requirement を付与する際、outcome は一切変更しない（既存語彙の意味は不変）。requirement は POLICIES §2 の OS別表に従い Windows 提供機能の GUI-L2/EXT-L3 を Required、macOS OS-L1 を Required と宣言する。confirmed（分類値は design で固定）。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. lane record に additive フィールド追加 + `schemaVersion` 2、`contract` v1 維持 | daily/tracker/yml 契約を壊さない。旧 artifact の意味不変。additive で後方互換 | readLanes の fail-closed 強化で workflow 全行の更新が必要 | yes |
| B. `contract` を `consumer-verify-tuple/v2` へ bump | schema 版と契約版が一致して見える | `ci/daily`・`daily-ci.yml` env・`docs/development-environment.md` へ波及し本 slice scope 外。decide.mjs の証拠照合を壊す | no |
| C. `outcome` を `result` へ改名 | Case schema の項目名と一致 | `ci/tracker` contract test（範囲外）が `outcome` キーと5値を pin。修正には範囲拡張が必要で利益がない | no |
| D. 新項目を optional のまま readLanes で検証しない | workflow 変更が最小 | fail-closed 原則違反。偽造/未知値が status に寄与し得る | no |

## Tentative Conclusions and Hypotheses

- confirmed: `SCHEMA_VERSION` を 2 に bump し `CONTRACT = "consumer-verify-tuple/v1"` を維持する。evidence JSON の `lanes[]` 各行が Case schema の記録項目を保持する。
- confirmed: lane record の必須新規項目は `case_id`・`level`（`OS-L1`/`GUI-L2`/`EXT-L3`）・`environment_kind`（5値）・`requirement`（`Required`/`Deferred`/`NotApplicable`）。`oracle`/`fixture_version`/`resolved_ruviform_sha`/`expected`/`observed`/`device`/`endpoint`/`plugin`/`abi`/`sdk`/`arch`/`os_ver` は optional（提示時は語彙・型を fail-closed 検証）。
- confirmed: `requirement` と結果軸（`outcome`）は直交 — 同一項目へ畳み込まない。`skipped` は derived record 専用の outcome であり Case result 語彙（Pass/Fail/NotRun/Unsupported）とは別物として維持。
- hypothesis: 現行 lane の大半は `OS-L1`/`deterministic-backend`/`Required`、GUI-CONTRACT/SEMANTIC 系も headless contract のため同分類、`GUI visual/OS` 行は `GUI-L2`/`real-window`/`Required`、`PLUGIN-NATIVE-DOCK` 行は `EXT-L3`/`test-plugin`/`Required`、`macOS/実 OS` 行は `OS-L1`/`deterministic-backend`/`Required`（outcome=unsupported は不変）。design で確定する。

## Open Questions

- なし（分類値・CLI flag 名・テスト方針は design/spec で確定可能。Issue レベル AC の tuple binding・consumer oracle・fixture version（#628-#631 由来）は本 slice 対象外として deferred 記録する）。

## Promotion Candidates

- [x] spec-doc needed: `docs/specs/issue-294-virtual-first-evidence-schema.md`（Case schema v2 の要求・AC）
- [ ] adr-doc needed: なし（契約は #627 design が正本。本 slice はその適用）

## Sources and Relations

- Issue #294（first slice: virtual-first Case schema への証拠契約拡張）/ Issue #627（着地済み virtual-first 検証契約、Closed）
- `docs/designs/issue-627-virtual-first-verification-contract-design.md` §1 `environment_kind` 分類・§2 Case schema
- `docs/designs/issue-627-policies-revision-proposal.md` ゲート移管台帳 RF #294/#281 行
- `POLICIES.md`（Effective: 2026-11-13 OS別検証・CI / 2026-11-15 仮想優先受入）
- `ci/consumer-verify/evidence.mjs`、`.github/workflows/consumer-verify.yml`、`ci/daily/decide.mjs`、`ci/tracker/tests/issue-tracker-workflows-contract.test.mjs`（実測）

## Evidence

- 2026-11-15 実測: `evidence.mjs` の `makeLaneRecord`（L81-96）は `{name, command, feature, os, outcome, reason}` を返し、`SCHEMA_VERSION = 1`（L64）・`CONTRACT = "consumer-verify-tuple/v1"`（L63）・`OUTCOMES` 5値（L62）。`readLanes`（L266-289）は name/outcome のみ fail-closed 検証。
- 2026-11-15 実測: `decide.mjs` `classifyRunEvidence`（L406-447）は `ev.kind`/`ev.contract`/`ev.run.runId`/`ev.consumer.{name,repo}`/`ev.status`/`ev.tuple.{ruviformRev,consumerResolvedSha}` を検証。`schemaVersion`・`lanes` は未参照。`DAILY_TUPLE_CONTRACT` 既定は L1424。
- 2026-11-15 実測: `ci/tracker/tests/issue-tracker-workflows-contract.test.mjs` L26 が `import { deriveStatus, OUTCOMES } from "../../consumer-verify/evidence.mjs"`、L270-277 が `outcome` キーの lane object を構築、L333-339 が `OUTCOMES` の5値 deepEqual を pin。
- 2026-11-15 実測: workflow の finalize 失敗 fallback `printf`（audiolens L631 / synvelis L1089）が `"schemaVersion":1` を hardcode。両 verify job の `rec()` helper（audiolens L573-579 / synvelis L1027-1033）と extra task 直接 `lane` 呼出し（audiolens L543-551 / synvelis L997-1005）が記録経路の全て。
