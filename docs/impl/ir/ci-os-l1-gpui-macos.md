---
id: "IMPL-034WR1i5LXEheCHSySWZsA"
type: "impl"
status: "in-progress"
title: "Issue #598 GPUI macOS Feature lane and 3OS native build/link CI evidence"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/598"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/ci-os-l1-gpui-macos.md"
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
  implements:
    - "docs/tasks/ci-os-l1-gpui-macos.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/ci-os-l1-gpui-macos.md"
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

# Issue #598 GPUI macOS Feature lane and 3OS native build/link CI evidence

## Summary

`ci.yml`（workflow_dispatch 専用・daily-ci 経由で日次 dispatch）に macOS 専用 job `ui-gpui-verify-macos` を追加し、`ui-gpui-verify`（ubuntu/windows matrix）と同一の検証範囲 — driver-neutral 契約 fixture 群 + `gpui`/`gpui-test` feature lane — を macos-latest で実走する。併せて `mise.toml` の `test-ui-gpui` を `--features "gpui-test uitest"` へ拡張し、`required-features = ["uitest"]` の `windowed_test_app` bin の Build/Link を全 GPUI lane（3 OS）で証明する。`windowed_uitest` test target は `RVF_WINDOWED_UITEST=1` gate の既存 no-op 契約を維持し、GUI-L2 の主張は行わない。fail-closed 集計は `ci-summary.needs`・`ci/summary/summary.mjs` の `REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS`・`summary.test.mjs` 列挙の 4 箇所同期で登録した（rf594 で確立した宣言 lane 契約）。

## Context

Issue #598（RF-CI-L1-01）の受入条件に基づく。`POLICIES.md` §2 は macOS の OS-L1（Native Build/Link・実行可能なテスト・利用 Feature）を必須とするが、GPUI adapter の feature 検証は `ui-gpui-verify` の Windows/Linux 2 OS のみで macOS lane が存在しなかった。また `test-ui-gpui` が `gpui-test` のみを有効化するため `windowed_test_app` bin は CI のどの OS でも Build/Link されていなかった。ローカル必須検証は変更範囲比例（`POLICIES.md` §5）へ移行済みで、全 Workspace 検証は日次/手動 Full CI に委譲されるため、委譲先 CI の OS-L1 証拠欠落を本変更で補完する。

## Implementation

### Changes

#### `.github/workflows/ci.yml`

- `ui-gpui-verify` の直後に `ui-gpui-verify-macos` job を追加。`runs-on: macos-latest`、`CARGO_INCREMENTAL: "0"`、`CHECKOUT_LEG: ui-gpui-verify-macos`。
- checkout は `${{ github.sha }}` pin、実測 `git rev-parse HEAD` を `checkout-sha-ui-gpui-verify-macos-attempt-<n>.txt` として upload（既存 lane と同一の Issue #366 sha evidence 契約）。
- step 構成は `ui-gpui-verify` と同一: mise 復元（`mise trust` + `mise install --locked rust prek`）、cargo registry cache（key `${{ runner.os }}-cargo-ui-gpui-${{ hashFiles('Cargo.lock') }}`・restore-keys `${{ runner.os }}-cargo-ui-gpui-`）、`cargo fetch --locked`（boundary guard の `cargo metadata --offline` 対策）、sccache 3 step（kill switch `CI_SCCACHE_DISABLE` 込み）、検証 step 11 件（`check-ui-driver-boundary` → `test-ui-driver-contract` → `test-ui-parity` → `test-ui-native-slot` → `test-ui-component-view` → `test-ui-golden` → `test-ui-scenario-contract` → `check-ui-gpui` → `lint-ui-gpui` → `test-ui-gpui` → `test-ui-consumer-observation`）→ sccache statistics。
- `Install GPUI Linux build/runtime dependencies` step のみ除去 — macOS 専用 job では常に偽の dead step であり、`verify-macos` が ALSA/disk-usage step を落とした先例に倣う。GPUI の macOS backend（Metal/Cocoa）は runner 同梱 SDK で賄え、追加 system package は不要。
- `ci-summary.needs` に `ui-gpui-verify-macos` を `ui-gpui-verify` の直後へ追加。

#### `ci/summary/summary.mjs`

- `REQUIRED_JOBS` に `"ui-gpui-verify-macos"`（`"ui-gpui-verify"` の直後）、`REQUIRED_CHECKOUT_LEGS` に `"ui-gpui-verify-macos"`（`"ui-gpui-verify-windows-latest"` の直後）を追加。job 未登録・非 success 結果・checkout evidence 欠落が既存 lane と同じく fail-closed で検出される。

#### `ci/summary/tests/summary.test.mjs`

- required-job 列挙 test と checkout-leg 列挙 test の双方に `ui-gpui-verify-macos` を追加。helper が export された required 配列から入力を導出するため、pass/fail/missing-evidence の全 matrix が新宣言を自動的にカバーする。

#### `mise.toml`

- `test-ui-gpui` を `cargo test -p ruviform-ui-gpui --features "gpui-test uitest"` へ変更し、description に bin Build/Link の意図と `RVF_WINDOWED_UITEST` gate を追記。`uitest` feature により `required-features = ["uitest"]` の `windowed_test_app` bin が `cargo test` の CARGO_BIN_EXE 用 build で link される。`gpui-test`/`uitest` の併用は禁止組合せではなく（禁止は `real-scenario` × `gpui-test` のみ）、cfg 面の衝突もないことを確認済み。

