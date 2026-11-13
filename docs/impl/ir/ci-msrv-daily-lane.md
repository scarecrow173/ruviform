---
id: "IMPL-034WJNAwo0m2Ia4NNALd1w"
type: "impl"
status: "completed"
title: "Issue #594 MSRV lane implementation record"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/594"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/specs/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/designs/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/plans/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/tasks/ci-msrv-daily-lane.md"
      - type: "file"
        path: "docs/test-specs/ci-msrv-daily-lane.md"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "ci/summary/summary.mjs"
      - type: "file"
        path: "ci/summary/tests/summary.test.mjs"
      - type: "file"
        path: "crates/ruviform-observability/tests/logging.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/tasks/ci-msrv-daily-lane.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/ci-msrv-daily-lane.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #594 MSRV lane implementation record

## Summary

`ci.yml`（workflow_dispatch 専用・daily-ci 経由で日次 dispatch）に `msrv-verify` job を追加し、`mise.toml` の 4 つの `test-*-msrv` タスクを rust 1.89.0 toolchain で CI 実行する経路を新設した。併せて fail-closed 集計の `ci-summary` に lane を登録するため、`ci/summary/summary.mjs` の `REQUIRED_JOBS` / `REQUIRED_CHECKOUT_LEGS` と `ci/summary/tests/summary.test.mjs` の列挙 test を更新した（Coordinator 承認の範囲拡張）。ローカル必須検証（`fmt-check` / `check` / `lint` / `test` / `test-rt` / `build`）には MSRV task を追加せず、第二 toolchain の再コンパイルは CI lane に委譲する。

## Context

Issue #594 の受入条件に基づく。synvelis の `msrv-verify` job を同型参照とし、ruviform では 4 task（`test-ofx-msrv`・`test-ui-test-msrv`・`test-observability-msrv`・`test-io-msrv`）を個別 step として実行する設計とした。`ci-summary` は `needs` だけでなく `summary.mjs` の宣言 lane 一覧と checkout-SHA evidence 宣言の両方を fail-closed で検査するため、job 追加だけでは不足し、evaluator 側の宣言更新が必須だった。

## Implementation

### Changes

#### `.github/workflows/ci.yml`

- `ci-summary` 直前に `msrv-verify` job を追加。`runs-on: ubuntu-latest`、`CARGO_INCREMENTAL: "0"`、`CHECKOUT_LEG: msrv-verify`。
- checkout は `${{ github.sha }}` pin、実測 `git rev-parse HEAD` を `checkout-sha-msrv-verify-attempt-<n>.txt` として upload（既存 lane と同一の evidence 契約）。
- mise 復元（`mise trust` + `mise install --locked rust prek`）、専用 cargo cache（`ubuntu-cargo-msrv-<Cargo.lock hash>` / restore key `ubuntu-cargo-msrv-`）、`mise install rust@1.89.0` を配置。
- 4 task を個別 step（`msrv-ofx` / `msrv-ui-test` / `msrv-observability` / `msrv-io`）で実行。各 step に `if: ${{ !cancelled() }}` を付し、先行 task 失敗時も後続 task が skip されず全件実行される（review で指摘された coverage gap への対応 — matrix lane の `fail-fast: false` と同じ意図）。`if: always()` の lane evidence step が run/attempt・実測 commit・宣言 rust version・各 task outcome を `$GITHUB_STEP_SUMMARY` に記録。
- `ci-summary.needs` に `msrv-verify` を追加。

#### `ci/summary/summary.mjs`

- `REQUIRED_JOBS` と `REQUIRED_CHECKOUT_LEGS` に `"msrv-verify"` を追加。これにより job 未登録・非 success 結果・checkout evidence 欠落が既存 lane と同じく fail-closed で検出される。

#### `ci/summary/tests/summary.test.mjs`

- required-job 列挙 test と checkout-leg 列挙 test の双方に `"msrv-verify"` を追加。

#### `crates/ruviform-observability/tests/logging.rs`（coordinator 承認の第 2 範囲拡張）

- 初回 `gh workflow run ci`（run 43383950490, `expected_sha` = 4a6026d8）で `test-observability-msrv` が `rolling_file_retention_deletes_oldest_at_build` / `rolling_file_retention_matches_prefixed_and_suffixed_files` の 2 件で失敗。
- 原因特定: upstream `tracing-appender` 0.2.5 の `prune_old_logs` は `Metadata::created()`（birth time）で retention 対象をソートする。seed file 5 件を tight loop で連続 write すると Linux runner で同一 timestamp tick を共有し、削除順が readdir 順に退化して「oldest first」契約が壊れた。feature-gated test target（`required-features = ["logging"]`）は `--all-features` を使う `test-observability-msrv` でのみ実行されるため、通常 lane では未検出で MSRV lane の Linux 初走行で顕在化。
- 修正: `SEED_FILE_BIRTH_GAP`（100ms）の実時間 sleep を 2 テストの seed 生成ループに挿入し、各 file の birth time を区別可能にした。birth time は明示設定不可のため実時間が唯一の lever。テスト無効化・allow 追加なし。production code・task 定義・検証強度は不変。

#### doc-chain（`docs/{specs,designs,plans,tasks,test-specs}/ci-msrv-daily-lane.md`）

- spec → design → plan → task → test-spec の canonical chain を作成。範囲拡張（`ci/summary/*`・`crates/ruviform-observability/tests/logging.rs`）を design/plan/task/spec に明記。

### Decisions During Implementation

