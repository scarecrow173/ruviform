---
id: "DESIGN-MSdMxhOZx7fphrbdNWWeNB"
type: "design"
status: "approved"
title: "Issue #598 設計 — ui-gpui-verify-macos lane の構成と Native Build/Link 棚卸し"
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
        path: "docs/designs/ci-os-l1-gpui-macos.md"
    modified: []
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
    - "docs/specs/ci-os-l1-gpui-macos.md"
  implemented-by: []
  depends-on:
    - "docs/specs/ci-os-l1-gpui-macos.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/ci-os-l1-gpui-macos.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ci-os-l1-gpui-macos.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/598"
  defers: []
  deferred-by: []
---

# Issue #598 設計 — ui-gpui-verify-macos lane の構成と Native Build/Link 棚卸し

## Overview

`ci.yml` に `ui-gpui-verify-macos` job を 1 本追加し、`ui-gpui-verify`（ubuntu/windows matrix）と同一の検証 step を macos-latest で実行する。job は既存 lane と同一の定型を踏む: checkout を `github.sha` に pin → 実測 checkout SHA を `checkout-sha-ui-gpui-verify-macos-attempt-<n>.txt` として artifact upload（Issue #366 sha evidence）→ mise 復元 → cargo registry cache（gpui 専用 prefix）→ `cargo fetch --locked` → sccache → driver-neutral 契約 fixture 群 → GPUI feature lanes（`check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`/`test-ui-consumer-observation`）。終端 `ci-summary` の `needs` に追加し、評価器 `ci/summary/summary.mjs` の `REQUIRED_JOBS` / `REQUIRED_CHECKOUT_LEGS` と `summary.test.mjs` 列挙へ同期登録する。

## Lane job design

- **分離 job を選ぶ理由（matrix 非採用）**: `ui-gpui-verify` の matrix に `macos-latest` を足すと checkout leg 名が `ui-gpui-verify-macos-latest` となり、既存の `verify-macos`/`ui-verify-macos`/`l1-real-verify-macos` の `-macos` 命名と不整合になる。より本質的には、Issue #333/#326 rev 2026-11-05 が macOS leg を物理分離した理由（macOS runner 取得 starvation が他 OS の必須 context を道連れにしない構造 + `needs` 経由の per-OS result 粒度）がそのまま当てはまる。matrix leg は `needs.<job>.result` が 1 値に集約され、どの OS が失敗したかを `needs` だけでは判別できない。分離 job なら `ui-gpui-verify-macos` が独立した fail-closed 判定単位になる。Stage-2 の matrix 統合は引き続き deferred。
- **runs-on / timeout**: `macos-latest` 単独。`timeout-minutes` は既存 lane がいずれも未設定のため付けない。
- **step 構成**: `ui-gpui-verify` の step をそのまま踏襲し、`Install GPUI Linux build/runtime dependencies`（`if: runner.os == 'Linux'`）のみ除去する — macOS 専用 job では常に偽の dead step であり、`verify-macos` が ALSA/disk-usage step を落とした先例に倣う。GPUI の macOS backend（Metal/Cocoa/CoreGraphics 系）は Xcode 同梱 SDK で賄え、追加の system package は不要。
- **mise 復元・cache**: `mise trust` + `mise install --locked rust prek`、cargo registry cache は key `${{ runner.os }}-cargo-ui-gpui-${{ hashFiles('Cargo.lock') }}`・restore-keys `${{ runner.os }}-cargo-ui-gpui-` — `ui-gpui-verify` と同一 prefix 慣行（`runner.os` が macOS で分岐するため OS 間で cache は混ざらない）。
- **`cargo fetch --locked`**: boundary guard が `cargo metadata --offline` を実行するため、冷キャッシュ対策として `ui-gpui-verify` と同じく全 lockfile 依存を事前 fetch する（run 43270011255 の既知障害への既存対策を踏襲）。
- **sccache**: `verify-macos`/`ui-verify-macos`/`ui-gpui-verify` と同一の 3 step（sccache-action・github-script env export・verify）。`SCCACHE_GHA_VERSION` は `RUNNER_OS` 由来で OS 別に分離される既存機構のまま。kill switch `CI_SCCACHE_DISABLE` も同一。
- **検証 step**: `check-ui-driver-boundary` → `test-ui-driver-contract` → `test-ui-parity` → `test-ui-native-slot` → `test-ui-component-view` → `test-ui-golden` → `test-ui-scenario-contract` → `check-ui-gpui` → `lint-ui-gpui` → `test-ui-gpui` → `test-ui-consumer-observation` → sccache statistics。fixture 群は `ruviform-ui-test`（fake layer・GPUI 非依存）だが macOS では現行 CI のどの lane でも未実行であり、GPUI lane family の一部として同時に OS-L1 証拠へ載せる。`test-ui-scenario-contract`/`test-ui-consumer-observation` は `gpui-test` feature を要求し、macOS での GPUI headless compile/link を兼ねる。
- **macOS での GPUI compile 前提**: `gpui` feature の optional deps のうち `windows-sys`/`x11rb` は target-gated（cfg(windows)/cfg(linux)）のため macOS では解決対象外 — これが本 Issue の検証対象そのものであり、feature 定義の変更は不要（既に 3 OS で正しく gate されている）。`gpui-test` は `gpui/test-support` の TestAppContext 系で実 OS window/GPU を必要とせず、macOS runner で headless に動く設計（`golden_capture.rs` は `#![cfg(all(windows, feature = "gpui"))]`、`real_window.rs` は `real-scenario` 必須で本 lane では skip、`windowed_uitest.rs` は `uitest` + `RVF_WINDOWED_UITEST` gate で no-op）。

