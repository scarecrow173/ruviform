---
id: "IMPL-034YYygeQzyUVTUGAny27w"
type: "impl"
status: "completed"
title: "Issue #721 post-seal temp boundary decision"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/721"
  changes:
    added:
      - type: "file"
        path: "evidence/rf721/bench-smoke-io-composition-excerpt.txt"
      - type: "file"
        path: "evidence/rf721/verification.md"
      - type: "doc"
        path: "docs/specs/issue-721-post-seal-temp.md"
      - type: "doc"
        path: "docs/designs/issue-721-post-seal-temp.md"
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
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-721-post-seal-temp.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/721"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/650"
    - "docs/designs/issue-721-post-seal-temp.md"
    - "evidence/rf721/verification.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-721-post-seal-temp-exp.jsonl"
    rejected: []
---

# Issue #721 post-seal temp boundary decision

## Summary

`scarecrow173/ruviform#721` の 2 残件を確定した doc+検証 slice。
post-seal temp 改竄境界の設計判断を「現行契約維持」（publish は
seal 後の temp 内容を再検証しない）として
`docs/designs/issue-721-post-seal-temp.md` に記録し、
`crates/ruviform-io-file/README.md` §信頼境界へ post-seal
再検証なしの 1 項目を追加した。bench-smoke lane の
`ruviform-io-e2e` `io_composition` 配線は実測で確認済み
（exit 0・6 case Success）。io-file/io-e2e の実装コードに
差分はない。

## Context

- spec: `docs/specs/issue-721-post-seal-temp.md`（approved）
- design: `docs/designs/issue-721-post-seal-temp.md`（approved）
- plan: `docs/plans/issue-721-post-seal-temp.md`（approved）
- test-spec: `docs/test-specs/issue-721-post-seal-temp.md`
  （approved）
- task: `docs/tasks/issue-721-post-seal-temp.md`
- Issue: `scarecrow173/ruviform#721`（#306 close 時の未 owner
  残件 2 件の引き受け — bench-smoke 配線照合 + post-seal temp
  改竄の設計判断）

## Implementation

### Changes

- `docs/specs|designs|plans|tasks|test-specs/issue-721-post-seal-temp.md`
  — Issue #721 の内部文書チェーン（要求・設計判断・計画・
  task・検証規範）。
- `crates/ruviform-io-file/README.md` — §信頼境界・保証範囲に
  「Post-seal temp 内容」項目を追加: `seal()` は write phase の
  型レベル終了であり `publish()` は seal 後の temp 内容を再検証
  しないこと、外部改竄は publish される bytes を変え外部削除は
  `NotPublished` になること、いずれも呼び出し側信頼境界内の
  事象として契約化されていることを明示（pin 試験・設計文書へ
  参照付き）。
- `evidence/rf721/` — bench-smoke 実行抜粋と検証照合記録。
- `docs/impl/{ir,exp}/issue-721-post-seal-temp.*` — 本記録と
  実験ログ。

### Decisions During Implementation

- **現行契約維持を採用**: consumer 全利用（audio-lens
  `al-job`/`al-media`、synvelis `synvelis-library`/
  `synvelis-player`）が同一信頼境界内の連続 pipeline であり、
  post-seal 改竄を脅威モデルに含める要件が存在しない。
  README §信頼境界の既存宣言（同一境界内の悪意ある差替えは
  防衛しない）と #306 の pin 済み verdict に整合。
- **強化案 3 件を却下/延期**: seal 時権限強化（同一境界内では
  実効なし + OS 別複雑さ）、publish 時再検証（全 publish に
  O(file) コスト + TOCTOU 残存）、temp 配置分離（同一 FS 前提を
  崩す）。Deferred Design Concerns に再検討トリガー付きで記録。
- **design doc を採用（ADR ではなく）**: `docs/designs/
  issue-306-*`・`issue-650-*` の先例と plan-doc design gate
  （approved status 必須 — ADR の `accepted` では満たさない）に
  整合。
- **`docs/development-environment.md` は非編集**: #720 並行
  Dispatch が占有中。bench 系記述の read-only 照合で drift
  なしを確認したため編集不要と判定。
- **docs index 未同期のまま**: `docs/*/README.md` は Coordinator
  が統合時に再生成する運用のため本 slice では一切触れない。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-721-post-seal-temp-exp.jsonl`（+
  `issue-721-post-seal-temp.md`）— consumer 照合・契約 pin 確認・
  強化案評価・bench-smoke 実測の観察/判断/検証記録。

### Rejected

- `docs/adr/issue-721-post-seal-temp.md` への記録 — plan-doc
  design gate との不整合と先例から design doc を採用。
- 「強化」実装（temp 権限強化・publish 時再検証・配置分離）—
  consumer 要件を超えるコストで実効境界を生まない（design
  決定 3）。

## Validation

canonical 検証（`mise run`、Windows 11 x86_64・rustc/cargo
1.99.1 — `evidence/rf721/verification.md` に一覧）:

- `mise run bench-smoke` — pass（exit 0。`io_composition` 6
  case 全て `Testing ... Success`・計測なし 1 回。
  抜粋: `evidence/rf721/bench-smoke-io-composition-excerpt.txt`）
- `mise run fmt-check` — pass（exit 0）
- `mise run check ruviform-io-file` — pass（exit 0）
- `mise run lint ruviform-io-file` — pass（exit 0）
- `mise run test ruviform-io-file` — pass（exit 0）
- `mise run check-io-e2e` — pass（exit 0）
- `mise run lint-io-e2e` — pass（exit 0）
- `mise run test-io-e2e` — pass（exit 0。`post_seal_temp_*` 2
  試験を含む既存 suite 全件 ok — 採用契約の pin 維持）
- dev-env doc ↔ `mise.toml` bench 系記述照合 — pass（read-only・
  drift なし）
- review-protocol self-review — blocker なし

NotRun: `*-full`・`*-msrv`・3 OS 横断（Daily Full CI 委譲）、
実 FS fault の OS 別挙動差の環境別検証（headless Pass を実 OS
受入に昇格しない — POLICIES.md §4）。Unsupported: なし。

## Risks

- 「seal = publish までの完全性保証」との誤読 — README 補強と
  設計文書で境界を明示して緩和。
- publish 時再検証の将来要件 — Deferred に再検討トリガー記録済み
  （consumer が post-seal 改竄を脅威モデルに含める場合）。
- bench-smoke `-p` 列挙の将来 drift — #650 と同じ構造的リスク、
  恒久対策は対象外。

## Follow-ups

- Coordinator による docs index（`docs/*/README.md`）再生成。
- PR merge 後の Issue #721 close は Coordinator が実施。
- consumer が post-seal 改竄防御を要求した場合は本設計を
  supersede して再検討（Deferred のトリガー条件）。
