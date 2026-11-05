---
id: "SPEC-034Smu6fnzDQaoN4box3qk"
type: "spec"
status: "approved"
title: "Issue #366+#364 Full CI 化 Stage 1 — 常時全 lane 実行・ci-summary fail-closed 集約・io-structured lane 新設"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/366"
    - "https://github.com/scarecrow173/ruviform/issues/364"
    - "https://github.com/scarecrow173/ruviform/issues/365"
  changes:
    added:
      - type: "directory"
        path: "ci/summary"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted:
      - type: "directory"
        path: "ci/impact"
      - type: "file"
        path: ".github/workflows/consumer-impact-check.yml"
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-366-full-ci-stage1.md"
    - "docs/plans/issue-366-full-ci-stage1.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-333-impact-based-ci.md"
    - "docs/designs/issue-346-ruleset-migration-runbook.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/designs/issue-366-full-ci-stage1.md"
    - "docs/plans/issue-366-full-ci-stage1.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/333"
    - "https://github.com/scarecrow173/ruviform/issues/346"
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers: []
  deferred-by: []
---

# Issue #366+#364 Full CI 化 Stage 1 — 常時全 lane 実行・ci-summary fail-closed 集約・io-structured lane 新設

## What Should Be Built

`.github/workflows/ci.yml` を「起動すれば常に全自動検証を行う」Full CI へ段階移行する第一段階。選択実行機構（`impact` 分類 job・`ci/impact/`・`CI_IMPACT_DISABLE`・lane ごとの選択 `if:`）を撤去し、全 lane を無条件実行化する。集約 job を `ci-gate` から **`ci-summary`** へ統一し、fail-closed 判定と実行証拠の Job Summary / JSON artifact 出力を持たせる。同時に Issue #364 の検証漏れを解消する `io-structured-verify` lane を新設し、`workflow_dispatch` に Daily scheduler 連携用 `expected_sha` 入力契約を追加する。`consumer-impact-check.yml` の PR 自動起動は Issue #366 方針どおり廃止する。

## Why Now

- Issue #365（横断移行計画・正本）の順序「まず各 repo の Full CI を全実行化し `ci-summary` へ統一」に従う最初の実装段階。Ruviform・AudioLens・Synvelis 3 repo 並行作業の Ruviform 側。
- Issue #364: `ruviform-io-structured` の `json`/`yaml`/`toml`/`csv`/`file`/`xml` feature-gated code が default workspace check/test/lint で一切コンパイルされず、CI lane も `ci/impact/map.mjs` エントリも無いまま #298 (csv)・#299 (xml)・#304 (io-archive 経由の file feature 使用) と feature が増えている。#299 で `mise.toml` に `check/lint/test-io-structured` は追加済みだが CI から呼ばれない。
- Issue #366: 選択実行の複雑な分類 DAG と PR 非ブロッキング macOS 分離由来の重複を減らし、Full CI をシンプル化する。

## Users and Value

- Ruviform maintainers: Full CI が「起動=全検証」の一契約になり、選択実行の分類不具合や `NotApplicable` 判定に依存しない fail-closed 集約を得る。
- Daily scheduler（Issue #367 後続）: `expected_sha` 入力で「期待した main revision を検証した run」と誤認しない契約を得る。
- Consumer repos (Synvelis/AudioLens): `ci-summary` が全 lane（macOS legs 含む）の終了まで待つ fail-closed 証拠を Run 単位で提供する。

## Scope

### In Scope

- `impact` job・`ci/impact/`（classify/gate/lib/map/observe + tests）・`CI_IMPACT_DISABLE`・全 lane の選択 `if:`・`needs: [impact]` の撤去。
- 全 lane 無条件実行化（PR/push/`workflow_dispatch` 全 trigger で同一 lane 集合）。
- `io-structured-verify` lane 新設（ubuntu-latest・`check-io-structured`/`test-io-structured`/`lint-io-structured`）。
- `ci-gate` + `macos-report` を統合した最終集約 job `ci-summary`（`if: always()`・全 lane `needs`・fail-closed・Job Summary 表・`ci-summary-report` JSON artifact・`expected_sha` 照合）。
- `workflow_dispatch` `expected_sha` 入力追加。
- `consumer-impact-check.yml` の撤去（PR 自動起動の廃止。分類要件自体は `PULL_REQUEST_TEMPLATE.md` + レビュアー human gate に残る）。
- `test-ci-impact` → `test-ci-summary` mise task 置換、living docs（`docs/development-environment.md`）同期。

### Out of Scope

- `pull_request`/`push` trigger の除去（ruleset `main-checkstatus` required 解除確認後の最終切替。Owner 作業）。
- ruleset / required context の変更（Owner が GitHub Settings で実施。本 stage は 9 required context の check 名を一切変更しない）。
- `*-macos` 別 job の 3OS matrix 物理統合（後続 stage の単純化候補として据置。本 stage では別 job のまま全実行・`ci-summary` needs 入り）。
- `daily-ci.yml` / `ci-issue-tracker.yml` 新設（Issue #367/#368）。
- `consumer-verify.yml` 変更（#294 候補 SHA 手動 Gate として維持）。
- real GUI / real hardware 未自動化範囲の偽装表示（実施済み Pass にしない）。