## Native Build/Link 棚卸し（test-full 包含検証）

`test-full` = `cargo test --workspace`（`--all-targets` ではない）。`cargo test` の target 選択は lib/bins/tests + doc tests で、example・bench・`required-features` 未充足 target は対象外。実測（最小 project で確認）: integration test を持つ package では通常 bin 実行ファイル（`target/debug/<bin>`）も生成され、bin は test-harness 版・通常版の双方が link される。integration test を持たない package の bin は test-harness 版のみ link される。

workspace の実行 bin 棚卸しと CI カバー状況:

| bin | crate | feature gate | CI での Build/Link |
| --- | --- | --- | --- |
| `ruviform-cli-sample`（main） | cli-sample | なし | `test-full` 通常 bin（tests/ あり）— 3 OS |
| `ruviform-plugin-l1-runner` / `l1-video-runner` | plugin-export | なし | `test-full` 通常 bin — 3 OS |
| `l1-test-worker` / `l1-test-video-worker` | plugin-host | なし | `test-full` 通常 bin — 3 OS |
| `dummy_app` / `gui_boundary_guard` | ui-test | なし | `test-full` 通常 bin — 3 OS |
| `theme-codegen` | ui-theme | なし | `test-full` 通常 bin — 3 OS |
| `ruviform-poc-native-dock`（main） | poc-native-dock | なし（default） | `test-full` test-harness bin（tests/ なしのため通常 bin は生成されないが、bin コードの compile+link は実施）— 3 OS |
| `ruviform-plugin-clap-l1-runner` | plugin-clap | `host` | `test-l1-real-plugin` の `cargo build --features host` — l1-real-verify 3 OS |
| `ruviform-plugin-vst3-l1-runner` | plugin-vst3 | `host` | 同上 — l1-real-verify 3 OS |
| `windowed_test_app` | ui-gpui | `uitest` | **従来: どの CI lane でも未構築 → 本変更で `test-ui-gpui` 拡張により 3 OS の GPUI lane で Build/Link** |

example target（ui-gpui 8 件・ui-webview 3 件・observability 1 件）は `[[example]]` であり「実行 bin」ではない。型検査は `lint-full`（`--all-targets`・default feature）と各 feature lane の `check-*/lint-*`（`--all-targets --features <f>`）でカバー済み。link 証拠は要求しない。

**結論**: `test-full` は workspace の実行 bin の Build/Link を実質包含しており（全 test exe + 通常 bin + bin test-harness が各 OS で link される）、`build-full` の追加接続は不要。唯一の空白だった `uitest` gate の `windowed_test_app` bin は `test-ui-gpui` の feature 拡張で塞ぐ。`poc-native-dock` の `gpui`/`vst3` feature bin は Windows 手動 PoC lane が正本であり、default feature の bin コードは test-harness link でカバー済みのため deferred とする（spec の Deferred Design Concerns に記録）。

## test-ui-gpui の feature 拡張

- 変更: `run = "cargo test -p ruviform-ui-gpui --features \"gpui-test uitest\""`。description に bin Build/Link の意図を追記。
- `uitest` = `gpui` + `dep:ruviform-ui-test` + `dep:base64`。`gpui-test` との併用は Cargo.toml 上の禁止組合せではない（禁止は `real-scenario` × `gpui-test` のみ — test platform が `capture_raw_parent` を stub するため）。`#[cfg]` 面でも両 feature は独立 gate で競合しない（`compile_error!`/`all(feature=...)` 衝突なしを確認済み）。
- 効果: `cargo test` は integration test を持つ package の通常 bin を CARGO_BIN_EXE 用に build するため、`windowed_test_app`（required-features=uitest 充足）が `target/debug/` に link される。`windowed_uitest.rs` test target もコンパイルされるが、`RVF_WINDOWED_UITEST` 未設定では即 return の既存 no-op 契約（`tests/windowed_uitest.rs:192`）により lane の成否に影響しない。
- コスト: `base64`（極小）+ uitest module + bin link のみ。gpui 本体の feature 組合せは `gpui/test-support` 有効の 1 種に統一され、`cargo build --features uitest` を別途走らせる場合の gpui 再 compile を回避できる。
- `check-ui-gpui`/`lint-ui-gpui` は `--features gpui` のまま変更しない — `real-scenario` feature が CI lint 対象外である既存先例（offscreen lane は compile のみ）と同じ整理で、uitest コードは test lane の compile+link で担保する。

