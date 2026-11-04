---
id: "TSPEC-034RBIMbJjTnaygGjWGrGx"
type: "test-spec"
status: "approved"
title: "Issue #330 CI-OPT-03 test specification — install 重複排除・dist 依存順・互換性"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "docs/specs/issue-330-ui-npm-ci-dedup.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-330-ui-npm-ci-dedup.md"
    - "docs/plans/issue-330-ui-npm-ci-dedup.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-330-ui-npm-ci-dedup.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #330 CI-OPT-03 test specification — install 重複排除・dist 依存順・互換性

## Purpose

spec AC-1〜AC-6 の検証契約を固定する。検証対象は「CI パイプラインの振る舞い」であり、UI パッケージ自体の正しさ（従来の check/test/build 群が担保するもの）を再検証しない。

## Feature

UI npm 依存の install が lockfile 単位で 1 回に集約され、verify（typecheck/test/build）と dist 生成が正しい順序・revision で実行されること。

## Rules

- Rule: 同一 `ui-verify` ジョブ内で `npm ci` は package-lock ごとに最大 1 回。`mise run setup-ui` 後の verify 系タスクの `depends` 解決では install は `sources up-to-date, skipping` となり npm ci 本体は走らない。
- Rule: Fresh checkout（node_modules 非存在）では verify タスク単独実行でも `depends` 経由で install が走り検証が完走する。`outputs`（`node_modules/.package-lock.json`）欠落は常に install を stale にする。
- Rule: `ui/browser-test` の typecheck・実行は `build-ui-showcase` 完了後の `ui/showcase/dist` を参照する（`@rvf/showcase` paths 契約）。同一 source revision では showcase dist は 1 度だけ emit される。
- Rule: verify 系タスク（check/test）に freshness は付与しない — 検証は常に実行される。
- Rule: `ui/showcase/test/deps.test.ts` の DAG audit（src import 方向・bare specifier 禁止・dev dep allowlist）は変更しない。
- Rule: CI ステップ分割・ジョブ構造・Evidence upload は不変。typecheck/test/build の失敗は従来どおり独立 step で特定できる。

## Examples

- Example: `ui/*/node_modules` を全削除して `mise run setup-ui` を実行 → npm ci が 7 回（package-lock ごとに 1 回）実行される。続けて `mise run test-ui-headless` → install-ui-core が skip され `node --test` のみ走る。
- Example: `mise run test-ui-browser` → `build-ui-showcase` が `sources up-to-date, skipping` となり、Playwright suite が showcase dist 参照で緑。
- Example: `ui/core/package-lock.json` を更新 → install-ui-core が stale になり再 install される。
- Example: `ui/showcase/dist` を削除 → `check-ui-browser-test` の depends 解決で `build-ui-showcase` が stale → dist 再 emit → typecheck 成功。

## Guarantees

- install 重複排除が「検証の skip」ではなく「準備の再利用」で実現されていること。
- install 失敗は setup-ui または当該 install-ui-* の step として、typecheck/test/build の失敗と分離して報告されること。
- lockfile・manifest・source の実変更が install/build の再実行を必ず引き起こすこと（freshness の保守性）。

## Non-goals

- typecheck/test/build の個別内容の正しさ（既存の各 lane が担保）。
- Playwright ブラウザバイナリ取得時間・OS system deps の最適化（CI-OPT-05 の範囲）。
- npm install の内容一致監査（npm ci の lockfile 厳格性に委譲）。

## Risk

- これら保証を落とすと、stale node_modules / stale dist による偽緑、または install 失敗と検証失敗の識別不能が入り得る。freshness は mise 標準機能の fail-safe 側（不明なら run）に寄せてあるため、崩れた場合の症状は「効率低下」に留まり検証欠落にならない設計。
