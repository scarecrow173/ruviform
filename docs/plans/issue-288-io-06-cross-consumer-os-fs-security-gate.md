---
id: "PLAN-034UtOFqGK9dkffJMoPRvK"
type: "plan"
status: "approved"
title: "Issue #288 IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/plans/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/tasks/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/test-specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
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
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #288 IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計 Implementation Plan

## Goal

Issue #288（IO-06 Cross-Consumer・OS/FS・Security・性能・互換性
Gate）の合意済み設計 v0.1 を、判定語彙・受入 matrix・否定系
判定表・Owner/境界接続表・Consumer 要件・Breaking Change
追跡導線・証拠導線・設計/実装分離を含む正本設計書として
`docs/` チェーンに文書化する（docs のみ。コード変更なし）。

## Scope

単一実装ストリーム・1 plan・1 task。変更は `docs/` の
`issue-288-*` 新規文書 5 件と生成 index のみ。`crates/`・
`.github/`・`mise.toml`・`Cargo.*`・既存他 Issue 文書には
触れない。matrix の実走・テスト実装・Consumer repo 変更は
対象外（実装段階の別 Issue/dispatch）。

## Upstream Documents

- [docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md](../specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)（approved）
- Issue #288 本文（合意済み設計 v0.1・2026-11-04・最終整合性
  レビュー追記 — 受入正本）
- Issue #282（親・汎用 I/O 基盤 横断設計正本）
- [docs/specs/issue-219-consumer-compat-contract.md](../specs/issue-219-consumer-compat-contract.md)
  （影響分類語彙の正本）

## Design Inputs

- [docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md](../designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)（approved）
- 参照正本: `docs/designs/issue-296-io-file-os-backend.md`（OS/FS
  capability matrix）・`docs/specs|test-specs/issue-29[5-9]-*.md`・
  `issue-30[0-6]-*.md`・`docs/specs/issue-294-cross-consumer-candidate-gate.md`・
  `docs/specs/issue-305-media-io-staged-file-cutover.md`

## File Map

- `docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md`
  — spec（何を・なぜ・範囲・受入条件）。
- `docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md`
  — Gate 設計正本（判定語彙・結果記録フォーマット・OS/FS
  capability 軸・受入 matrix・否定系判定表 5 領域・Owner/
  境界接続表・Consumer 要件・Breaking Change 導線・証拠導線・
  Gate 種別ごとの実行割当て・設計/実装分離）。
- `docs/test-specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md`
  — 設計文書の正しさを検証する test-spec（R1–R10）。
- `docs/plans/issue-288-io-06-cross-consumer-os-fs-security-gate.md`
  — 本計画。
- `docs/tasks/issue-288-io-06-cross-consumer-os-fs-security-gate.md`
  — 単一 task（docs チェーン作成の実施記録）。
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`
  — 生成 index。

## Implementation Sequence

- [x] Issue #288/#287/#282 本文と関連 Issue 状態（#219/#294・
  #295〜#306・AL #139/#140/#141/#143・SV #82/#84/#25）を
  `gh`/`git log` で棚卸し。
- [x] 既存 IO 文書チェーン（spec/design/test-spec）・
  `mise.toml` task・`consumer-verify.yml`/`daily-ci.yml`・
  `crates/` 構成を走査し、参照先の実在を確認。
- [x] spec を作成（Issue #288 AC の dispatch スコープ対応付け）。
- [x] design（Gate 設計正本）を作成 — 判定語彙・結果記録
  フォーマット・OS/FS capability 軸・受入 matrix（F/S/C/A/M/E）・
  否定系判定表 5 領域・Owner/境界接続表・Consumer 要件・
  Breaking Change 導線・証拠導線・実行割当て・設計/実装分離。
- [x] review-protocol によるレビュー（参照実在性・matrix 網羅・
  判定語彙の整合を確認し、結果記録フォーマットと実行割当ての
  2 点を補強）。
- [x] test-spec / plan / task を作成。
- [x] docs index を再生成。
- [ ] `git fetch origin main && git merge origin/main` で最新化し
  index を再生成・再検証。
- [ ] `mise run fmt-check` + doc-status 系 lint を実行。
- [ ] commit → push → PR 作成（`Refs #288`）。

## Dependencies and Risks

- 参照先の Issue 状態は変動し得る（AL #139 系は CLOSED、SV #82/#84
  は OPEN — 2026-11-09 snapshot）。本書は snapshot として記録し、
  接続表の「役割」記述が将来古くなる可能性を test-spec Risk に
  明記済み。
- `docs/specs/issue-219-consumer-compat-contract.md` は status
  `proposed` — 語彙参照の正本として `depends-on`/`related` で
  接続するが、本 slice の承認対象ではない。
- 既存の broken relation（`docs/impl/ir/issue-243-…` 等、
  `route_graph.js` が報告）は本タスク範囲外の既存問題であり、
  doc-status 全体 lint がこれを報告する可能性がある — 本 PR
  では修正せず証拠として報告する。
- matrix の実走は本 slice 対象外。`not run`/`NotRun`/
  `Unsupported` を PASS にしない方針を設計に明記済み。

## Task Breakdown

- T1: spec 作成（本 task に包含 — docs チェーンは 1 単位の
  論理変更）。
- T2: design（Gate 設計正本）作成。
- T3: test-spec / plan / task 作成 + index 再生成。
- T4: review-protocol レビューと blocker 解消。
- T5: main 取り込み・最終検証・commit/push/PR。

## Verification

- [ ] `mise run fmt-check` pass。
- [ ] doc-status 系 lint/audit で本 PR 追加分に broken relation・
  未解決参照がないこと（既存の範囲外 finding は分離して記録）。
- [ ] 設計文書の参照先（Issue 番号・PR 番号・test-spec パス・
  mise task 名・workflow 名・crate 名）が実在することを
  作成時に確認済みであること。
- [ ] PR diff が `docs/` の新規 `issue-288-*` 文書と生成 index
  のみであることを `git status`/`git diff` で確認。

## Open Questions / Gaps

- matrix の実走結果・consumer-verify の候補 run・Consumer 実測・
  macOS/Linux 実機は実装段階の要件であり本 slice では
  `not run`/`NotRun`/`Unsupported` として記録する（解消しない
  gap ではなく意図的な分離）。
- 性能許容差の具体値・Network/Cloud FS の保証範囲は Deferred
  Design Concerns として設計文書に記録（実測・製品要件待ち）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: no（review-protocol で代替）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 受入 matrix の各行が Issue #288 の受入スコープ表の項目を
  漏れなく対応付けているか。
- 判定語彙が #219/#294 の既存契約と矛盾しないか
  （`unsupported`/`not run`/`unknown` の扱い）。
- Owner/境界接続表の Issue 状態 snapshot（2026-11-09）が
  誤解を招かないか。
- 証拠導線が「専用 YAML/新 Dashboard/重複レポートなし」の
  Issue #288 要求を満たすか。
