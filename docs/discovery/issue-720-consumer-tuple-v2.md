---
id: "DSC-720tupleV201"
type: "discovery"
status: "final"
title: "Issue #720 consumer-verify-tuple contract v1→v2 bump — 現状 pin 箇所の棚卸しと設計論点の調査"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/720"
    - "evidence/rf294/verification.md"
    - "evidence/rf294/ac-inventory.md"
  related:
    - "../specs/issue-720-consumer-tuple-v2.md"
---

# Issue #720 consumer-verify-tuple contract v1→v2 bump — 調査記録

`consumer-verify-tuple` contract を v1 から v2 へ bump するにあたり、
producer（`ci/consumer-verify/evidence.mjs`）・consumer
（`ci/daily/decide.mjs`）・両 workflow・contract test・docs の現状 pin
箇所を棚卸しし、Issue 本文「設計論点」の 2 件を調査した。

## v1 pin 箇所の棚卸し（main `7124f0b2` 時点で再照合）

| 箇所 | 内容 |
| --- | --- |
| `ci/consumer-verify/evidence.mjs:63` | `export const CONTRACT = "consumer-verify-tuple/v1"`（証拠 JSON の `contract`/`tuple.contract` 両フィールドの供給源） |
| `ci/consumer-verify/tests/evidence.test.mjs:109-111`,`:434` | `CONTRACT` export と証拠 JSON `contract` の v1 pin |
| `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs:428`,`:503` | summary 文面・fallback JSON の v1 pin |
| `ci/daily/decide.mjs:40`,`:59`,`:1424` | tuple mode ヘッダコメント・usage・`tupleContract` 既定値（`--tuple-contract`/`DAILY_TUPLE_CONTRACT` で上書き可） |
| `ci/daily/tests/decide.test.mjs:780`,`:791`,`:1024`,`:1396` | tuple 説明コメント・`CONTRACT` 定数・contract 不一致 fixture・report 文面 regex |
| `ci/daily/tests/daily-ci-yml-contract.test.mjs:59` | `DAILY_TUPLE_CONTRACT` env pin |
| `.github/workflows/daily-ci.yml:24`,`:110`,`:148` | ヘッダコメント・job コメント・`DAILY_TUPLE_CONTRACT` env |
| `.github/workflows/consumer-verify.yml:12`,`:709`,`:1250`,`:1287` | ヘッダコメント・AL/SV fallback `printf`・summary echo |
| `docs/development-environment.md:100`,`:134`,`:135` | 証拠契約記述 3 箇所 |

`schemaVersion: 2`・Case schema v2 の lane record 語彙は rf294 first
slice（`c6e4f334`）で確定済みであり、本 bump は `contract` 文字列のみを
対象とする（`schemaVersion`/`lanes` の語彙自体は変更しない）。

## 設計論点 (a): 過去 run の v1 artifact の扱い

`classifyRunEvidence`（`ci/daily/decide.mjs`）は証拠 JSON の `contract`
文字列を候補 tuple の `contract` と**完全一致**で照合し、不一致は
proven different tuple として `non-target`（寄与なし・retry 計上なし）
に分類する。したがって:

- **hard cutover**: 既定値を v2 に切り替えるだけで、v1 artifact は
  自動的に `non-target` になる。コード変更は既定値 1 箇所＋文書・test
  の pin 更新のみ。
- **両受理の移行窓**: `contract` を受理集合（v1 OR v2）に拡張する判定
  変更が必要。v1 run が v2 候補 tuple を「covering」し得るが、tuple の
  残要素（ruviform SHA・consumer main SHA）は日々動くため、実際に
  両受理が意味を持つのは merge 直後の極小 window のみ。

**決定: hard cutover**。理由: (1) contract 文字列は tuple 同一性の一
要素であり、bump の目的は「証拠語彙の拡張を tuple 識別に反映する」
こと — 旧 contract の証拠を新 tuple の検証済みとして受理することは
bump の意味を損なう。(2) KISS/YAGNI — 受理集合の導入は仮想的な移行
要件のための複雑化。(3) fail-closed 側に倒れる — v1 artifact は
`non-target`（無視ではなく proven different）で、`unverifiable` の
retry 保守計上にも `verified` にもならない。(4) `--tuple-contract` /
`DAILY_TUPLE_CONTRACT` の上書き経路が既に残るため、将来の contract
変更時も同じ規則で運用できる。

## 設計論点 (b): `schemaVersion`/`lanes` 未検査の現状維持

`decide.mjs` は証拠 JSON の `kind`/`contract`/`run.runId`/
`consumer.{name,repo}`/`status`/`tuple.{ruviformRev,
consumerResolvedSha}` を検査するが、`schemaVersion` と `lanes` 配列は
読まない（`evidence/rf294/verification.md` §5 の判断メモで固定済み）。

**決定: 現状維持**。理由: consumer が必要とするのは tuple 同一性と
consumer status であり、証拠内部語彙（`schemaVersion`・lane record
schema）は producer 側の契約 — `evidence.mjs` の `assertLaneRecord` が
finalize 時に fail-closed で検査済み。producer/consumer 間の version
軸は `contract` 文字列であり、証拠語彙の非互換変更が起きた場合は
contract bump で通知する規律を本 Issue が確立する。consumer が
`schemaVersion`/`lanes` を読み始めるのは、consumer がそれらを解釈
する要件が生じた時点の別 Issue とする。

## 既知の関連事項（本 slice で発見・対処）

`mise run test-ci-consumer-verify` が main（`7124f0b2`）時点で 2 件
fail していた: `consumer-verify-yml-contract.test.mjs:231` の
consumer-side GUI contract 行 regex が `#1(66|75)` を pin するが、
`70b48304`（rf294 close-out の stale owner 修正）で workflow 側の
owner 注記が `AL #166/#127`→`AL #127`、`SV #175-177`→`SV #176/#177`
へ更新され test 側が未追従。base worktree（HEAD `7124f0b2`）で同
test を実行し fail を再現確認済み — 本 slice 起因ではない main の
既存破損。regex が pin すべき契約は「owner 帰属を持つ not_run 行」
であり issue 番号自体ではないため、`#[0-9]+` への修正で契約を維持
したまま stale 化を解消した（workflow の lane 行・owner 注記自体は
不変 — #629/#630/#631 の実走更新に委ねる）。

## lease 外の残存 v1 言及（legacy・明示分離）

- `crates/ruviform-ui-test/src/driver_lab.rs:16` — doc comment が
  `consumer-verify-tuple/v1` を参照（lane verdict 語彙の説明文脈。
  `crates/` は本 slice の編集範囲外のため未変更 — 後続で追従可）。
- `docs/{designs,discovery,impl,plans,specs,tasks,test-specs}/
  issue-294-*`・`issue-219-*`・`issue-650-*` — 履歴文書の当時の記述
  （変更しない）。
- `evidence/rf288/`・`evidence/rf294/`・`evidence/rf628/`・
  `evidence/rf650/` — 過去 run の証拠・検証記録（変更しない）。
