---
id: "PLAN-034YYuiRq03qc7utzYExKJ"
type: "plan"
status: "approved"
title: "Issue #721 post-seal temp 境界確定 — implementation plan"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/721"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/tasks/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/test-specs/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/impl/ir/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/impl/exp/issue-721-post-seal-temp.md"
      - type: "file"
        path: "docs/impl/exp/issue-721-post-seal-temp-exp.jsonl"
      - type: "file"
        path: "evidence/rf721/"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-721-post-seal-temp.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-721-post-seal-temp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-650-bench-smoke-coverage.md"
    - "docs/plans/issue-306-io-e2e-cases.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-721-post-seal-temp.md"
    - "docs/designs/issue-721-post-seal-temp.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-721-post-seal-temp.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/721"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/650"
  defers: []
  deferred-by: []
metadata:
  issue: 721
---

# Issue #721 post-seal temp 境界確定 — implementation plan

## Goal

post-seal temp 改竄境界の設計判断（現行契約維持）を設計文書・
README 補強・実測証拠として確定し、bench-smoke lane の
`ruviform-io-e2e` 配線を実測で照合する。io-file/io-e2e の
実装コードに差分を生じない doc + 検証 slice。

## Scope

単一 implementation stream。内部文書チェーン + README 1 項目 +
evidence + 検証実測のみで、並列分割の必要はない。

## Upstream Documents

- `docs/specs/issue-721-post-seal-temp.md`

## Design Inputs

- `docs/designs/issue-721-post-seal-temp.md`（approved — 決定 1〜4）

## File Map

- `docs/specs/issue-721-post-seal-temp.md` — 要求・受入条件・
  deferred concern の正本。
- `docs/designs/issue-721-post-seal-temp.md` — 設計判断の正本
  （現行契約維持・強化案却下理由・consumer 照合）。
- `docs/plans/issue-721-post-seal-temp.md` — 本文書。
- `docs/tasks/issue-721-post-seal-temp.md` — 実施 task 記録。
- `docs/test-specs/issue-721-post-seal-temp.md` — 契約 pin・
  回帰・bench-smoke 実測の検証規範。
- `crates/ruviform-io-file/README.md` — §信頼境界・保証範囲へ
  post-seal 再検証なしの 1 項目を追加（doc のみ）。
- `evidence/rf721/` — bench-smoke 実行抜粋・検証照合記録。
- `docs/impl/ir/issue-721-post-seal-temp.md`・
  `docs/impl/exp/issue-721-post-seal-temp.md` +
  `issue-721-post-seal-temp-exp.jsonl` — 実装記録・実験記録。

## Implementation Sequence

- [x] consumer 利用実績の照合（audio-lens `al-job`/`al-media`・
      synvelis `synvelis-library`/`synvelis-player` — 全て同一
      信頼境界内の連続 pipeline、read-only 参照）
- [x] 現行契約の根拠確認（`staged.rs` seal/publish doc・README
      §信頼境界・`post_seal_temp_*` 試験・#306 design Case 1/2）
- [x] `mise run bench-smoke` 実測 → `io_composition` 6 case
      実行・exit 0 を `evidence/rf721/` に記録（残件 a）
- [x] `docs/development-environment.md` bench 系記述 ↔
      `mise.toml` `[tasks.bench]`/`[tasks.bench-smoke]` の
      read-only 照合（一致・drift なし）
- [x] spec/design/test-spec/plan/task/impl/exp の文書チェーン完成
- [x] `crates/ruviform-io-file/README.md` §信頼境界へ
      post-seal 再検証なしの最小項目を追加
- [x] `mise run test ruviform-io-file` / `mise run test-io-e2e` /
      `mise run fmt-check` 等の比例検証を実測（全て exit 0）
- [x] review-protocol self-review → blocker 解消
- [ ] commit → push → PR → worker_done

## Dependencies and Risks

- `docs/development-environment.md` は `scarecrow173/ruviform#720`
  の並行 Dispatch が占有中 — 本 slice は read-only 照合のみ。
  drift が判明した場合は `orca orchestration ask` で相談し、
  自分では編集しない。
- `mise.toml` の bench-smoke 配線は PR #651 で main 着地済み —
  未配線が残っていた場合のみ `ask` で占有拡張を依頼する。
- docs index（`docs/*/README.md`）は Coordinator が統合時に
  再生成 — 本 slice では一切触れない（index 未同期は許容済み）。
- `*-msrv`・`*-full`・3 OS 横断 lane は本 slice の必須検証に
  含めない（NotRun — Daily Full CI 委譲）。

## Task Breakdown

- `docs/tasks/issue-721-post-seal-temp.md`（単一 task）。

## Verification

- [x] `mise run bench-smoke` → exit 0、`io_composition` 6 case
      （`staged_write`/`staged_seal`/`staged_publish`/
      `staged_readback`/`tar_gzip_staged`/`tar_zstd_staged`）が
      `Testing ... Success`（`evidence/rf721/` に抜粋記録）
- [x] dev-env doc ↔ `mise.toml` bench 系記述の一致照合
      （read-only・`evidence/rf721/verification.md`）
- [x] `mise run test ruviform-io-file` → exit 0（Issue AC3）
- [x] `mise run test-io-e2e` → exit 0（Issue AC3 — `post_seal_temp_*`
      を含む既存 suite 回帰なし）
- [x] `mise run fmt-check` → exit 0（doc/Markdown 変更の軽量
      チェック）
- [x] `mise run check ruviform-io-file` / `lint ruviform-io-file` /
      `check-io-e2e` / `lint-io-e2e` → 全て exit 0（比例検証として
      実施 — test lane が主だが check/lint も canonical 指定のため実測）
- [x] review-protocol self-review で blocker なし
- [ ] `*-msrv`・`*-full`・3 OS 横断 — 本 slice の明示対象外
      （NotRun）

## Open Questions / Gaps

- なし。設計判断（現行契約維持）は consumer 照合・既存信頼境界
  宣言・強化案の費用対効果に基づき確定済み。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 task・doc+検証中心）
- Independent work streams: none
- User approval required before dispatch: no（coordinator 確定の
  受入条件に基づく是正タスク — review-protocol self-review を
  承認代替として実施）

## Review Handoff

- reviewer focus: 設計判断「現行契約維持」の根拠が consumer
  実績・README 信頼境界宣言・#306 pin と整合するか。強化案の
  却下理由が YAGNI/信頼境界の観点で妥当か。bench-smoke 実測
  証拠が `io_composition` 6 case 全てをカバーするか。
- docs index 未同期は意図的（Coordinator 統合時に再生成）。
  `docs/development-environment.md` 非編集は #720 占有への
  準拠であり drift 未検出を意味しない（照合記録を参照）。
