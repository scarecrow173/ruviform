---
id: "TASK-034udQraUdxLs1SxuYiaAK"
type: "task"
status: "in-progress"
title: "Issue #127 Linux 実配布方式 設計 — docs チェーン作成・索引再生成・検証・PR"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/127"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/designs/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/plans/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/tasks/issue-127-linux-dist-design.md"
    modified:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-127-linux-dist-design.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-127-linux-dist-design.md"
    - "docs/designs/issue-127-linux-dist-design.md"
    - "docs/test-specs/issue-127-linux-dist-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-127-linux-dist-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-127-linux-dist-design.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #127 Linux 実配布方式 設計 — docs チェーン作成・索引再生成・検証・PR

## Work

Issue #127 本文を正本として、Ruviform の GPUI 標準 Driver の Linux 実配布要件を docs チェーン 5 文書で正本化する。`docs/` のみを変更し、`crates/`・`Cargo.*`・`mise.toml`・`.github/` には触れない。

## Classification

- `docs-only-design`

## Upstream Documents

- `docs/specs/issue-127-linux-dist-design.md`
- `docs/designs/issue-127-linux-dist-design.md`
- `docs/test-specs/issue-127-linux-dist-design.md`
- `docs/plans/issue-127-linux-dist-design.md`

## Dependencies

- なし（本タスクがチェーンの終端）

## Affected Files

- `docs/specs/issue-127-linux-dist-design.md`（新規）
- `docs/designs/issue-127-linux-dist-design.md`（新規）
- `docs/test-specs/issue-127-linux-dist-design.md`（新規）
- `docs/plans/issue-127-linux-dist-design.md`（新規）
- `docs/tasks/issue-127-linux-dist-design.md`（新規・本書）
- `docs/specs/README.md`・`docs/designs/README.md`・`docs/test-specs/README.md`・`docs/plans/README.md`・`docs/tasks/README.md`（管理索引再生成）

## Done When

- [ ] 5 文書が front matter relations で相互リンクし、全 `relations` path が実在する
- [ ] design が Issue #127 受入 5 行を全て対応節へ写像（§9）
- [ ] capability matrix・runtime 依存・形式比較が実測/未実測/宣言/非対応/未実行を区別し、Linux 実機 `Pass` を主張しない
- [ ] deb/AppImage/Flatpak が全て `保留` で未実測形式を配布可能と断定しない
- [ ] 旧 WebView/WebKitGTK が GPUI 必須条件から除外・#465 分離・削除済み非主張
- [ ] Ruviform/AudioLens/Synvelis の Owner 境界と暗黙の必須依存を生じさせない条件を記述
- [ ] `doc_maintenance.js apply` で 5 README 再生成・新規文書が索引に載る
- [ ] `audit_docs` で新規 5 文書の error が 0（既存の無関係 findings は分離報告）
- [ ] `mise run fmt-check` が pass
- [ ] `git diff --name-only origin/main...HEAD` が `docs/` のみ
- [ ] review-protocol 実施・確認済み P0/P1 blocker なし
- [ ] `feature/127-linux-dist-design` を push し日本語 PR（要約/変更点/検証/NotRun/review-protocol/`Refs #127`）作成
- [ ] `worker_done` を 1 回送信

## Verification

- test-spec `docs/test-specs/issue-127-linux-dist-design.md` の R1–R12 に従う。
- `mise run fmt-check`・index 再生成・audit・diff scope を実際に実行してから報告する。

## Notes

- Linux 実機検証・パッケージ形式採用・distro 決定・WebView 撤去・GPUI 最終採否は全て後続 Issue/Consumer 責務。本タスクはそれらを `NotRun`/`保留`/`Unsupported` として正直に記録するのみ。
- 既存 docs audit findings（`al-media-support-matrix` の ID/status/source、issue-313/243/410 の broken relation、graph route の `focus-required`）は本タスクの範囲外であり修復しない。
