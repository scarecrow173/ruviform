---
id: "TASK-034YYxvzrqG58pQIKDaukG"
type: "task"
status: "done"
title: "Issue #721 post-seal temp 境界確定 — bench-smoke 照合・README 補強・検証"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["devin-worker"]
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
    - "docs/plans/issue-721-post-seal-temp.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-721-post-seal-temp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-650-bench-smoke-coverage.md"
    - "docs/tasks/issue-306-io-e2e-cases.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-721-post-seal-temp.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-721-post-seal-temp.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/721"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/650"
  defers: []
  deferred-by: []
metadata:
  issue: 721
---

# Issue #721 post-seal temp 境界確定 — bench-smoke 照合・README 補強・検証

## Work

`scarecrow173/ruviform#721` の 2 残件を確定する doc+検証 slice。

1. 残件 (a): `mise run bench-smoke` が `ruviform-io-e2e`
   `io_composition` を計測なし 1 回実行することを実測確認し、
   `docs/development-environment.md` の bench 系記述と
   `mise.toml` を read-only で照合する（#720 占有のため同
   ファイルは編集しない）。
2. 残件 (b): post-seal temp 改竄境界の設計判断を
   `docs/designs/issue-721-post-seal-temp.md` で「現行契約維持」
   と確定し、`crates/ruviform-io-file/README.md` §信頼境界へ
   post-seal 再検証なしの最小項目を追加する（実装変更なし）。
   spec→design→plan→task→test-spec→impl/ir・exp の文書
   チェーンと `evidence/rf721/` を作成し、比例検証を実測する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-721-post-seal-temp.md`（approved）
- `docs/designs/issue-721-post-seal-temp.md`（approved）
- `docs/plans/issue-721-post-seal-temp.md`（approved）
- `docs/test-specs/issue-721-post-seal-temp.md`（approved）
- `POLICIES.md`

## Dependencies

- PR `scarecrow173/ruviform#651`（merge commit `e69d92d0`）で
  bench-smoke の io-e2e 配線が main 着地済み — 確認済み。
- consumer 利用実績（audio-lens `al-job`/`al-media`、synvelis
  `synvelis-library`/`synvelis-player`）は read-only 参照で照合
  済み — 全て同一信頼境界内の連続 pipeline。
- `docs/development-environment.md` は `scarecrow173/ruviform#720`
  の並行 Dispatch が占有中 — 本 slice では read-only 参照のみ。

## Affected Files

- `crates/ruviform-io-file/README.md` — §信頼境界・保証範囲へ
  Post-seal temp 内容の 1 項目追加（doc のみ・実装差分なし）
- `evidence/rf721/` — bench-smoke 抜粋・検証照合記録
- `docs/{specs,designs,plans,tasks,test-specs,impl}/{ir,exp}/
  issue-721-post-seal-temp.*` — 内部 doc chain
- `docs/*/README.md` — index 未同期のまま（Coordinator が統合時に
  再生成・本 slice では触れない）

## Done When

- [x] `mise run bench-smoke` が `ruviform-io-e2e` `io_composition`
      6 case を計測なし 1 回実行して exit 0（実測・
      `evidence/rf721/`）
- [x] dev-env doc ↔ `mise.toml` bench 系記述の一致を read-only
      照合（drift なし）
- [x] post-seal temp 改竄の設計判断（現行契約維持）が設計文書で
      確定し、根拠・強化案却下理由・consumer 照合が記録される
- [x] `crates/ruviform-io-file/README.md` §信頼境界が採用契約と
      整合する（post-seal 再検証なしの明示）
- [x] `mise run test ruviform-io-file` / `mise run test-io-e2e`
      が pass（既存 suite 回帰なし・`post_seal_temp_*` 2 試験 ok）
- [x] `mise run fmt-check` が exit 0
- [x] review-protocol self-review で blocker なし

## Verification

- [x] `mise run bench-smoke` — Pass（exit 0、`io_composition`
      6 case 全て `Testing ... Success`・Windows x86_64・
      rustc 1.99.1）
- [x] dev-env doc ↔ `mise.toml` bench 系記述照合 — Pass
      （`bench`/`bench-smoke` 両行の `-p` 列挙・説明一致）
- [x] `mise run check ruviform-io-file` / `mise run lint
      ruviform-io-file` / `mise run test ruviform-io-file` —
      Pass（全て exit 0・README doc のみの変更）
- [x] `mise run check-io-e2e` / `mise run lint-io-e2e` /
      `mise run test-io-e2e` — Pass（全て exit 0・
      `post_seal_temp_*` を含む既存 suite 回帰なし）
- [x] `mise run fmt-check` — Pass（exit 0）
- [x] review-protocol self-review — blocker なし
- [ ] `*-msrv`・`*-full`・3 OS 横断 — 本 slice の明示対象外
      （Daily Full CI 委譲・NotRun）
- [ ] 実 FS fault の OS 別挙動差（Windows ACL/readonly・POSIX
      share-lock）— headless lane Pass を実 OS 受入に昇格しない
      方針で NotRun/Unsupported を分離記録（POLICIES.md §4）
