---
id: "PLAN-034UyQfNa0aSYqwNDMIZn9"
type: "plan"
status: "approved"
title: "Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/284"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/plans/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/tasks/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/test-specs/issue-284-io-02-structured-formats-serde-boundary.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
    - "docs/designs/issue-284-io-02-structured-formats-serde-boundary.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-284-io-02-structured-formats-serde-boundary.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計 Implementation Plan

## Goal

Issue #284（IO-02 Structured Formats — Serde 公開・
JSON/YAML/TOML/CSV/XML Adapter 契約）の合意済み設計
（v0.1・v0.2・2026-11-04・最終整合性レビュー追記）を、
Serde 公開境界・Format/Version 契約・Resource Budget・
Error 契約・Feature/Backend 確定値・Consumer 利用面・
証拠項目対応を含む正本設計書として `docs/` チェーンに
文書化する（docs のみ。コード変更なし）。

## Scope

単一実装ストリーム・1 plan・1 task。変更は `docs/` の
`issue-284-*` 新規文書 5 件と生成 index のみ。
`crates/`・`.github/`・`mise.toml`・`Cargo.*`・既存他
Issue 文書には触れない。per-format の実装詳細・
Consumer 実測・matrix 実走は対象外（実装/受入段階の
別 Issue/dispatch）。

## Upstream Documents

- [docs/specs/issue-284-io-02-structured-formats-serde-boundary.md](../specs/issue-284-io-02-structured-formats-serde-boundary.md)（approved）
- Issue #284 本文（合意済み設計 v0.1・v0.2・2026-11-04・
  最終整合性レビュー追記 — 受入正本）
- Issue #282（親・汎用 I/O 基盤 横断設計正本）

## Design Inputs

- [docs/designs/issue-284-io-02-structured-formats-serde-boundary.md](../designs/issue-284-io-02-structured-formats-serde-boundary.md)（approved）
- 参照正本: `docs/designs/issue-297-io-structured-json-yaml-toml.md`・
  `issue-298-io-structured-csv.md`・`issue-299-io-structured-xml.md`
  （per-format 詳細正本）、`docs/designs/issue-295-…`・
  `issue-296-…`（IO-01 File 契約）、`docs/designs/
  issue-288-…`（cross-consumer gate）、`docs/designs/
  issue-306-…`（合成 E2E）、`docs/designs/issue-409-…`
  （rooted capability）、`crates/ruviform-io-structured/`
  （実装済み crate — 設計の確定値の根拠）。

## File Map

- `docs/specs/issue-284-io-02-structured-formats-serde-boundary.md`
  — spec（何を・なぜ・範囲・受入条件）。
- `docs/designs/issue-284-io-02-structured-formats-serde-boundary.md`
  — IO-02 横断設計正本（Serde 境界・driver/backend 中立・
  Format/Version 契約・Budget 3 層・Error 契約・Feature/
  Backend 確定値・DAG/RT・Consumer 利用面・File/#409 境界・
  証拠項目対応・設計/実装分離）。
- `docs/test-specs/issue-284-io-02-structured-formats-serde-boundary.md`
  — 設計文書の正しさを検証する test-spec（R1–R10）。
- `docs/plans/issue-284-io-02-structured-formats-serde-boundary.md`
  — 本計画。
- `docs/tasks/issue-284-io-02-structured-formats-serde-boundary.md`
  — 単一 task（docs チェーン作成の実施記録）。
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`
  — 生成 index。

## Implementation Sequence

- [x] Issue #284/#282 本文と関連 Issue 状態（#283〜#306・
  #409・#219・#294・AL #139/#143・SV #82/#84/#25）を
  `gh`/`git log` で棚卸し（2026-11-09 snapshot）。
- [x] 既存 IO 文書チェーン（#295/#296・#297/#298/#299・
  #288・#305/#306・#409）と `crates/ruviform-io-structured`
  の実装（src/error.rs・limits.rs・input.rs・file.rs・
  json/yaml/toml/csv/xml.rs・Cargo.toml feature matrix）を
  走査し、参照先の実在を確認。
- [x] spec を作成（Issue #284 AC の dispatch スコープ対応
  付け）。
- [x] design（IO-02 横断設計正本）を作成 — Serde 境界・
  Format/Version 契約・Budget 3 層・Error 契約・Feature/
  Backend 確定値・Consumer 利用面・証拠項目対応。
- [x] test-spec / plan / task を作成。
- [x] docs index を再生成（`doc_maintenance.js apply`）。
- [x] review-protocol によるレビュー（参照実在性・契約
  語彙の整合・証拠項目対応の網羅を確認）。
- [ ] `git fetch origin main && git merge origin/main` で
  最新化し index を再生成・再検証。
- [ ] `mise run fmt-check` + doc-status 系 lint を実行。
- [ ] commit → push → PR 作成（`Refs #284`）。

