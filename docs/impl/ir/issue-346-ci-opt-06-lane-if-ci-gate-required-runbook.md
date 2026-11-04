---
id: "IMPL-034RP8X0bpYtK5In9i3F00"
type: "impl"
status: "completed"
title: "Issue #346 CI-OPT-06 選択実行有効化（lane if 配線）・ci-gate 単一 required 移行 runbook"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/346"
  changes:
    added:
      - type: "file"
        path: "ci/impact/tests/workflow.test.mjs"
      - type: "file"
        path: "docs/designs/issue-346-ruleset-migration-runbook.md"
      - type: "file"
        path: "docs/impl/ir/issue-346-ci-opt-06-lane-if-ci-gate-required-runbook.md"
      - type: "file"
        path: "docs/impl/exp/issue-346-selective-enforcement.md"
      - type: "file"
        path: "docs/impl/exp/data/issue-346-enforcement-before-runs.jsonl"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/designs/issue-333-impact-based-ci.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-346-ruleset-migration-runbook.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-333-impact-based-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-333-impact-based-ci.md"
    - "docs/designs/issue-346-ruleset-migration-runbook.md"
    - "docs/impl/ir/issue-333-ci-opt-06-impact-gate.md"
    - "docs/impl/exp/issue-346-selective-enforcement.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #346 CI-OPT-06 選択実行有効化（lane if 配線）・ci-gate 単一 required 移行 runbook

## Summary

issue-333 設計の段階移行 runbook Stage C/D を実行し、変更影響ベース選択実行を
**有効化**した。6 つの検証 lane job（`verify`・`rt-verify`・`icc-verify`・
`ui-verify`・`l1-real-verify`・`bench`）に `needs: [impact]` と
`if: always() && needs.impact.outputs.lane_* != 'false'` を配線し、
classifier が「影響なしを証明した」lane のみ skip する。skip される lane の
required context 報告が止まる deadlock を不可分対処として、`ci-gate` を唯一の
安定 required context とする ruleset 移行 runbook
（`docs/designs/issue-346-ruleset-migration-runbook.md`）を新設し、
`if: always()` の gate が skipped を Pass にしない fail-closed 照合は
Shadow Mode 時代の `gate.mjs` そのままに成立する。`push`/`workflow_dispatch`
は classifier の `FULL_EVENTS` で常に expected=ALL（Full CI 維持）。

## Context

Issue #346（CI-OPT-06 follow-up・#333 の残課題）。macOS arm64 runner 逼迫で
required lane が queue→cancel される実害を受け、owner 判断で Shadow 観測期を
前倒しして有効化へ進む。Shadow 実績: PR #347/#348 で selective 判定・偽陰性 0
を 2 件観測（`docs/impl/exp/data/issue-334-*`・`issue-335-*` jsonl）。ruleset
`main-protect`（id `26529830`）の移行操作自体は coordinator が `gh api` で
実施するため、worker はコード配線と runbook 化を担当する。

## Implementation

### Changes

- `.github/workflows/ci.yml`:
  - 6 lane job に `needs: [impact]` +
    `if: always() && needs.impact.outputs.lane_<id> != 'false'` を追加
    （`rt_verify`/`icc_verify`/`ui_verify`/`l1_real_verify`/`bench`/`verify`
    — output 名の hyphen→underscore 変換は classify.mjs の GITHUB_OUTPUT
    emit 規約に一致）。
  - `impact` job の Shadow Mode 記述 comment を enforcement 記述へ更新。
  - `ci-gate` job の非 required 記述 comment を「単一 required context として
    ruleset 移行 runbook に従い昇格」記述へ更新。`needs`/`if: always()` は
    無変更（skipped lane を `needs.*.result == "skipped"` として照合する
    既存ロジックが enforcement でも同一に正しい）。
- `ci/impact/tests/workflow.test.mjs` 新設: lane→impact の `needs`/`if:`
  配線・`impact` の無条件実行・`ci-gate` の `if: always()` + needs 全集約を
  構造 contract test として pin（`mise run test-ci-impact` で 33 件へ拡大）。
- `docs/designs/issue-346-ruleset-migration-runbook.md` 新設: ruleset
  `main-protect` required context を `12 lane context → ci-gate 単一` へ
  「追加→報告確認→除去」で無保護期間なく移行する操作 runbook + 移行中
  deadlock 扱い + 監視規則 + rollback。
- `docs/impl/exp/issue-346-selective-enforcement.md` + `data/issue-346-enforcement-before-runs.jsonl`:
  Before（Shadow 全実行）計測の記録と After 未実測の明示。