- **`mise.toml` は無変更**: 4 つの `test-*-msrv` task は既に `mise exec rust@1.89.0` で第二 toolchain を明示選択し、target dir も分離済み（`target/msrv-*`）。task 調整は不要と判断。
- **`ci-summary` 宣言の 3 箇所更新**: workflow `needs` だけでは evaluator が undeclared job として扱い、checkout evidence の fail-closed 検査も効かない。`summary.mjs` + test の両方を更新するのが唯一の整合的変更。
- **4 task を個別 step 化**: synvelis は単一 `test-msrv` task だが、ruviform は task が 4 分割済み。個別 step + ID により、先行 task 失敗時も後続 task の outcome を evidence に残せる。
- **4 step に `if: ${{ !cancelled() }}`（review-protocol 対応）**: 初版は default の `success()` ゲートで、先行 task 失敗時に後続 3 task が skip される設計だった。1 件の MSRV 破損が修正されるまで他 crate set の独立破損を不可視にする coverage gap があるため、失敗後も全 task を実行する形に修正。cancel 時は停止する `!cancelled()` を選び `always()` は避けた（cancelled run で証跡だけ残る誤判定を防ぐ）。
- **IR file 名を doc-chain slug に統一**: `new_impl_record.js` の既定名 `issue-594-msrv-lane-implementation-record.md` から `ci-msrv-daily-lane.md` へ rename し、spec/plan が宣言していた `docs/impl/ir/ci-msrv-daily-lane.md` への参照と一致させた（review 指摘）。
- **ローカル必須検証から MSRV を除外**: 第二 toolchain の全量再コンパイルはコストが高く、日次 CI lane が担保する。Issue の意図どおりローカル必須セットは不変。
- **retention fixture は実時間 sleep で決定化**: birth time は OS が採番し明示設定できないため、seed 間隔を実時間（100ms）で確保する方式を採用。ファイルシステムの timestamp 粒度が 100ms を超える環境では依然として非決定的になり得るが、ローカル FS の実用粒度（ns〜ms 級）に対し十分な余裕がある。upstream 恒久修正（mtime/filename ベース retention）は別 Issue 候補。
- **`crates/` 範囲拡張は fixture のみに限定**: coordinator 承認は「seed file 生成を決定的にする修正」に限定され、テスト無効化・allow 追加は禁止条件。production code・他 crate・他 test target には触れていない。

## Related Experiments

### Adopted

- なし（実験 log は作成していない — 設計判断は design doc に集約）。

### Rejected

- なし。

## Validation

実施済み:

- `mise exec -- actionlint`（変更前 baseline）→ exit 0、警告なし。
- `mise exec -- actionlint`（変更後）→ exit 0。
- `mise run test-ci-summary` → exit 0（31 tests / 31 pass / 0 fail / 0 skipped）。
- `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `build` → 全て exit 0（Windows 11 x64・rust 1.99.1）。fixture 修正 commit 後に 6 件全て再実施し exit 0。
- `mise exec -- cargo test -p ruviform-observability --all-features --test logging`（補助証拠 — canonical は mise task）→ exit 0、28/28 pass（retention 2 件含む）。fixture 修正の直接検証。
- `gh workflow run ci` 1 回目（run 43383950490, `expected_sha` = 4a6026d8）→ failure。`msrv-verify` の `test-observability-msrv` で上記 retention 2 件が失敗、`ci-summary` は `msrv-verify:job-failure` で fail-closed（checkout-sha evidence は正常に提出・評価された — 新 lane の evidence 契約は機能）。
- `gh workflow run ci` 2 回目（run 43385858552, `expected_sha` = f43ea2c = fixture 修正込みブランチ先端）→ **success**。`msrv-verify` job success（4 task step 全て success・checkout-sha evidence upload・lane evidence 記録を step 単位で確認）、`ci-summary` verdict PASS（`msrv-verify` declared=true/result=success/Pass、checkout leg `msrv-verify` sha 一致 Pass）。run 全体 conclusion = success、他 lane の非退行も確認。https://github.com/scarecrow173/ruviform/actions/runs/43385858552

## Risks

- `msrv-verify` は `rust@1.89.0` の初回 install と第二 toolchain の全量ビルドを伴い、他 lane より実行時間が長い。専用 cargo cache（`cargo-msrv` key）で緩和するが、cold cache 時はさらに延びる可能性がある。
- MSRV task が CI で初めて 1.89.0 実機検証されるため、既存の潜在的 MSRV 破損が顕在化する可能性がある。その場合は本 lane の失敗として正しく検出される（fail-closed の意図どおり）— 実際に fixture 非決定性が顕在化し、範囲拡張で対処した。
- retention fixture の 100ms sleep は filesystem timestamp 粒度への依存を残す（粒度 >100ms の FS では再発し得る）。upstream `tracing-appender` の retention が birth time 依存である限り構造的な脆さは残る — 恒久対応は upstream または依存更新の別 Issue 候補。
- `docs/development-environment.md` の lane 数記述（旧来の宣言数）が本変更で stale になるが、編集範囲外のため未対応。PR 本文の「未対応の作業」に記載。

## Follow-ups

- `gh workflow run ci` 2 回目 dispatch の `msrv-verify` 実機 green 確認 → **完了**（run 43385858552 success・ci-summary PASS）。
- `docs/development-environment.md` の lane 構成記述を新 lane 数に合わせて更新する別タスク。
- upstream `tracing-appender` retention の timestamp 依存に関する恒久対応（mtime/filename ソートへの変更、または依存更新）の Issue 起票検討。