## ci-summary 集計への組み込み

- `ci-summary.needs` に `ui-gpui-verify-macos` を追加（`ui-gpui-verify` の直後）。`NEEDS_JSON` 経由で `needs.ui-gpui-verify-macos.result` が評価器に届き、success 以外は既存 job と同じ `job-<result>` reason で Fail → run 失敗 → `ci-issue-tracker` の `workflow_run` 経路で tracker に伝播する。
- `ci/summary/summary.mjs` の `REQUIRED_JOBS` に `"ui-gpui-verify-macos"`（`"ui-gpui-verify"` の直後）、`REQUIRED_CHECKOUT_LEGS` に `"ui-gpui-verify-macos"`（`"ui-gpui-verify-windows-latest"` の直後）を追加。「kept in sync with ci.yml needs list」契約を維持し、宣言 lane として fail-closed 検出を有効化する。
- `ci/summary/tests/summary.test.mjs` の 2 つの lane 列挙テストに `ui-gpui-verify-macos` / `ui-gpui-verify-macos`（leg）を追加。
- tracker 連携（`ci-issue-tracker.yml`）は run 結論単位で評価するため変更不要。

## Walltime / runner 時間の見積り

ベースライン: 直近の全 lane 成功 run 43385858552（rf594 ブランチ・14 job + ci-summary、約 8.5 分）。最長 lane は `ui-gpui-verify (windows-latest)` 約 8.2 分。

- 追加: `ui-gpui-verify-macos` 1 job。macOS runner は Windows より概ね低速だが、sccache GHA backend は repo 共通で `verify-macos`/`ui-verify-macos` の workspace compile 済み artifact を部分的に再利用できる（gpui feature 固有の依存は冷たい）。想定 5〜10 分。run 全体の walltime は現行 critical path（windows gpui leg ~8 分）を超えなければ不変。
- `test-ui-gpui` の `uitest` 追加コスト: base64 + uitest module + bin link のみで、既存 3 lane（ubuntu/windows/macOS）の各 step に数十秒程度。新規の重複ビルドは発生しない（feature 組合せは 1 種のまま）。
- runner 時間: macOS 分は課金倍率が高いが、OS-L1 必須証拠の欠落を埋める最小の 1 job であり、既存 lane の step を複製する以外の重複はない。

## File map

- 変更: `.github/workflows/ci.yml`（`ui-gpui-verify-macos` job 追加 + `ci-summary.needs` 更新）、`ci/summary/summary.mjs`（2 リストに `ui-gpui-verify-macos` 追加）、`ci/summary/tests/summary.test.mjs`（2 列挙に追加）、`mise.toml`（`test-ui-gpui` の feature 拡張 + description）
- 新規: doc-chain 6 件（spec/design/plan/task/test-spec/IR）
- 再生成: `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md`（管理 index）
- 変更なし確認: `daily-ci.yml`/`consumer-verify.yml`/`ci-issue-tracker*.yml`/`mise.ci.toml`/`crates/`/`POLICIES.md`/`docs/development-environment.md`（範囲外・残件）

## Risks

- **macOS での gpui compile 失敗**: `gpui` feature の macOS backend 依存（Metal/Cocoa 系 crates）は CI で初めて compile される。失敗時は run ログの step 単位で `check-ui-gpui`（型）と `test-ui-gpui`（link/test）を切り分け、原因が crate 側なら別 Issue へ切り出す（本タスクの範囲は CI 配線）。
- **`uitest` 併用の予期しない相互作用**: cfg 面の衝突は確認済みだが、実 compile は CI 実走でのみ確定する。失敗時は `test-ui-gpui` の feature 組合せを `gpui-test` のみに戻し、bin Build/Link を専用 step（`cargo build --features "gpui-test uitest" --bins`）へ退避する fallback を検討する。
- **macOS runner starvation**: `verify-macos` で `abandoned`（runner 取得 timeout）の前例あり。`ui-gpui-verify-macos` も同リスクを負うが、分離 job のため他 lane の証拠を道連れにせず、`ci-summary` が `job-abandoned` で fail-closed に検出する（これは意図どおりの挙動）。
- **並行 Worker との index 競合**: merge 後に index を再生成して同一コミットに含める。生成物のみの競合は merge → 再生成 → push で解消（フルレーン再検証不要）。
