---
id: "PLAN-K87gfjgzqnomAg4MWPIJvP"
type: "plan"
status: "completed"
title: "IO-06 Gate matrix 実走と証拠記録（Issue #288 slice）"
created: "2026-11-11"
updated: "2026-11-12"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/io-06-gate-run.md"
      - type: "doc"
        path: "docs/designs/io-06-gate-run-design.md"
      - type: "doc"
        path: "docs/plans/io-06-gate-run-plan.md"
      - type: "doc"
        path: "docs/tasks/io-06-gate-run-task.md"
      - type: "doc"
        path: "docs/test-specs/io-06-gate-run.md"
      - type: "doc"
        path: "docs/impl/ir/io-06-gate-run-ir.md"
      - type: "file"
        path: "evidence/rf288/io06-gate-matrix.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/io-06-gate-run.md"
  implemented-by: []
  depends-on:
    - "docs/specs/io-06-gate-run.md"
    - "docs/designs/io-06-gate-run-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/io-06-gate-run.md"
    - "docs/designs/io-06-gate-run-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/io-06-gate-run.md"
  references: []
  defers: []
  deferred-by: []
---

# IO-06 Gate matrix 実走と証拠記録（Issue #288 slice）

## Upstream Documents

- [spec](../specs/io-06-gate-run.md)
- [design](../designs/io-06-gate-run-design.md)
- [test-spec](../test-specs/io-06-gate-run.md)
- Gate 判定正本: [issue-288 design](../designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)

## Scope

本 slice は **コード変更を伴わない検証・証拠 slice**。
`evidence/rf288/io06-gate-matrix.md` と文書チェーンのみを
成果物とし、`crates/`・`mise.toml`・`.github/`・`Cargo.toml`
系は無変更。

## File Map

- `docs/specs/io-06-gate-run.md` — spec（範囲・要件・受入条件）。
- `docs/designs/io-06-gate-run-design.md` — 記録設計（行 ID 引継ぎ・
  venue 分類・lane→行対応規則・NotRun 分類・OS/FS 軸の扱い）。
- `docs/test-specs/io-06-gate-run.md` — 証拠記録の正しさを検証
  する test-spec。
- `docs/plans/io-06-gate-run-plan.md` — 本計画。
- `docs/tasks/io-06-gate-run-task.md` — 単一 task の実施記録。
- `docs/impl/ir/io-06-gate-run-ir.md` — Implementation Record。
- `evidence/rf288/io06-gate-matrix.md` — matrix 記録本体
  （セル → lane/exit code/OS/toolchain/既存記録参照）。
- `docs/*/README.md` index — doc-status audit で新規 doc の
  index 未登録が blocking finding になるため、
  `doc_maintenance.js apply` で管理 index を再生成した
  （task spec の「生成ファイルの再生成は範囲外でもよい」に
  基づく。再生成で他 doc の status 齟齬も正規化された）。

## Implementation Sequence

- [x] Issue #288 本文・issue-288 design 正本・`evidence/rf306/`
      3 本・`mise.toml` io 系 task・`ci.yml`/`consumer-verify.yml`
      lane 構成を棚卸し。
- [x] 既存 consumer-verify run（42937524742）の evidence
      artifact を取得し tuple（ruviformRev × consumer SHA・
      lane status）を確認。
- [x] spec / design を作成。
- [x] Windows 実走 lane を `mise run` で実行し exit code を
      記録（fmt-check・check・lint・test・test-rt・
      check/lint/test-io-file・check/lint/test-io-structured・
      check/lint/test-io-compression・check/lint/test-io-e2e・
      test-io-msrv）。第 1 run の check(101)/lint(66) は infra
      flake（rustc metadata stub 破損・無診断終了）と診断し
      `lanes-rerun.log` で再実走 — check/lint/test/test-rt/
      test-io-msrv は全て exit 0 で確定。
- [x] `evidence/rf288/io06-gate-matrix.md` を作成（全セルの
      判定・根拠・残存 NotRun・matrix の穴 7 件）。
- [x] test-spec / task / IR を作成。
- [x] review-protocol レビュー（blocker 解消 — Reviewer A/B
      相当の独立 pass で候補 3 件を検出・検証: zip の性質誤記・
      test-io-file ignored 数の誤記を修正、N=6/8 の文書齟齬は
      観察メモとして記録。確定 P0/P1 残存なし）。
- [x] `git fetch origin main && git merge origin/main` で最新化
      （`588d29a2..1b78607c` fast-forward・docs 簿記のみ）。
- [x] `mise run fmt-check` + doc-status 系 lint/audit
      （fmt-check exit 0。doc_maintenance apply で index 再生成
      後、追加分の blocking finding 0 を確認）。
- [ ] commit → push → `gh pr create` → `gh issue comment 288`
      （実走結果と残存 NotRun・matrix の穴）。

## Dependencies and Risks

- `test`（workspace 全体）と `test-io-msrv` は実行時間が長い。
  `CARGO_BUILD_JOBS=2` で並列度を制限して実行する（rf306 と
  同じ運用）。
- `test-io-msrv` は `mise exec rust@1.89.0` が必要。toolchain
  が利用可能なことを `rustc --version` で事前確認済み
  （rustc 1.89.0 (29483883e 2025-08-04)）。
- CI run 参照は snapshot。run ID・headSha・event・日時を
  matrix に明記して鮮度を判別可能にする。
- consumer-verify の最新 run（42937524742）の candidate
  Ruviform Revision は `9865cd57`（main の祖先・最新 tip では
  ない）。tuple の revision をそのまま記録し、「最新 main での
  consumer 受入」とは記録しない。
- doc-status は既存の blocking finding を多数抱える
  （ベースライン）。本 PR 追加分に新規 finding を出さないことを
  もって docs 検証 Pass とする。

## Task Breakdown

- T1: lane 実走と exit code 記録（本 task に包含）。
- T2: matrix 記録ファイル作成。
- T3: 文書チェーン残部（test-spec/task/IR）+ review。
- T4: main 取り込み・最終検証・commit/push/PR/Issue comment。

## Verification

- [x] `mise run fmt-check` pass。
- [x] doc-status 系 lint/audit で本 PR 追加分に新規 blocking
      finding がないこと。
- [x] matrix の全必須セルに判定と根拠があること
      （test-spec の規則で検証）。
- [x] `git status`/`git diff --stat` で変更が許可範囲
      （`evidence/rf288/`・`docs/*/io-06-gate-run*` + 生成
      index の再生成分）のみであること。

## Open Questions / Gaps

- macOS/Linux 実機・Network/Cloud FS・実 4GiB・consumer 固有
  受入・`Indeterminate` 系 fixture 化は本 slice 対象外であり、
  matrix 上 `not run`/`unsupported` として残る。これらは
  後続 Issue（Coordinator が起票）の対象。
