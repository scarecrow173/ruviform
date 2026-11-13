---
id: "SPEC-gE5E1nt62r02iDSRRElIQc"
type: "spec"
status: "approved"
title: "Issue #598 GPUI macOS Feature lane 追加と 3OS Native Build/Link の Daily CI 証拠補完"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/598"
  changes:
    added:
      - type: "file"
        path: "docs/specs/ci-os-l1-gpui-macos.md"
      - type: "file"
        path: "docs/designs/ci-os-l1-gpui-macos.md"
      - type: "file"
        path: "docs/plans/ci-os-l1-gpui-macos.md"
      - type: "file"
        path: "docs/tasks/ci-os-l1-gpui-macos.md"
      - type: "file"
        path: "docs/test-specs/ci-os-l1-gpui-macos.md"
      - type: "file"
        path: "docs/impl/ir/ci-os-l1-gpui-macos.md"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "ci/summary/summary.mjs"
      - type: "file"
        path: "ci/summary/tests/summary.test.mjs"
      - type: "file"
        path: "mise.toml"
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
  implements: []
  implemented-by:
    - "docs/plans/ci-os-l1-gpui-macos.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ci-os-l1-gpui-macos.md"
    - "docs/test-specs/ci-os-l1-gpui-macos.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ci-os-l1-gpui-macos.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #598 GPUI macOS Feature lane 追加と 3OS Native Build/Link の Daily CI 証拠補完

## What Should Be Built

`workflow_dispatch` 専用の `.github/workflows/ci.yml` に macOS 専用 job `ui-gpui-verify-macos` を追加し、`ui-gpui-verify`（ubuntu/windows matrix）と同一の検証範囲を macOS で実走する。併せて `ruviform-ui-gpui` の `uitest` feature を有効化して `windowed_test_app` AUT bin の Build/Link を 3 OS の GPUI lane で証明し、`test-full`（`cargo test --workspace`）の Native Build/Link 包含範囲を棚卸しして `build-full` 接続の要否を確定する。新 job は既存 lane と同一の checkout-sha evidence・`ci-summary` needs 集約・fail-closed 伝播経路に乗せる。

## Why Now

- `POLICIES.md` §2 は macOS の OS-L1（Native Build/Link・実行可能なテスト・利用 Feature）を必須とするが、GPUI adapter の `gpui`/`gpui-test` feature 検証は `ui-gpui-verify` の Windows/Linux 2 OS のみで、macOS lane が存在しない。driver-neutral 契約 fixture 群も macOS では未実行だった。
- ローカル必須検証は変更範囲比例へ移行済み（`POLICIES.md` §5）で、全 Workspace の `check-full`/`lint-full`/`test-full`/`build-full` は日次・手動 Full CI に委譲される。委譲先の CI が OS-L1 証拠として不足していると、ローカル省略の担保が崩れる。
- `test-ui-gpui` が `gpui-test` のみを有効化するため、`required-features = ["uitest"]` の `windowed_test_app` bin は CI のどの OS でも Build/Link されていない（手動 Windows lane のみ）。

## Users and Value

- 開発者・検証 Worker: ローカルで全 Workspace を回さなくても、対象 SHA の日次/手動 Full CI が 3 OS の OS-L1 証拠（型/Lint/Test・Native Build/Link・GPUI feature）を fail-closed で供給する。
- プロジェクト: `POLICIES.md` §5 の「ローカル検証の成功は統合受入の成功ではない」を支える CI 側の受け皿が OS・Feature の観点で完備され、Issue #294 の Cross-Consumer 候補受入が依拠する Ruviform 側証拠の欠落がなくなる。

## Scope

### In Scope

- `.github/workflows/ci.yml` への `ui-gpui-verify-macos` job 追加（macos-latest・`ui-gpui-verify` と同一 step 構成から Linux 依存導入 step を除いたもの）と `ci-summary.needs` への登録。
- `ci/summary/summary.mjs` の `REQUIRED_JOBS` / `REQUIRED_CHECKOUT_LEGS` への `ui-gpui-verify-macos` 登録と、`ci/summary/tests/summary.test.mjs` の lane 列挙の整合（rf594 で確立した宣言 lane 同期契約）。
- `mise.toml` の `test-ui-gpui` を `--features "gpui-test uitest"` へ拡張し、`windowed_test_app` bin の Build/Link を全 GPUI lane で証明（`windowed_uitest` test は `RVF_WINDOWED_UITEST` 未設定で no-op の既存契約を維持）。
- Native Build/Link 棚卸しの文書化（`test-full` が `cargo test --workspace` で実行 bin の Build/Link を包含することの検証結果と、`build-full` 接続不要の判断根拠）。
- doc-chain（spec/design/plan/task/test-spec/IR）と管理 index の再生成。
- 対象ブランチ SHA での手動 `ci.yml` Full CI 実走と 3 OS の OS-L1 結果・walltime 比較の記録。

### Out of Scope

