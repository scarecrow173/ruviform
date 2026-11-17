---
id: "DESIGN-720tupleV203"
type: "design"
status: "approved"
title: "Issue #720 consumer-verify-tuple contract v1→v2 bump — 設計"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-720-consumer-tuple-v2.md"
    - "docs/discovery/issue-720-consumer-tuple-v2.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-720-consumer-tuple-v2.md"
  implements:
    - "docs/specs/issue-720-consumer-tuple-v2.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-720-consumer-tuple-v2.md"
  blocks:
    - "docs/plans/issue-720-consumer-tuple-v2.md"
  supersedes: []
  superseded-by: []
---

# Issue #720 consumer-verify-tuple contract v1→v2 bump — 設計

## 方針

contract 文字列 `consumer-verify-tuple/v1` → `consumer-verify-tuple/v2`
の一括 bump。証拠 JSON のフィールド形状・`schemaVersion: 2`・Case
schema v2 lane 語彙は不変 — 変更は contract 識別文字列と、それを
pin する test/docs のみ。producer と consumer を同一変更で更新し、
main 上で v1 pin の混在を残さない（片側だけ変えると daily が証拠
artifact を回復できず fail-closed 側に倒れて dispatch が止まる）。

## 設計論点の決定

### (a) v1 artifact の扱い — hard cutover

`classifyRunEvidence` は contract 文字列の完全一致で covering /
non-target / unverifiable を分類する既存ロジックをそのまま使う。
既定 `tupleContract` を v2 に変えると、v1 証拠 artifact は他の
tuple 要素が全て一致しても `non-target`（proven different tuple —
寄与なし・retry 計上なし・verified にならない）となる。

両受理窓を設けない理由は discovery §設計論点(a) の通り: bump の
目的は証拠語彙拡張の tuple 識別への反映であり、旧 contract の証拠を
新 tuple の検証済みとして受理することは目的に反する。受理集合の
導入は仮想要件への複雑化（YAGNI）であり、`--tuple-contract` /
`DAILY_TUPLE_CONTRACT` の override 経路が escape hatch として残る。

### (b) `schemaVersion`/`lanes` 未検査 — 現状維持

consumer（`decide.mjs`）は証拠の `kind`/`contract`/`run.runId`/
`consumer.{name,repo}`/`status`/`tuple.{ruviformRev,
consumerResolvedSha}` を検査し、`schemaVersion`/`lanes` は読まない
現状を維持する。証拠内部語彙の検証は producer の
`assertLaneRecord`（finalize 時 fail-closed）が担い、consumer が
必要とするのは tuple 同一性と consumer status のみ。証拠語彙の
非互換変更は contract bump で consumer へ通知する規律とし、
consumer 側で `schemaVersion`/`lanes` を読み始めるのは解釈要件が
生じた時点の別 Issue とする。

## 変更マップ

| ファイル | 変更 |
| --- | --- |
| `ci/consumer-verify/evidence.mjs` | `CONTRACT` → v2 + bump 経緯コメント |
| `ci/daily/decide.mjs` | 既定 `tupleContract` → v2、ヘッダ/usage コメント → v2、hard cutover・未検査維持の判断をコメント記録 |
| `.github/workflows/daily-ci.yml` | `DAILY_TUPLE_CONTRACT` env → v2、ヘッダ/job コメント → v2 |
| `.github/workflows/consumer-verify.yml` | ヘッダコメント・AL/SV fallback `printf`・summary echo → v2 |
| `ci/consumer-verify/tests/evidence.test.mjs` | CONTRACT/証拠 JSON の v2 pin |
| `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` | summary/fallback の v2 pin + consumer-side owner regex の stale 修復（`#1(66|75)`→`#[0-9]+`、main 既存破損・discovery §既知の関連事項） |
| `ci/daily/tests/decide.test.mjs` | `CONTRACT` 定数 → v2、tuple コメント → v2、report regex → v2、legacy v1 → non-target の hard cutover pin 追加、`resolveConfig` 既定 v2 pin 追加 |
| `ci/daily/tests/daily-ci-yml-contract.test.mjs` | `DAILY_TUPLE_CONTRACT` env pin → v2 |
| `docs/development-environment.md` | 証拠契約記述 3 箇所 → v2 + hard cutover・未検査維持の記述追加 |
| `evidence/rf720/` | 検証記録・legacy v1 言及の明示分離 |

## リスクと対応

- **merge 前後の skew**: 本 branch と main の両側で producer/consumer/
  workflow/test/docs を同時更新するため、merge 後の main では v1 pin
  が残らない。merge 前の branch 上で workflow を dispatch しても
  本 PR の workflow 定義（v2）が使われるため整合する。
- **日次 decide への影響**: merge 直後の `decide-consumer-verify` は
  既存 v1 run を `non-target` として読み飛ばし、v2 run が無ければ
  通常どおり dispatch 判定へ進む — 停止も誤 verified も起きない。
- **fallback 経路の取り残し**: `evidence_finalize_error` の最小証拠
  `printf` も v2 に揃える — fallback artifact が旧 contract のままだと
  finalize 失敗時の証拠だけが non-target 化し、失敗の記録が
  tuple 照合からこぼれる。
- **共有 workflow の差分競合**: `.github/workflows/` は #629/#630/
  #631 の lane 行更新と同一ファイルを触り得る。差分は contract 文字列
  とコメントに限定し、lane 行・owner 注記には触れない。
