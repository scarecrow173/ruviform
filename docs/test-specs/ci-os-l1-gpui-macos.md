---
id: "TSPEC-MzO18iZ8kbgYrUumUAClj3"
type: "test-spec"
status: "approved"
title: "Issue #598 テスト仕様 — macOS GPUI lane の CI 実行・ci-summary fail-closed 伝播・Native Build/Link 包含の検証"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/598"
    - "docs/specs/ci-os-l1-gpui-macos.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/ci-os-l1-gpui-macos.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ci-os-l1-gpui-macos.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ci-os-l1-gpui-macos.md"
    - "docs/specs/ci-os-l1-gpui-macos.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ci-os-l1-gpui-macos.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/ci-os-l1-gpui-macos.md"
  derived-by: []
  verifies:
    - "docs/specs/ci-os-l1-gpui-macos.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #598 テスト仕様 — macOS GPUI lane の CI 実行・ci-summary fail-closed 伝播・Native Build/Link 包含の検証

## Purpose

GPUI adapter の `gpui`/`gpui-test`/`uitest` feature 検証が Windows/Linux のみであった欠落を macOS lane で埋め、(1) `ui-gpui-verify-macos` job が `ui-gpui-verify` と同一範囲を macOS で実際に実行し、(2) その結果が `ci-summary` の fail-closed 集約に既存 job と同一の形で乗り、(3) `test-full` の Native Build/Link 包含が検証され `windowed_test_app` bin の空白が `test-ui-gpui` 拡張で塞がれ、(4) 既存 lane の検証意図を一切弱めないことを保証する。

## Feature

ci OS-L1 lane — `ci.yml` の `ui-gpui-verify-macos` job と `ci-summary` 集計・`ci/summary` 評価器の宣言 lane 登録、`test-ui-gpui` の feature 組合せ拡張。

## Rules

- INV-1(同一検証範囲): `ui-gpui-verify-macos` job は `ui-gpui-verify` と同一の検証 step（driver boundary guard・driver contract・parity・native slot・component/view・golden・scenario contract・`check-ui-gpui`・`lint-ui-gpui`・`test-ui-gpui`・`test-ui-consumer-observation`）を macos-latest で実行し、Linux 専用依存導入 step を持たない。
- INV-2(宣言 lane): `ci-summary.needs`・`summary.mjs` `REQUIRED_JOBS`・`REQUIRED_CHECKOUT_LEGS`・`summary.test.mjs` 列挙の 4 箇所が `ui-gpui-verify-macos` で一致し、needs からの削除・evidence 欠落・failure/cancelled/skipped/abandoned 終了が fail-closed で検出される。
- INV-3(sha evidence): `ui-gpui-verify-macos` は `CHECKOUT_LEG=ui-gpui-verify-macos` で `checkout-sha-ui-gpui-verify-macos-attempt-<n>` artifact を upload し、他 lane と同一の実測 sha を供給する。
- INV-4(bin Build/Link): `test-ui-gpui` は `gpui-test` + `uitest` feature で実行され、`windowed_test_app`（required-features=uitest）が 3 OS の GPUI lane で Build/Link される。`windowed_uitest` test は `RVF_WINDOWED_UITEST` 未設定で no-op、`real-scenario` との非併用を維持する。
- INV-5(検証意図の維持): 既存 job 定義・他 workflow・`mise.ci.toml`・`crates/` を変更せず、lane 削除・skip・検査緩和を行わない。`check-ui-gpui`/`lint-ui-gpui` の feature 指定（`gpui`）は維持する。

## Examples

- Given `ci.yml` に `ui-gpui-verify-macos` job が定義されている, When `gh workflow run ci.yml --ref <branch>` で dispatch される, Then job が macos-latest で起動し checkout-sha evidence upload 後に全検証 step が実行され、全て success なら job は success になる。
- Given `ui-gpui-verify-macos` が `ci-summary.needs` と `REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS` に登録されている, When 同 job が failure/cancelled/skipped/abandoned で終了する, Then `ci-summary` が `job-<result>` reason で Fail となり run 全体が失敗し tracker に伝播する。
- Given `ui-gpui-verify-macos` が success だが checkout-sha artifact を upload しなかった, When `ci-summary` が評価する, Then `ui-gpui-verify-macos:checkout-sha-missing` で Fail（宣言 leg の evidence 欠落は fail-closed）。
- Given `test-ui-gpui` が `--features "gpui-test uitest"` で実行される, When `cargo test` が完了する, Then `windowed_test_app` bin が Build/Link され、`windowed_uitest` test は env gate 未設定で no-op のまま success になる。

## Guarantees

- `actionlint` が workflow 構文を受理する。
- `mise run test-ci-summary`（`node --test ci/summary/`）が宣言 lane 追加後も pass する。
- `gh workflow run ci.yml --ref <branch> -f expected_sha=<sha>` の run で `ui-gpui-verify-macos` job が success になり、run ログで GPUI feature の check/lint/test と driver fixture の実行を確認できる。
- `ci-summary` verdict が PASS で、checkout SHA（全 leg 一致）と run URL が report JSON/step summary に記録される。
- 変更前後の CI walltime/runner 時間が記録され、既存 lane への重複ビルド追加がないことが確認できる。

## Non-goals

- macOS での `real-scenario`/`real_window`/pixel capture（GUI-L2/Deferred）。
- `ui-gpui-offscreen-verify`（Linux Xvfb）の macOS 展開。
- `docs/development-environment.md`・`daily-ci.yml`・`consumer-verify.yml`・`ci-issue-tracker*.yml` の変更。
- `build-full` の CI 接続（`test-full` が実行 bin の Build/Link を包含する検証済みのため不要 — design の棚卸し表を正本とする）。

## Risk

- macOS での gpui compile が CI 初走行で失敗する可能性 — `check-ui-gpui`（型）と `test-ui-gpui`（link/test）の step 単位で切り分け、crate 側原因なら別 Issue へ。
- `uitest` feature 併用の compile 相互作用 — cfg 衝突なし確認済みだが CI 実走で確定。失敗時は `test-ui-gpui` を `gpui-test` のみに戻し bin build を専用 step へ退避する fallback を検討。
- macOS runner の `abandoned`（取得 timeout）— 分離 job のため他 lane 証拠は保全され、fail-closed で検出・再実行可能。