## Requirements

- R1: 既存 PR/push trigger・既存 required context（`verify`/`l1-real-verify`/`ui-verify` の Linux・Windows leg + `rt-verify`/`icc-verify`/`bench` の 9 context）を維持し、dead required context を作らない。
- R2: 全 lane が選択分類なしで常時実行される。利用不可環境は skip ではなく Fail/Cancelled として run が正しく終了する。
- R3: `ci-summary` は全必要 job の `needs` 結果を `if: always()` で待ち、`success` 以外（failure/cancelled/skipped/abandoned/not_run/unknown）を成功にしない。
- R4: `io-structured-verify` が `check`/`lint`/`test-io-structured` の feature matrix（default・xml・xml,file・all-features）を実行する。
- R5: `expected_sha` 非空かつ全 lane leg の実測 checkout（`git rev-parse HEAD`）共通値と不一致・または実測不能なら `ci-summary` を Fail にする（異なる revision を検証済みにしない — `github.sha` は期待値の代理にしない）。実際の検証 SHA・event・job/OS・結果・leg 別 checkout SHA・workflow revision・Run URL を Job Summary と JSON artifact に記録する。
- R6: 選択実行だけに使う script/tests/docs を削除し、別用途（診断・配布・metrics）の関数を誤削除しない。
- R7: 必要テストを削減しない。差分・失敗を隠すテスト無効化・警告抑制・`-D warnings` 解除を行わない。
- R8: `consumer-impact-check.yml` の PR 自動起動を廃止し、Consumer 影響分類要件を `PULL_REQUEST_TEMPLATE.md` + レビュアー確認で維持する。

## Acceptance Criteria

- [ ] `ci.yml` が `pull_request`/`push`/`workflow_dispatch` で起動し、全 lane（verify・verify-macos・rt-verify・icc-verify・compression-verify・io-structured-verify・ui-verify・ui-verify-macos・bench・l1-real-verify・l1-real-verify-macos）が分類なしで実行される。
- [ ] `ci-summary` が全 lane 終了まで待ち、1 本でも failure/cancelled/skipped/unknown があれば job 自体が Fail になる。
- [ ] `io-structured-verify` job が存在し、`mise run check/test/lint-io-structured` を実行する。
- [ ] `mise exec -- actionlint .github/workflows/ci.yml` が pass する。
- [ ] `mise run test-ci-summary` の contract test が verdict 行列（success/failure/cancelled/skipped/missing・expected_sha 一致/不一致/未指定）を検証する。
- [ ] `ci/impact/`・`test-ci-impact`・`CI_IMPACT_DISABLE`・`impact` job・lane `if:` が存在しない。
- [ ] `consumer-impact-check.yml` が存在せず、PR 起動しない。
- [ ] `docs/development-environment.md` が新 lane 構成・`ci-summary`・`expected_sha`・consumer-impact-check 廃止を反映する。

## Deferred Design Concerns

- `*-macos` 3 job の matrix 統合 | Stage 1 は needs 直接参照による per-OS 集約を優先（API 不要・fail-closed 判定が needs 純関数のまま） | `ci-summary` required 化後の単純化 stage | 重複 ~250 行が残る
- PR/push trigger 除去 | required 解除前の除去はマージ不能デッドロック（#365 ルール 6） | Owner が `main-checkstatus` required を空にした確認後 | 先行除去で全 PR ブロック
- 手動/scheduled 同一 SHA 判定・Daily dispatch | Issue #367 の専用 workflow | #367 実装時 | scheduler なしでは `expected_sha` は手動運用のみ
- `consumer-impact-check` のローカル検証移行 | 本 stage は template+human gate 維持を選択 | 運用観測後に必要なら別 Issue | 記入漏れはレビュアー依存になる

## Sources and Relations

- [Issue #366](https://github.com/scarecrow173/ruviform/issues/366): Full CI 単純化・`ci-summary` 統一・選択実行撤去・`expected_sha` 契約・consumer-impact-check 廃止（実装手順 1-6。手順 7 の trigger 除去は最終切替）。
- [Issue #364](https://github.com/scarecrow173/ruviform/issues/364): io-structured dedicated lane 不在・format feature 全未カバー。
- [Issue #365](https://github.com/scarecrow173/ruviform/issues/365): 横断移行計画（Full CI 全実行化 → Scheduler/Tracker → Required 解除 → trigger 移行の順序と不変条件、required context 実測 9 件）。
- 旧選択実行の正本（履歴として保持・新方式には持ち込まない）: `docs/designs/issue-333-impact-based-ci.md`、`docs/designs/issue-346-ruleset-migration-runbook.md`。
