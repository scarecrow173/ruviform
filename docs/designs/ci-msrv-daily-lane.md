---
id: "DESIGN-034WJELx3H0UQk20mwM76W"
type: "design"
status: "approved"
title: "Issue #594 設計 — ci.yml MSRV lane の構成と ci-summary 宣言 lane 登録"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/594"
    - "docs/specs/ci-msrv-daily-lane.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/ci-msrv-daily-lane.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/designs/README.md"
  implements:
    - "docs/specs/ci-msrv-daily-lane.md"
  implemented-by: []
  depends-on:
    - "docs/specs/ci-msrv-daily-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/ci-msrv-daily-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ci-msrv-daily-lane.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/synvelis/blob/main/.github/workflows/ci.yml"
  defers: []
  deferred-by: []
---

# Issue #594 設計 — ci.yml MSRV lane の構成と ci-summary 宣言 lane 登録

## Overview

`ci.yml` に `msrv-verify` job を 1 本追加し、4 つの `*-msrv` mise タスクを `rust@1.89.0` toolchain で逐次実行する。job は既存 lane と同一の定型を踏む: checkout を `github.sha` に pin → 実測 checkout SHA を `checkout-sha-msrv-verify-attempt-<n>.txt` として artifact upload（Issue #366 sha evidence）→ mise 復元 → cargo registry cache → `mise install rust@1.89.0` → 4 タスク → `if: always()` の lane evidence を `GITHUB_STEP_SUMMARY` に記録。終端 `ci-summary` の `needs` に `msrv-verify` を追加し、評価器 `ci/summary/summary.mjs` の `REQUIRED_JOBS` / `REQUIRED_CHECKOUT_LEGS` にも登録して宣言 lane とする。

## Lane job design

- **runs-on / timeout**: `ubuntu-latest` 単独。対象 crate 群（plugin-core/plugin-ofx・ui-test・observability・io 系）は cpal/audio-device を推移依存に持たず、ALSA 等の OS 依存は不要（`Cargo.toml` の依存宣言で確認済み）。`timeout-minutes` は ruviform `ci.yml` の既存 lane がいずれも未設定のため付けない（synvelis は 60 分設定だが本リポジトリの慣行に合わせる）。
- **mise 復元**: `mise trust` + `mise install --locked rust prek` — 全 lane 共通の「Restore CI-required tools」step をそのまま使う（`MISE_ENV=ci` で rust 1.99.0 pin が有効。prek は本 lane では未使用だが step の一貫性を優先し special-case しない）。
- **cargo registry cache**: key は `${{ runner.os }}-cargo-msrv-${{ hashFiles('Cargo.lock') }}`、restore-keys は `${{ runner.os }}-cargo-msrv-`。ui lane（`-cargo-ui-`）・gpui lane（`-cargo-ui-gpui-`）と同じ「特殊 lane は専用 prefix」慣行に倣い、synvelis `msrv-verify` と同じ key 形にする。registry 内容は toolchain 非依存だが、共有 key への書き込み競合・eviction 干渉を避けて分離する。
- **MSRV toolchain**: `mise install rust@1.89.0` を明示 step として置く（`workspace.package` の `rust-version = "1.89"` 宣言との一致検証。synvelis と同型）。4 タスクは内部で `mise exec rust@1.89.0 -- cargo ...` を呼ぶため、task 側の変更は不要。
- **4 タスク**: `test-ofx-msrv` → `test-ui-test-msrv` → `test-observability-msrv` → `test-io-msrv` の順に別 step（`id: msrv-ofx` / `msrv-ui-test` / `msrv-observability` / `msrv-io`）で実行。step 分離により失敗時の job log と evidence 表でどのタスクが落ちたか一意に分かる。各 step に `if: ${{ !cancelled() }}` を付し、先行タスクの失敗で後続タスクが skip されないようにする — 1 件の MSRV 破損が修正されるまで他 crate set の独立破損を不可視にしないため（matrix lane の `fail-fast: false` と同じ意図）。cancel 時は従来どおり停止する。
- **lane evidence**: `if: always()` で `GITHUB_STEP_SUMMARY` に run id/attempt・実測 commit・宣言 rust-version（`grep -E '^rust-version' Cargo.toml`）・4 step の `outcome` 表を記録（synvelis 同型。dep-access credential は ruviform に存在しないため含めない）。
- **sccache / ALSA / LFS**: 付けない — 軽量 test lane（icc-verify/compression-verify と同じ構成）であり、対象 graph に cpal・git 依存・長時間 compile の必要性がない。`*-msrv` タスクは `CARGO_TARGET_DIR=target/msrv-*` で dev/CI profile と分離済み。