#### doc-chain（`docs/{specs,designs,plans,tasks,test-specs}/ci-os-l1-gpui-macos.md` + 本 IR）

- spec → design → plan → task → test-spec → IR の canonical chain を作成。Native Build/Link 棚卸し（`test-full` 包含検証）と walltime 見積りは design doc に集約。

### Decisions During Implementation

- **matrix ではなく物理分離 job**: `ui-gpui-verify` の matrix に `macos-latest` を足すと checkout leg が `ui-gpui-verify-macos-latest` となり既存 `*-macos` 命名と不整合。より本質的には Issue #333/#326 が macOS leg を分離した理由（macOS runner starvation が他 OS lane を道連れにしない + `needs` 経由の per-OS result 粒度）がそのまま当てはまる。matrix leg は `needs.<job>.result` が 1 値に集約され失敗 OS を判別できない。
- **`build-full` は接続しない**: `test-full`（`cargo test --workspace`）は workspace の実行 bin の Build/Link を実質包含する（全 test exe + tests/ あり package の通常 bin + bin test-harness が各 OS で link）。唯一の空白だった `uitest` gate の `windowed_test_app` は `test-ui-gpui` の feature 拡張で塞ぎ、重複ビルドを増やさない。棚卸し表は design doc 参照。
- **`check-ui-gpui`/`lint-ui-gpui` は `--features gpui` のまま**: `uitest` コードの compile+link は test lane が担保し、`real-scenario` が CI lint 対象外である既存先例と同じ整理。feature 組合せを増やすと gpui 本体の再 compile が発生するため、`gpui-test`+`uitest` の 1 組合せに統一した。
- **`docs/development-environment.md` の lane 記述は未更新**: 編集範囲外のため残件として PR に記録（rf594 と同じ扱い）。
- **ローカルでの `cargo check -p ruviform-ui-gpui --features "gpui-test uitest" --all-targets`**: Windows ローカルで exit 0 を確認（補助証拠 — canonical は mise task）。feature 組合せの compile 可能性を CI 実走前にローカルで確認した。

## Related Experiments

### Adopted

- なし（実験 log は作成していない — 設計判断は design doc に集約）。

### Rejected

- なし。

## Validation

実施済み:

- `mise exec -- actionlint`（変更後）→ exit 0。
- `mise run test-ci-summary` → exit 0（31 tests / 31 pass / 0 fail / 0 skipped / 0 cancelled）。
- `mise tasks ls` / `mise task test-ui-gpui` → `mise.toml` が parse 可能で、新しい run コマンド `--features "gpui-test uitest"` が正しく解決されることを確認。
- `mise exec -- cargo check -p ruviform-ui-gpui --features "gpui-test uitest" --all-targets`（補助証拠 — canonical は mise task）→ exit 0（Windows 11 x64・rust 1.99.1・mbx managed target）。`gpui-test`+`uitest` 組合せの型検査が通ることを確認。
- doc-maintenance `index apply`（docs/specs, designs, plans, tasks, test-specs）→ 全 dir で written/blocked なし（index は既に同期）。`docs/impl/ir` は sync_impl_index 経路で別途 reconcile 予定。
- `git diff --check` 相当の空白検査はコミット前に実施予定。

実走予定（Issue 受入条件）:

- `gh workflow run ci.yml --ref chore/598-ci-os-l1-gpui-macos -f expected_sha=<対象SHA>` で手動 Full CI を実走し、`ui-gpui-verify-macos`・3 OS `verify`/`verify-macos`・既存 GPUI lane・`ci-summary` の結果を run URL・checkout SHA evidence・walltime 比較と共に記録する。

## Risks

- **macOS での gpui compile 失敗**: `gpui` feature の macOS backend 依存（Metal/Cocoa 系）は CI で初めて compile される。失敗時は step 単位で `check-ui-gpui`（型）と `test-ui-gpui`（link/test）を切り分け、原因が crate 側なら別 Issue へ切り出す（本タスクの範囲は CI 配線）。
- **macOS runner starvation**: `verify-macos` で `abandoned`（runner 取得 timeout）の前例あり。`ui-gpui-verify-macos` も同リスクを負うが、分離 job のため他 lane の証拠を道連れにせず、`ci-summary` が `job-abandoned` で fail-closed に検出する（意図どおりの挙動）。
- **`uitest` 併用の実機相互作用**: cfg 面の衝突は確認済みでローカル `cargo check --all-targets` も通過したが、macOS runner での compile/link は CI 実走でのみ確定する。
- **walltime 増**: macOS 分は課金倍率が高いが、OS-L1 必須証拠の欠落を埋める最小の 1 job。sccache GHA backend が `verify-macos`/`ui-verify-macos` の workspace compile 済み artifact を部分再利用するため、gpui 固有依存以外の compile は軽い見込み。実測は CI run で記録する。

## Follow-ups

- 対象 SHA の手動 `ci.yml` Full CI 実走 → `ui-gpui-verify-macos` success・3 OS lane 非退行・`ci-summary` PASS を確認し、run URL・walltime 比較を本 IR と PR に記録（Issue #598 AC5）。
- `docs/development-environment.md` の lane 構成記述を新 lane 数に合わせて更新する別タスク（編集範囲外のため本タスクでは未対応）。