- `AGENTS.md`: `ci.yml` 説明文を Shadow Mode から enforcement 記述へ同期、
  `docs/designs/README.md`（generated index）・`docs/impl/ir/README.md`・
  `docs/impl/exp/README.md` 更新。

### Decisions During Implementation

- `if:` は `always() && outputs.lane_* != 'false'` — `impact` job 失敗・出力
  欠落時は出力が空になり lane が**実行される側**に倒す（fail-open coverage）。
  fail-closed 性は gate の classification-error Fail 側に保持する設計どおり。
- required context の移行対象は `ci-gate` 単一 — `impact-classify` を別途
  required にしない（その失敗は gate の classification-error で Fail に
  なるため gate 1 本で coverage・分類健全性の両方を cover できる）。
- `docs/designs/issue-333-impact-based-ci.md` 自体は変更しない — Stage C/D
  手順の高水準版は既に同書にあり、操作 runbook は新規 design doc として
  分離（approved 文書への追記を避け、issue→doc の対応を明確化）。
- ruleset 移行自体は本 PR には含めない（workflow 外の admin 操作）。
  移行中 deadlock 窓（Phase 2 merge〜Phase 3 完了）の扱いを runbook に明記。

## Related Experiments

### Adopted

- Shadow 実績（PR #347 run 41840536142・PR #348 run 41848239157）:
  `decision=selective`・`falseNegatives=0`・not-expected lane が全 pass で
  over-executed 監査として記録済み（`docs/impl/exp/data/issue-334-*`・
  `issue-335-*` jsonl）。
- Before 計測（2026-11-05 収集・`ci/metrics/collect-ci-metrics.mjs
  --runs 15`）: wall p50 307s / p95 497s・代表 run critical path 235s・
  runner_total 1474s・macOS queue 滞留（push run 41851785028 queued
  2279s+）を実測記録。raw: `docs/impl/exp/data/issue-346-enforcement-before-runs.jsonl`。

### Rejected

- lane 内 step 条件付き skip（job は常に success）: 「何も検証していない
  成功」を required context が緑として報告する形骸化（issue-333 design が
  既に不採用とした案 — 同じ理由で踏襲）。
- `if:` なしの lane + artifact 参照による分岐: job `if:` + `needs` output の
  方が宣言的で gate の `needs.result` 照合と一致するため採用せず。
- ruleset の同時置換（12 除去 + ci-gate 追加を 1 操作）: `ci-gate` が全 PR で
  報告される実績確認前に単一化すると、報告経路の不具合時に無保護になる
  リスクがあるため「追加→確認→除去」の 2 段階を採用。

## Validation

実測環境（ローカル）: Windows 11 x64 / Git Bash、mise 管理 toolchain
（node 26.12.1）。

| 項目 | 実行 | 結果 |
| --- | --- | --- |
| `mise run test-ci-impact` | `node --test ci/impact/` | Pass — 33/33（classify 19 + gate 11 + workflow 3） |
| `mise exec -- actionlint .github/workflows/ci.yml` | — | 実施結果は PR 本文の検証証拠欄を参照 |
| `prek run`（変更ファイル全 hook） | commit 時フック | 同上 |
| classifier E2E（本 PR 差分） | `.github/**`・`ci/**` → `decision=full`・6 lane expected | 同上 |
| 本 PR の CI run | 13 context（12 lane + `ci-gate`）全緑 | CI 上で確認（PR checks 参照） |

## Risks

- 移行中 deadlock 窓（Phase 2 merge 〜 Phase 3 完了）: selective PR が旧 12
  context を報告せず merge 不能になる。runbook に最小化・復旧手順を明記済み。
- `impact` job が critical path に直列化するため全 run の wall が +~20-25s
  増える（Shadow 期は並列で critical path 外）。見返りは selective PR の
  runner_total 削減と macOS queue 滞留の回避。
- 偽陰性の残存リスク: Shadow 実績は 2 PR（selective）のみで蓄積が薄い。
  gate-report の監視規則で継続検出し、検出時は `CI_IMPACT_DISABLE=1` で
  即時 Full CI に退避する。

## Follow-ups

- ruleset `main-protect` の Phase 1〜3 を coordinator が `gh api` で実施
  （runbook どおり「ci-gate 追加 → 報告確認 → 12 除去」）。
- selective 実動証（docs-only PR で NotApplicable 判定）を merge 後の
  後続 PR で確認し `docs/impl/exp/issue-346-selective-enforcement.md` に記録。
- After メトリクス（wall/runner_total/lane skip 率）を同一収集手順で実測し
  Before と比較。
