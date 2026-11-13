---
id: "IMPL-034WR1i5LXEheCHSySWZsA"
type: "impl"
status: "completed"
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
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/capture.rs"
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
- **macOS 限定 lint 欠陥の修正（coordinator 承認の範囲拡張）**: 初回 Full CI（run 43421898733, `expected_sha` = f3790d3）で `ui-gpui-verify-macos` が `lint-ui-gpui` の `clippy::doc_markdown` で失敗 — `capture.rs:883` の doc comment が `NSView` をバックティック無しで記述しており、`#[cfg(not(any(windows, target_os = "linux")))]` の macOS 専用 fallback module 内のため Linux/Windows lane では一度も lint されていなかった潜在欠陥。crate 全体の cfg 監査で macOS 固有の compile surface はこの 1 module のみと確認し、clippy 提示の修正（`NSView` をバックティックで囲む comment-only 変更）を `crates/` 範囲拡張の承認を得て適用。`#[allow]` による抑制・コード変更なし。新 lane が macOS 限定の潜在欠陥を正しく fail-closed で検出したこと自体が本 Issue の証拠価値を実証した。

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
- `mise run lint ruviform-ui-gpui`（capture.rs 修正後・coordinator 指定の scoped lint）→ exit 0（Windows 11 x64）。macOS cfg 節は Windows では compile されないため lint 実効は CI 実走で確認。
- doc-maintenance `index apply`（docs/specs, designs, plans, tasks, test-specs）→ 全 dir で written/blocked なし（index は既に同期）。`docs/impl/ir` は `sync_impl_index.js --kind ir --apply` で reconcile 済み。
- `git diff --check` → 問題なし（prek pre-commit hook も全 commit で pass）。
- `gh workflow run ci.yml` 1 回目（run [43421898733](https://github.com/scarecrow173/ruviform/actions/runs/43421898733), `expected_sha` = f3790d3b = doc-chain + CI 配線 commit）→ **failure（fail-closed 正常動作）**。`ui-gpui-verify-macos` が `lint-ui-gpui` の `clippy::doc_markdown`（capture.rs:883 `NSView`）で失敗し、`ci-summary` が `ui-gpui-verify-macos:job-failure` で Fail — 他 18 job は全て success（`uitest` feature 拡張による `test-ui-gpui` は ubuntu/windows GPUI lane で成功 = bin Build/Link が 2 OS で実証済み）。walltime 11m21s。
- `gh workflow run ci.yml` 2 回目（run [43424042262](https://github.com/scarecrow173/ruviform/actions/runs/43424042262), `expected_sha` = 50695a4f = NSView 修正込みブランチ先端）→ **success**。全 20 job（15 declared lane + matrix leg + ci-summary）success。`ui-gpui-verify-macos` success（8m35s — `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`（`gpui-test uitest` で `windowed_test_app` Build/Link）/`test-ui-consumer-observation` 全 step success、checkout-sha evidence upload 確認）。`ci-summary` verdict **PASS** — `ui-gpui-verify-macos` declared=true/result=success/Pass、checkout leg `ui-gpui-verify-macos` sha=`50695a4f…` 一致 Pass、event/expected/summary-checkout SHA 3 者一致、run URL 記録。3 OS の `verify`/`verify-macos`（ubuntu/windows/macos）と全 GPUI lane（ubuntu/windows/macos + offscreen）の OS-L1 証拠を対象 SHA で取得。

### Walltime / runner 時間の比較（AC4）

| 指標 | 変更前（run 43385858552, success） | 変更後（run 43424042262, success） | 差分 |
| --- | --- | --- | --- |
| run walltime | 8m29s | 13m44s | +5m15s |
| critical path | `ui-gpui-verify (windows)` 8m11s | `ui-gpui-verify (windows)` 12m34s | +4m23s |
| `ui-gpui-verify-macos` | —（未存在） | 8m35s | 新規 1 job |
| runner 時間合計（概算） | ~46 min（19 job） | ~62 min（20 job） | +~16 min |

増分の内訳: (a) 新規 macOS GPUI lane 8m35s（OS-L1 必須証拠の欠落を埋める最小の 1 job — macOS runner は課金倍率が高いが他 lane の step 複製以上の重複なし）。(b) `test-ui-gpui` の `uitest` 追加による feature 組合せ変更で GPUI 系 3 lane の compile が増加 — windows leg +4m23s・ubuntu leg +1m10s・macOS leg は初回 cold のため 8m35s。`uitest` は `gpui`/`gpui-test` と feature 統合されるため gpui 本体の重複 compile は発生せず、増分は `ruviform-ui-gpui` 自身（uitest module + `windowed_test_app` bin link）に限定される。初回は新 feature 組合せで sccache key が cold のため以後の run で短縮が見込めるが、実測として walltime +5m15s・runner +~16 min を記録する（設計見積りの macOS lane 5〜10 分の範囲内）。

## Risks

- **macOS での gpui compile/lint**: 実走で顕在化した `doc_markdown` 欠陥は修正済みで 2 回目 run で green。gpui 本体の macOS backend compile は初回から成功（`check-ui-gpui` step success — Metal/Cocoa 系 deps は runner SDK で問題なく build された）。
- **`uitest` 併用の実機相互作用**: 3 OS の `test-ui-gpui` が全て success — `windowed_uitest` は env gate で no-op、`windowed_test_app` は Build/Link のみで実行されない（GUI-L2 主張なし）。
- **walltime 増**: 上記比較のとおり +5m15s（critical path は windows GPUI leg の初回 cold compile が主因）。新規重複ビルドはなく、sccache 温存後の定常値は下がる見込み。
- **macOS runner starvation**: 今回は `abandoned` なく 8m35s で完了。発生時は分離 job のため他 lane 証拠は保全され `ci-summary` が fail-closed で検出する（意図どおり）。

## Follow-ups

- ~~対象 SHA の手動 `ci.yml` Full CI 実走~~ → **完了**（run 43424042262 success・ci-summary PASS・3 OS OS-L1 証拠取得。初回 run 43421898733 で macOS 限定 lint 欠陥を検出→修正→再実走で green）。
- `docs/development-environment.md` の lane 構成記述を新 lane 数に合わせて更新する別タスク（編集範囲外のため本タスクでは未対応）。
- `POLICIES.md` L3-4 の行末空白（main 既存欠陥・coordinator 事前通知）: run 43424042262 の `verify` 系 lane（prek `trailing-whitespace` 含む）が全て success で実害なしを実測確認 — 本タスクでは未着手のまま。
