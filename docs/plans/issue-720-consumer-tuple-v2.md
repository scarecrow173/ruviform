---
id: "PLAN-720tupleV204"
type: "plan"
status: "approved"
title: "Issue #720 consumer-verify-tuple contract v1→v2 bump — 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-720-consumer-tuple-v2.md"
    - "docs/designs/issue-720-consumer-tuple-v2.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-720-consumer-tuple-v2.md"
  implements:
    - "docs/specs/issue-720-consumer-tuple-v2.md"
    - "docs/designs/issue-720-consumer-tuple-v2.md"
  implemented-by:
    - "docs/tasks/issue-720-consumer-tuple-v2.md"
  depends-on:
    - "docs/specs/issue-720-consumer-tuple-v2.md"
    - "docs/designs/issue-720-consumer-tuple-v2.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #720 consumer-verify-tuple contract v1→v2 bump — 実装計画

## フェーズ

1. **現状棚卸しと設計論点の決定**（済）
   - v1 pin 箇所の再照合、(a) hard cutover・(b) schemaVersion/lanes
     未検査維持の決定 → discovery 文書。
2. **producer/consumer/workflow/test/docs の一括 bump**
   - `evidence.mjs` `CONTRACT`、`decide.mjs` 既定値+コメント、両
     workflow の env/fallback/summary/ヘッダ、4 つの contract test、
     `docs/development-environment.md`。
   - decide test に legacy v1 → non-target の hard cutover pin と
     `resolveConfig` 既定 v2 pin を追加。
   - consumer-verify-yml-contract test の stale owner regex を修復
     （main 既存破損・証拠は discovery 記載）。
3. **検証**
   - `mise run test-ci-consumer-verify` / `mise run test-ci-daily` /
     `mise run test-ci-tracker` / `mise exec -- actionlint`（両
     workflow）/ `mise exec -- node --test ci/`（補助・canonical=mise）。
   - 結果を `evidence/rf720/verification.md` に記録。
4. **文書チェーン完成・commit・push・PR**
   - doc chain（spec→design→plan→task→test-spec→discovery→ir→exp）
     完成、Conventional Commits で論理単位に commit、
     `chore/720-consumer-tuple-v2` を push、PR 本文は `Refs #720`。

## ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `ci/consumer-verify/evidence.mjs` | `CONTRACT` v2 + コメント |
| `ci/daily/decide.mjs` | 既定 `tupleContract` v2 + コメント 3 箇所 |
| `.github/workflows/daily-ci.yml` | env + コメント 2 箇所 |
| `.github/workflows/consumer-verify.yml` | コメント + fallback printf×2 + summary |
| `ci/consumer-verify/tests/evidence.test.mjs` | v2 pin ×2 |
| `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` | v2 pin ×2 + stale regex 修復 |
| `ci/daily/tests/decide.test.mjs` | v2 pin ×4 + hard cutover/default pin 追加 |
| `ci/daily/tests/daily-ci-yml-contract.test.mjs` | env pin v2 |
| `docs/development-environment.md` | 証拠契約記述 → v2 + 方針記述 |
| `docs/{discovery,specs,designs,plans,tasks,test-specs,impl}/issue-720-consumer-tuple-v2*` | doc chain（新規） |
| `evidence/rf720/` | 検証記録（新規） |

## リスク

- workflow 共有ファイルの差分競合 → 差分を contract 文字列・コメントに
  限定し lane 行に触れない。merge 前に `git merge origin/main` で最新化。
- main 既存破損（owner regex stale）を本 slice で修復する判断 →
  修復範囲は test regex のみで workflow 行は不変。証拠と共に PR 本文
  へ明記。
- docs index 未同期 → Coordinator が統合時に再生成（本 slice では
  触らない）。
