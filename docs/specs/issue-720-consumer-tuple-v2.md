---
id: "SPEC-720tupleV202"
type: "spec"
status: "approved"
title: "Issue #720 consumer-verify-tuple contract v1→v2 bump — spec"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/720"
    - "docs/discovery/issue-720-consumer-tuple-v2.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-720-consumer-tuple-v2.md"
      - type: "file"
        path: "ci/consumer-verify/evidence.mjs"
      - type: "file"
        path: "ci/daily/decide.mjs"
      - type: "file"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: ".github/workflows/daily-ci.yml"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "evidence/rf720/verification.md"
  implements:
    - "docs/discovery/issue-720-consumer-tuple-v2.md"
  implemented-by:
    - "docs/designs/issue-720-consumer-tuple-v2.md"
  depends-on:
    - "docs/discovery/issue-720-consumer-tuple-v2.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #720 consumer-verify-tuple contract v1→v2 bump — spec

## 目的

`consumer-verify` の候補 tuple 証拠契約を `consumer-verify-tuple/v1`
から `consumer-verify-tuple/v2` へ bump し、Case schema v2 の証拠語彙
（`schemaVersion: 2`・lane record の `case_id`/`level`/
`environment_kind`/`requirement`）を tuple 識別に反映する。rf294 で
`schemaVersion` 側は先行 bump 済み・contract 文字列の bump は
「`ci/daily`+`daily-ci.yml`+docs 連動の後続 slice」として Deferred
されていた残件を回収する。

## 要求

1. **全経路の同時更新**: producer（`evidence.mjs` の `CONTRACT`
   export → 証拠 JSON の `contract`/`tuple.contract`）、consumer
   （`decide.mjs` 既定 `tupleContract`）、両 workflow
   （`DAILY_TUPLE_CONTRACT` env・fallback `printf`・summary/ヘッダ
   コメント）、contract test、`docs/development-environment.md` が
   全て同一の v2 文字列に揃う。v1 pin が残るのは legacy 記述として
   明示分離された箇所のみ。
2. **v1 artifact 移行方針の決定と固定**: hard cutover（v2 のみ受理・
   v1 artifact は proven different tuple として non-target）を採用し、
   コード・docs・decide テストで固定する。両受理の移行窓は設けない。
3. **`schemaVersion`/`lanes` 未検査の現状維持を記録**: consumer は
   証拠の `schemaVersion`/`lanes` を読まず、contract 文字列を
   producer/consumer 間の version 軸とする判断をコード・docs・
   証拠に記録する。
4. **downstream 非破壊**: `ci/tracker`・`ci/summary` 等の既存
   consumer を壊さない（`CONTRACT` export 名・証拠 JSON フィールド
   形状は不変 — 変更は文字列値のみ）。
5. **正直な検証記録**: 実行した検証のみ Pass と記録し、post-merge
   の `consumer-verify` workflow_dispatch 実 run + daily 回復確認
   （Issue AC5）は NotRun（Coordinator 所有の post-merge gate）として
   分離記録する。

## 受入条件（Issue #720 から転記・本 slice の headless 範囲）

- `ci/consumer-verify/` 発行証拠・`ci/daily/decide.mjs` 既定
  `tupleContract`・`daily-ci.yml` の `DAILY_TUPLE_CONTRACT`・
  `consumer-verify.yml` の summary/fallback 記録経路が全て同一の
  v2 contract 文字列に揃う。
- 過去 run の v1 artifact の扱い（hard cutover）がコードと docs に
  明記され、decide.mjs の対応テストで固定される。
- `mise run test-ci-consumer-verify`・`mise run test-ci-daily` が
  pass し、`node --test ci/` 全体で既存 consumer に非破壊。
- `docs/development-environment.md` の証拠契約記述が v2・
  schemaVersion 2・lane Case 項目へ同期される。
- （post-merge gate・本 slice 範囲外）merge 後の実 run で v2 証拠
  artifact 発行と daily `decide-consumer-verify` の contract 一致
  回復 — NotRun として追跡。

## 非目標

- `schemaVersion` 3 以降・`schemaVersion`/`lanes` 語彙自体の変更
  （v2 確定済み）。
- `ci/tracker/`・`ci/summary/` の変更、`mise.toml` への task 追加、
  docs index（`docs/**/README.md`）の編集・再生成。
- consumer-verify lane の固定行（`unsupported`/`not_run` の owner
  注記）の変更 — `scarecrow173/ruviform#629`/`#630`/`#631` の実走
  結果が更新する関係。
- `workflow_dispatch` の実行全般（Coordinator 所有）。
- virtual-first L2/L3 gate の実走・Consumer 個別 OS-L1 受入・
  per-case verdict emitter（各後続 Issue の責務）。