## ci-summary 集計への組み込み

- `ci-summary.needs` に `msrv-verify` を追加（配列末尾）。`NEEDS_JSON` 経由で `needs.msrv-verify.result` が評価器に届き、success 以外は既存 job と同じ `job-<result>` reason で Fail → run 失敗 → `ci-issue-tracker` の `workflow_run` 経路で tracker に伝播する。
- `ci/summary/summary.mjs` の `REQUIRED_JOBS` に `"msrv-verify"`、`REQUIRED_CHECKOUT_LEGS` に `"msrv-verify"` を追加（ともに末尾）。コメントの「kept in sync with `.github/workflows/ci.yml` needs list」契約を維持し、宣言 lane として (a) needs からの削除・改名を `missing` で fail-closed 検出、(b) checkout-sha evidence artifact 未提出を `checkout-sha-missing` で fail-closed 検出、の両方を有効にする。undeclared の leg は artifact が存在する場合のみ行が生成され、未提出なら評価対象にすら現れないため、宣言登録は省略できない。
- `ci/summary/tests/summary.test.mjs` の 2 つの lane 列挙テスト（"covers every Full CI lane" / "expands matrix lanes"）に `msrv-verify` を追加し、列挙の意味（全 lane 網羅）を維持する。テストは subset 検査のため追加のみで既存 assert は壊れない。
- tracker 連携（`ci-issue-tracker.yml`）は run 結論単位で評価するため変更不要。

## File map

- 変更: `.github/workflows/ci.yml`（`msrv-verify` job 追加 + `ci-summary.needs` 更新）、`ci/summary/summary.mjs`（2 リストに `msrv-verify` 追加）、`ci/summary/tests/summary.test.mjs`（2 列挙に追加）
- 変更なし確認: `mise.toml`（4 タスクは既存のまま動作するため調整不要）
- 新規: doc-chain 6 件（spec/design/plan/task/test-spec/IR）
- 再生成: `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md`（管理 index）

## Risks and Trade-offs

- **lane 時間**: 4 タスクは別 toolchain での部分 workspace 再コンパイル。cache 冷時は数十分規模になり得るが、日次 dispatch のみで実行され PR 経路には乗らないため許容。cache key を `-cargo-msrv-` に分離した分、初回は registry 全取得になる（restore-keys prefix で msrv lane 過去 run の cache は再利用される）。
- **rust@1.89.0 install**: `mise install rust@1.89.0` は `MISE_ENV=ci` の pin（1.99.0）と独立に明示 version を install する。synvelis で実績のある手順。
- **summary.mjs 同期漏れの再発**: 今回は ci.yml needs・summary.mjs 両リスト・test 列挙の 3 箇所を同一変更で揃える。将来 lane を追加する変更も同じ 3 箇所更新が必要 — 恒久的な単一ソース化は別 Issue 候補（workflow YAML からの生成等）として PR に記録。
- **docs/development-environment.md の lane 記述**: 同書は lane 構成と「宣言 17 leg」を記述しており、本変更で 18 leg に増える。編集範囲外のため未更新 — PR の未対応作業に記録する。

## Verification Notes

- [ ] actionlint で workflow 構文を検証（`mise exec -- actionlint`）。
- [ ] `mise run test-ci-summary` で評価器 contract test が pass（宣言 lane 追加後）。
- [ ] `gh workflow run ci`（`expected_sha` = ブランチ先端）で `msrv-verify` job の green を run ログで実測。
- [ ] lane evidence（step summary）と checkout-sha artifact が run に出力されることを run ログ/artifact で確認。
