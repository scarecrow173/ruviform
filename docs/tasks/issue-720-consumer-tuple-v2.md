---
id: "TASK-720tupleV205"
type: "task"
status: "done"
title: "Issue #720 consumer-verify-tuple contract v1→v2 bump — task"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-720-consumer-tuple-v2.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-720-consumer-tuple-v2.md"
  implements:
    - "docs/plans/issue-720-consumer-tuple-v2.md"
  implemented-by:
    - "docs/impl/ir/issue-720-consumer-tuple-v2.md"
  depends-on:
    - "docs/plans/issue-720-consumer-tuple-v2.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #720 consumer-verify-tuple contract v1→v2 bump — task

- [x] v1 pin 箇所の棚卸しと設計論点決定（hard cutover・schemaVersion/lanes 未検査維持）→ discovery
- [x] `ci/consumer-verify/evidence.mjs` `CONTRACT` → v2
- [x] `ci/daily/decide.mjs` 既定 `tupleContract` → v2 + 方針コメント
- [x] `.github/workflows/daily-ci.yml` env・コメント → v2
- [x] `.github/workflows/consumer-verify.yml` fallback printf×2・summary・ヘッダ → v2
- [x] contract test 4 ファイルの v2 pin + hard cutover/default pin 追加
- [x] consumer-verify-yml-contract test の stale owner regex 修復（main 既存破損）
- [x] `docs/development-environment.md` 証拠契約記述 → v2 + 方針記述
- [x] 正規検証コマンド一式の実行と exit code 記録（`evidence/rf720/`）
- [x] doc chain 完成・commit・push・PR（Refs #720）