## Dependencies and Risks

- 参照先の Issue 状態は変動し得る（AL #139/#143 CLOSED、
  SV #82 CLOSED・#84 OPEN・#25 CLOSED — 2026-11-09
  snapshot。SV #82 は #288 記録時の OPEN から変動）。
  本書は snapshot として記録し、接続表の「役割」記述が
  将来古くなる可能性を test-spec Risk に明記済み。
- `docs/specs/issue-219-consumer-compat-contract.md` は
  status `proposed` — 影響分類語彙の参照正本として
  `depends-on` で接続するが、本 slice の承認対象ではない。
- 既存の broken relation（`docs/impl/ir/issue-243-…`・
  `issue-313-…`・`issue-410-…` 等、`route_graph.js` が
  報告）は本タスク範囲外の既存問題であり、doc-status 全体
  lint がこれを報告する可能性がある — 本 PR では修正せず
  証拠として報告する。
- per-format の詳細（buffer 戦略・dialect・namespace 解決・
  fuzz 規則）は #297/#298/#299 が正本。本書は横断契約と
  索引のみを所有し、記述の drift は参照構造で緩和する。

## Task Breakdown

- T1: spec 作成（本 task に包含 — docs チェーンは 1 単位の
  論理変更）。
- T2: design（IO-02 横断設計正本）作成。
- T3: test-spec / plan / task 作成 + index 再生成。
- T4: review-protocol レビューと blocker 解消。
- T5: main 取り込み・最終検証・commit/push/PR。

## Verification

- [ ] `mise run fmt-check` pass。
- [ ] doc-status 系 lint/audit で本 PR 追加分に broken
  relation・未解決参照がないこと（既存の範囲外 finding は
  分離して記録）。
- [ ] 設計文書の参照先（Issue/PR/test-spec/mise task/
  crate/doc パス）が実在することを作成時に確認済みで
  あること。
- [ ] PR diff が `docs/` の新規 `issue-284-*` 文書と生成
  index のみであることを `git status`/`git diff` で確認。

## Open Questions / Gaps

- Consumer 実測・consumer-verify tuple run・macOS/Linux
  実機は実装/受入段階の要件であり本 slice では
  `not run`/`NotRun`/`Unsupported` として記録する（解消
  しない gap ではなく意図的な分離）。
- `LimitNotGuaranteed` 相当の明示的診断面は Deferred
  Design Concerns として設計文書に記録（Consumer の
  programmatic 判別実需待ち）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology
  are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: no（review-protocol
  で代替）
- If approved: discover available implementation and
  delegation capabilities in the current environment, then
  execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- Serde 境界節が「許可型/非公開型/source() のみ/自作
  Serializer・DOM・Registry なし」を漏れなく固定し、
  per-format test-spec の audit 規則への参照を持つか。
- Format/Version 契約節が Consumer 所有の
  schema/version/canonical bytes と Format A→DOM→B 非
  提供を #288・crate README と矛盾なく記述しているか。
- 証拠項目対応表の OWN/CSV/XML/DOC/ERR/COMP・Acceptance
  Gate 各項目が実在する正本へ解決できるか。
- Backend 選定表の版・MSRV・ライセンスが `Cargo.toml` pin
  と per-format 設計に一致するか。
- Consumer 利用面の責務分界（schema は Consumer・単一
  Publish ≠ Project Transaction）が #288 と同一原則か。
