---
id: "PLAN-YgAwqghg3ANT8uWJE1Y1t2"
type: "plan"
status: "approved"
title: "Issue #285 IO-03 Compression — Serde境界・Format/Version契約・Consumer利用面・証拠項目対応 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/285"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/specs/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/plans/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/tasks/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/test-specs/issue-285-io-03-compression-serde-boundary.md"
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
    - "docs/specs/issue-285-io-03-compression-serde-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-285-io-03-compression-serde-boundary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-285-io-03-compression-serde-boundary.md"
    - "docs/designs/issue-285-io-03-compression-serde-boundary.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-285-io-03-compression-serde-boundary.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #285 IO-03 Compression — Serde境界・Format/Version契約・Consumer利用面・証拠項目対応 設計

## Purpose

Issue #285 の spec・design・test-spec を作成し、docs チェーン
を task 記録まで完結させる docs 専用 slice の作業順序を定める。
対象 Issue は
[#285](https://github.com/scarecrow173/ruviform/issues/285)
（IO-03 Compression Serde 境界設計・親 #282）。コード変更・
テスト追加・CI 変更・mise 設定変更は本 plan の対象外である。

## Task List

- [ ] **T1. Spec の作成** —
  `docs/specs/issue-285-io-03-compression-serde-boundary.md`
  を作成する。Issue #285 本文の受入条件・合意済み設計
  v0.1/v0.2・IO-IMP-06/07 引継ぎを要求として転記し、
  Scope/Non-goals/依存関係・受入条件を明記する。
  - Output: spec 文書（status: draft → review 後 approved）。
  - Verify: spec が Issue #285 の全要件（Serde 境界・
    format/version・consumer 面・証拠項目対応）を要求
    として含むこと。**完了**

- [ ] **T2. 実装正本の調査** —
  `crates/ruviform-io-compression` の `Cargo.toml`・
  `src/{lib,error,options,receipt,stream,gzip,deflate,zstd}.rs`・
  `tests/`・crate README を読み、確定した API 形・Error
  種別・budget・feature/Backend を確定値として記録する。
  `docs/designs/issue-300-…`・`issue-301-…`（per-format
  正本）、`issue-295/296`（IO-01）、`issue-288`（gate）、
  `issue-306`（E2E）、`issue-409`（rooted）、`issue-30{2,3,4}`
  （archive）を消費する。実装の現状（main 上に既存・
  #300 PR #356・#301 PR #359 merge 済み）を design の
  Context に記録する。
  - Output: 設計の根拠となる事実リスト。
  - Verify: 設計で使う型名・variant 名・feature 名・
    Issue/PR 番号・test 名が実在すること。**完了**

- [ ] **T3. Design の作成** —
  `docs/designs/issue-285-io-03-compression-serde-boundary.md`
  を作成する。Serde 境界・Format/Version 契約・Finish/状態
  機械・Resource Budget・Error 契約・Feature/Backend 確定値・
  Consumer 利用面・証拠項目対応（ENC/DEC/SEC/INT/ARC/COMP/
  BUF → per-format test-spec・#288 matrix・実装 Issue/PR）・
  設計/実装分離を記述する。per-format 詳細は #300/#301 への
  参照で構成し、複写による drift を避ける。
  - Output: design 文書（status: approved）。
  - Verify: R1〜R9 の規則を満たす（test-spec 参照）。
    **完了**

- [ ] **T4. Test-spec の作成** —
  `docs/test-specs/issue-285-io-03-compression-serde-boundary.md`
  を作成する。docs 文書の正しさを検証する規則（R1〜R11:
  Serde 境界完全性・Format/Version 契約・参照実在性・
  Finish 状態機械・Resource Budget・Error 契約・
  Feature/Backend 確定値・Consumer 利用面・証拠項目対応・
  設計/実装分離・変更範囲）を定義する。
  - Output: test-spec 文書。
  - Verify: 各 rule が検査可能な文書検査として記述され、
    Non-goals が per-format 実走・Consumer 実測を除外
    していること。**完了**

- [ ] **T5. Plan・Task の作成** —
  `docs/plans/issue-285-io-03-compression-serde-boundary.md`
  と `docs/tasks/issue-285-io-03-compression-serde-boundary.md`
  を作成し、チェーンの前後関係を接続する。
  - Output: plan・task 文書。
  - Verify: task が spec/design/test-spec への依存を持ち、
    plan が本 task list を記録すること。**完了**

- [ ] **T6. Index 再生成と検証** —
  `.agents/skills/doc-maintenance/scripts/doc_maintenance.js apply`
  で docs index を再生成し、`mise run fmt-check` と
  `audit_docs`（`node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js`
  等の文書検査入口）を実行する。新規 #285 文書に error 0
  を確認し、pre-existing の broken relation（#243/#313/
  #410 等）を新規文書と分離して記録する。
  - Output: 更新された index・検証結果。
  - Verify: 新規文書の audit error 0・差分が `docs/` のみ。
    **完了**

- [ ] **T7. 最終統合検証** —
  `git fetch origin main && git merge origin/main` で最新
  main を取り込み、index 再生成・`mise run fmt-check`・
  audit を再実行する。
  - Output: 最終検証結果。
  - Verify: 生成物の再生成後も検証が通ること。**完了**

- [ ] **T8. Review・Commit・Push・PR** —
  review-protocol に従い blocker なしを確認し、Conventional
  Commit で commit、`feature/285-io03-compression` を push、
  日本語 PR を作成する（要約・変更点・検証結果・NotRun/
  Unsupported・review 結果・`Refs #285`）。
  - Output: PR URL・head SHA。
  - Verify: PR 本文が要件を満たすこと。**完了**

- [ ] **T9. 完了報告** —
  `worker_done` を 1 回送信する（outcome・PR URL・head
  SHA・検証要約）。
  - Output: 完了通知。
  - Verify: 送信確認。**完了**

## Progress

- [x] T1: spec 作成 — `docs/specs/issue-285-io-03-compression-serde-boundary.md`（SPEC-7BYANBc86b8qnIn77CPmoE）
- [x] T2: 実装正本調査 — `Cargo.toml`/`src/`/`tests/`/per-format docs/#288/#306/#409 を消費
- [x] T3: design 作成 — `docs/designs/issue-285-io-03-compression-serde-boundary.md`（DESIGN-TUrjxcJLe7rvYr6up8mhf3）
- [x] T4: test-spec 作成 — `docs/test-specs/issue-285-io-03-compression-serde-boundary.md`（TSPEC-ZpVwcAYhu5UaIR5HfPUNAb）
- [x] T5: plan/task 作成 — `docs/plans/…`（PLAN-YgAwqghg3ANT8uWJE1Y1t2）・`docs/tasks/…`（TASK-5IbSvHOaqP5LVXNKwJaonI）
- [ ] T6: index 再生成と検証
- [ ] T7: 最終統合検証（origin/main merge → 再生成 → 再検証）
- [ ] T8: review・commit・push・PR
- [ ] T9: worker_done

## Completion Criteria

- `docs/specs`/`designs`/`test-specs`/`plans`/`tasks` に
  Issue #285 の `issue-285-io-03-compression-serde-boundary.md`
  が存在し、相互の relations が解決する。
- docs index（5 つの README）が再生成されている。
- `mise run fmt-check` が通る。
- `audit_docs` で新規 #285 文書の error が 0（pre-existing
  の broken relation は別途記録）。
- 差分が `docs/` のみで、`crates/`/`Cargo.toml`/`Cargo.lock`/
  `mise.toml`/`.github/` に変更がない。
- PR が日本語で作成され、`Refs #285` を含み、検証結果と
  NotRun/Unsupported を正直に記録している。