- `crates/` 以下の実装コード、`mise.ci.toml`、他 workflow（`daily-ci.yml`・`consumer-verify.yml`・`ci-issue-tracker*.yml`）、`.devin/`、`POLICIES.md` の変更。
- `ui-gpui-verify` matrix への `macos-latest` 追加（macOS 分離 job の既存設計を維持 — Issue #333/#326 の starvation 分離理由と per-OS `needs` 粒度を保つ）。
- `real-scenario` feature の macOS 展開（Xvfb offscreen は Linux 専用設計。macOS の real-window/VisualCapture は GUI-L2/Deferred 方針に従い本 Issue の OS-L1 対象外）。
- `ruviform-poc-native-dock` の `gpui`/`vst3` feature lane の CI 常設化（Windows 手動 PoC lane の既存設計を維持。default feature の bin コードは `test-full` の test-harness Build/Link でカバー済み）。
- `docs/development-environment.md` 等の lane 一覧記述の更新（編集範囲外。残件として PR に記録）。
- GUI-L2/EXT-L3 の受入（macOS/Linux Deferred、Windows 実機受入は別 Owner・別 Issue）。

## Requirements

1. `ui-gpui-verify-macos` job は `ui-gpui-verify` と同一の検証 step（driver boundary guard・driver contract・parity・native slot・component/view・golden・scenario contract・`check-ui-gpui`・`lint-ui-gpui`・`test-ui-gpui`・`test-ui-consumer-observation`）を macos-latest で実行し、Linux 専用の依存導入 step は持たない。全 lane 共通の checkout-sha evidence 契約（`CHECKOUT_LEG=ui-gpui-verify-macos`・実測 sha artifact）を満たす。
2. `ci-summary.needs`・`summary.mjs` `REQUIRED_JOBS`・`REQUIRED_CHECKOUT_LEGS`・`summary.test.mjs` 列挙の 4 箇所が `ui-gpui-verify-macos` で一致し、needs からの削除・evidence 欠落・失敗が fail-closed で検出される。
3. `test-ui-gpui` は `gpui-test` + `uitest` feature で `ruviform-ui-gpui` の test target を実行し、`windowed_test_app` bin の Build/Link を暗黙に含む。`RVF_WINDOWED_UITEST` 未設定時の no-op 契約と `real-scenario` との非併用契約を維持する。
4. `test-full`（`cargo test --workspace`）が workspace の実行 bin の Build/Link を包含することを検証済みとし、包含されない bin が残る場合のみ最小の接続を行う。重複ビルドを無条件に増やさない。
5. 変更前後の CI walltime/runner 時間を比較し、結果を記録する。
6. 最新 main を取り込み、doc-chain と管理 index を再生成してコミットに含める。
7. lane 削除・skip、既存 job の検証意図を損なう変更、`NotRun`/`Skipped`/`Fail` を Pass にする緩和を行わない。

## Acceptance Criteria

- AC1: Windows/macOS/Linux それぞれの対象 SHA に対し、Native Build/Link（実行 bin を含む）と Workspace 型/Lint/Test の OS-L1 証拠が既存 CI から得られる。`test-full` の Build/Link 包含検証結果が文書化され、欠落時のみ `build-full` 等が接続される。
- AC2: `ruviform-ui-gpui` の `gpui`/`gpui-test`（および `uitest` bin Build/Link）を macOS でも `check`・`clippy`・headless テストで検証し、既存 Windows/Linux GPUI レーンの範囲を維持する。
- AC3: `ci-summary` が各 OS/Feature の fail-closed 証拠を統合し、必要 case の NotRun/Skipped/Fail を Pass にしない。checkout SHA と CI run URL を確認できる。
- AC4: 変更前後の CI walltime/runner 時間を比較し、既存の重複ビルドを無条件に増やしていないことを記録する。
- AC5: 対象 SHA の手動 `ci.yml` Full CI を実走し、3 OS の OS-L1 検証結果を run URL と共に記録する。

## Deferred Design Concerns

- macOS での `real-scenario`/`real_window` 実ウィンドウ・pixel capture | macOS の GUI-L2/VisualCapture は Deferred であり、offscreen golden は Linux Xvfb 設計に依存 | macOS で GUI 機能の利用要件が発生したとき | macOS の real-window 受入経路が未整備のまま残る（現行方針どおり OS-L1 のみ必須）
- `poc-native-dock` の `gpui`/`vst3` feature bin の CI Build/Link | 当該 bin は Windows 手動 PoC lane が正本で、default feature のコードは `test-full` の test-harness link でカバー済み | PoC を CI 検証対象へ昇格する判断があったとき | feature 有効時の bin link が CI 証拠に含まれない状態が継続する
- GPUI feature の `check`/`lint`/`test` task の feature 組合せ総当たり | `gpui`/`gpui-test`/`uitest` の CI 使用組合せは lane で固定済み。`real-scenario` は offscreen lane、`uitest` run は手動 lane が担う | feature 組合せを増やす Issue が起票されたとき | 未使用の feature 組合せの破損は CI で検出されない

## Sources and Relations

- Issue #598（本 spec の source）。
- 検証方針の正本: `POLICIES.md` §2（OS-L1/GUI-L2/EXT-L3）・§4（既存 Daily/手動 Full CI の利用）・§5（ローカル検証と Daily CI の責務分離）。
- 機構の正本: Issue #366 Stage 1 の checkout-sha evidence / `ci-summary` fail-closed 集約（`ci/summary/summary.mjs`）、`docs/designs/issue-366-full-ci-stage1.md`。
- macOS 分離 job の先行: Issue #333/#326 rev 2026-11-05（`verify-macos`/`ui-verify-macos`/`l1-real-verify-macos`）。
- 同型参照: rf594 `msrv-verify` lane 追加（`docs/specs/ci-msrv-daily-lane.md`）— 宣言 lane の 4 箇所同期契約。
