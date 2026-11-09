---
id: "TASK-034UtPLgt4p9kVhsCVVQ5f"
type: "task"
status: "done"
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
    - "docs/plans/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #288 IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計

## Work

Issue #288 の合意済み設計 v0.1（2026-11-04）を正本設計書として
文書化する docs-only slice。`docs/designs/
issue-288-io-06-cross-consumer-os-fs-security-gate.md` に
判定語彙・結果記録フォーマット・OS/FS capability 軸・
受入 matrix（IO-01 File/IO-02 Structured/IO-03 Compression/
IO-04 Archive/IO-05 Migration/合成 E2E）・否定系判定表
5 領域（publish/recovery・安全性・resource budget・
Media/PCM/RT・Consumer 非回帰）・Owner/境界接続表・
Consumer 要件・Breaking Change 追跡導線・証拠導線・
Gate 種別ごとの実行割当て・設計/実装分離を記述し、
spec/test-spec/plan/task の文書チェーンと生成 index を
整備する。コード変更なし。matrix の実走・テスト実装・
Consumer repo 変更は対象外（実装段階の別 Issue/dispatch）。

## Classification

- `normal-plan-task`

## Upstream Documents

- [plan](../plans/issue-288-io-06-cross-consumer-os-fs-security-gate.md)
- [spec](../specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)
- [design](../designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)
- [test-spec](../test-specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)

## Dependencies

- 参照先の Issue/PR/test-spec/mise task が main に実在すること
  （作成時に確認済み: #295〜#306 merge・PR #391/#395・
  `test-io-*`/`test-ci-consumer-verify`/`check-linux` task・
  `consumer-verify.yml`/`daily-ci.yml`）。
- IO-IMP-01〜12（#295〜#306）の文書チェーンが main に存在する
  こと（本 slice の接続表の根拠）。

## Affected Files

- `docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md`（新規）
- `docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md`（新規・正本）
- `docs/test-specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md`（新規）
- `docs/plans/issue-288-io-06-cross-consumer-os-fs-security-gate.md`（新規）
- `docs/tasks/issue-288-io-06-cross-consumer-os-fs-security-gate.md`（新規）
- `docs/{designs,specs,plans,tasks,test-specs}/README.md`（生成 index）

## Done When

- [ ] 受入 matrix が IO-01〜05+合成 E2E の正常系/否定系を
      Issue #288 受入スコープ表の粒度で網羅し、各行が判定基準と
      既存正本への参照を持つ。
- [ ] publish/recovery・安全性・resource budget・Media/PCM/RT・
      Consumer 非回帰の否定系判定表 5 領域が存在する。
- [ ] Owner/境界接続表が IO-01〜05 テスト要件を実装 Issue
      （#295〜#306）・OS/FS/脅威モデル境界を Consumer Issue
      （AL #139/#140/#141/#143、SV #82/#84/#25）へ接続する。
- [ ] Consumer 要件（影響分類・Golden・回帰・性能・Dependency
      更新）と Breaking Change 採用/戻し追跡導線が明文化される。
- [ ] 証拠導線が既存 Actions/PR/Issue 参照のみで、新規 YAML・
      Dashboard・重複レポートを追加していない。
- [ ] 「実装段階の実走 PASS 要求・設計合意≠テスト完了」が
      明記され、未実施は `not run`/`NotRun`/`Unsupported` と
      記録する方針が明示される。
- [ ] 差分が `docs/` の `issue-288-*` 新規文書と生成 index のみ。

## Verification

- [ ] `mise run fmt-check` pass（docs のみの変更。check/test は
      NotRun — コード差分なし）。
- [ ] doc-status 系 lint/audit で本 PR 追加分に broken relation・
      未解決参照がないこと（既存の範囲外 finding は分離して記録）。
- [ ] 設計文書の参照先（Issue/PR/test-spec/mise task/workflow/
      crate）が実在することを確認済み。
- [ ] `git status`/`git diff --stat` で変更が `docs/` の
      `issue-288-*` 文書と生成 index のみであることを確認。
