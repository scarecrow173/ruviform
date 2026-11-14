---
id: "SPEC-034Xzvx8VyUUbiqyFokB52"
type: "spec"
status: "proposed"
title: "GPUI Native First採用Gate verdict記録・OS/性能/互換性評価（Issue #262）"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/synvelis/issues/176"
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "docs/designs/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "docs/test-specs/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "docs/plans/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "docs/tasks/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "docs/impl/ir/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "docs/impl/exp/issue-262-gpui-verdict.md"
      - type: "doc"
        path: "evidence/rf262-gpui-verdict/report.md"
      - type: "doc"
        path: "evidence/rf262-gpui-verdict/verdict-matrix.md"
      - type: "doc"
        path: "evidence/rf262-gpui-verdict/handoff-419.md"
      - type: "doc"
        path: "evidence/rf262-gpui-verdict/verification.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by:
    - "docs/designs/issue-262-gpui-verdict.md"
    - "docs/plans/issue-262-gpui-verdict.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-465-legacy-webview-p0p1.md"
    - "docs/specs/issue-465-legacy-webview-p0p1.md"
    - "docs/designs/ci-os-l1-gpui-macos.md"
    - "evidence/rf388-os-l1-ci/report.md"
    - "evidence/rf465/README.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/262"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-262-gpui-verdict.md"
  references:
    - "evidence/rf262-gpui-verdict/report.md"
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/handoff-419.md"
    - "evidence/rf262-gpui-verdict/verification.md"
    - "evidence/rf388-os-l1-ci/handoff-262-419.md"
    - "evidence/rf388-os-l1-ci/report.md"
  defers: []
  deferred-by: []
---

# GPUI Native First採用Gate verdict記録・OS/性能/互換性評価（Issue #262）

## Problem

Issue #262 は「GPUI Native First を標準 Driver とし、公開 UI 契約を Driver-neutral とする」既定設計（1 件目の受入条件、確定済み）に対し、A〜E の採用 Gate（A: 依存非漏出・Fake/GPUI semantic fixture、B: 実 OS Window/Focus/IME/DPI/a11y/2Window、C: Component/Viz の正しさ・負荷・性能、D: 実 Plugin Native Dock の format×OS×L0/L1 matrix、E: 両 Consumer 独立 Oracle）について、対象 SHA・Run・Oracle 付きの証拠で Pass/Fail/Unsupported/NotRun/Blocked を能力別に判定し、#419 の後継 ADR 最終採用へ引き渡す評価 Issue である。

現時点で判明している構造的事実:

- **Gate A（L1 契約基盤）は 3 OS の既存 CI で実走・Pass の証拠が揃っている**（`evidence/rf388-os-l1-ci/report.md`、対象 SHA `e93c438a` の Full CI run 43428172820）。
- **Gate D（実 Native Dock）は Windows で未達が確定的**: GPUI 0.2.2 の DirectComposition `topmost=true` が child HWND を覆い、plugin editor の画素が合成されないことが `evidence/rf388-native-dock-windowing/report.md` で実測特定され、Issue #532 として未解決のまま open。HWND 機構（parentage/geometry/clip/focus/key/redock）は実測 Pass だが「Native Dock PASS（完了条件全体）」は不成立と各 report が明記している。
- **Gate E（両 Consumer Oracle）は未実施**: `consumer-verify.yml` は workflow_dispatch 専用で対象 SHA の run がない。
- **観測済みの CI Fail が 1 件ある**: run 43540652567（SHA `2613b238`）の `verify (ubuntu-latest)` で `e2e_disable_env_kills_active_transport` が FAILED（Issue #625 tracker、修正 PR #626 = 現 HEAD `7aab6302`）。HEAD には修正が入っているが同 SHA の CI run は未実施。

本スライスの責務は、この証拠を集約して **能力別 verdict matrix と推奨 verdict（根拠表）** を記録し、#419/#465 への引き渡し文面を作ることである。**判定（verdict）の決裁宣言・Issue コメント投稿・Close 判定・ADR status 変更は Coordinator の権限であり、本スライスは行わない。**

## Audience

- Coordinator / Issue Owner — #419 の ADR status 決裁（accepted / conditional / rejected）の判断材料
- #465 Legacy WebView 撤去の担当 — L1 機能・既存 Consumer 互換と未実 GUI 製品機能の破壊を独立判断するための境界記録
- Synvelis #176 / AudioLens #166 の製品 GUI Owner — Windows で実際に提供・利用する能力の L2/L3 未達範囲の確認材料
- 以降の #262/#388/#389/#532 slice の担当 — 何が Pass/Fail/NotRun/Unsupported で、何が残件かの引き継ぎ

## Scope

### In Scope

- Issue #262 の現行受入条件（2026-11-13 16:22 JST Windows L2/L3 条件付き必須を正本とする 7 項目）に対応する証拠照合
- A〜E Gate の verdict matrix（CaseID/OS/Driver/Plugin/Expected/Observed/verdict/証拠 URL）の作成
- 既存 CI run・`evidence/` 配下の先行 PoC 証拠・ローカル lane 再実走の収集と対象 SHA との照合
- macOS/Linux の Build/Link 不可・未実装 OS Driver 欠落の L1 Fail/Capability 欠落としての追跡（GUI 実機 NotRun と区別）
- #419 向け引き渡し文面（「L1 のみ評価済み、L2/L3 非保証」の明記、#465 独立判断の境界）
- 観測した CI Fail（run 43540652567）の記録と後続状態の参照

### Out of Scope

- コード・CI workflow・`Cargo.toml`/`Cargo.lock`/`mise.toml` の変更（文書・証拠 slice）
- 新規 CI Workflow の作成（POLICIES.md §4/§5、Issue 受入条件「新CI Workflowを作らない」）
- #419 ADR 本文・status の変更（`docs/adr/` は本 slice の編集範囲外。ADR status 変更は ruviform#419 の領分）
- Issue #262/#419 の Close 判定、Issue コメント投稿（Coordinator が実施）
- 実 Native Dock の PASS 付与・GPUI/OS/Plugin 実装修正（#532/#388/#389 の残件）
- Windows L2/L3・macOS/Linux L2/L3 の新規実機検証の実施（既存証拠の照合と、正規 lane の再実走による同 SHA 補強のみ）
- #465 の Legacy 撤去実施・判定（独立 Issue の責務）

## Requirements

- 判定語彙は `POLICIES.md` §3 に従う: 要求状態 `Required`/`Deferred`/`NotApplicable` と実行結果 `Pass`/`Fail`/`NotRun`/`Unsupported`（根拠を伴う技術的非対応）を別軸で管理する。
- 各 Case は対象 Repo/SHA（Consumer の場合 resolved SHA）、OS/Arch、Feature/テスト名、期待結果・観測結果、実行環境、CI Run URL/実機ログを記録する。Job green や lane 定義の存在だけを個別 Case の Pass としない。
- 未実験を `Unsupported` と混同しない。`Unsupported` は実装/環境 Capability が明示不対応の場合のみ。`NotRun` は実機不在・CI 未実行・Skip/ignored・Feature 無効を含む。
- Windows でも未利用機器・Plugin 形式・特殊 FS の総当りは必須にしない。利用する能力だけを Case/Device/Format/環境ごとに指定する。
- Floating Window 重ね・Generic Parameter Editor・Fake/Mock Surface の成功を実 Native Dock Pass として扱わない。
- macOS/Linux の Build/Link 不可・未実装 OS Driver の欠落は実際の L1 Fail/Capability 欠落として追跡し、GUI 実機 NotRun と区別する。
- #419 の採用決定には「L1 のみ評価済み、L2/L3 非保証」を明示する。#465 の Legacy 撤去では L1 機能/既存 Consumer 互換・未実 GUI 製品機能の破壊を独立判断する。
- 対象機能/OS/Run URL/Oracle/Fail/Blocked を別記録し、新 CI Workflow を作らない。

## Contract outcome

本スライスが生む成果物は「証拠に基づく判定記録と推奨 verdict」である:

1. **verdict matrix**（`evidence/rf262-gpui-verdict/verdict-matrix.md`）: A〜E Gate × OS × driver/plugin/feature の Case 別に expected/observed/verdict/証拠 URL を記録した判定表
2. **評価 report**（`evidence/rf262-gpui-verdict/report.md`）: 証拠収集方法・対象 SHA/Run 照合・判定まとめ・既知ギャップの記録
3. **#419 引き渡し文面**（`evidence/rf262-gpui-verdict/handoff-419.md`）: 「L1 のみ評価済み、L2/L3 非保証」と #465 独立判断境界を明記した投稿用文面（投稿は Coordinator）
4. **検証記録**（`evidence/rf262-gpui-verdict/verification.md`）: 本 slice で実行した検証コマンド・OS・toolchain・exit code・結果
5. **文書チェーン**: spec/design/test-spec/plan/task/IR/exp（本ファイルを正本 spec とする）

**推奨 verdict（Coordinator 分析に基づく記録。決裁は Coordinator）**: GPUI Native First を標準 Driver とする設計方針は維持。OS-L1（3 OS Native Build/Link・Headless UI Scenario/Theme/Driver boundary・必要 Feature・RT 非干渉）は条件付き採用可。Native Plugin Dock の無条件本番採用は不可（#532 DComp 未解消・Gate D 未達）。#419 ADR には「L1 のみ評価済み、L2/L3 非保証」を明記する。

## Acceptance criteria

Issue #262「受入条件」欄の転記に対応する:

- [x] GPUI Native Firstを標準Driverとし、公開UI契約をDriver-neutralとする既定設計を確定。（Issue 側で確定済み。本 slice はその前提を証拠で検証する）
- [ ] Windows/macOS/LinuxでCI実走したNative Build/Link、Headless UI Scenario/Theme/Driver boundary、必要なFeatureと両Consumer L1非回帰の対象SHA・Run結果を照合する。→ verdict matrix Gate A/E と report の SHA/Run 対応表で照合記録する。
- [ ] macOS/LinuxのBuild/Link不可・未実装OS Driverの欠落は実際のL1 Fail/Capability欠落として追跡し、GUI実機NotRunとは区別する。→ verdict matrix で `native_surface_slot` 等の OS 別 Capability を `Unsupported`（宣言済み技術的非対応）/`NotRun`（未実施）に分離して記録する。
- [ ] #419の採用決定に「L1のみ評価済み、L2/L3非保証」を明示し、#465のLegacy撤去ではL1機能/既存Consumer互換・未実GUI製品機能の破壊を独立判断する。→ `handoff-419.md` に明記する。
- [ ] **Windows L2/L3（利用機能のみ必須）:** WindowsのGPUI採用判定には実GUI Window/Input/Focus等の使用能力L2と、提供予定のNative Plugin Dock L3（使用するPlugin形式のみ）の実証を含める。#532 DComp未解消なら当該能力は採用済みとせず条件付き/未受入を明記する。→ verdict matrix Gate B/D の Windows 行で能力別に記録し、DComp 未解消の Dock 画素能力は「条件付き/未受入」とする。
- [ ] **macOS/Linux L1のみ必須:** 既存CIのNative Build/Link・Headless/RT/必要Featureを対象SHAで実走。L2/L3は将来利用時のDeferredでありNotRunをPass扱いしない。→ verdict matrix の macOS/Linux 行で L1 証拠と Deferred を分離する。
- [ ] **Scope/Evidence:** Windowsでも未利用機器・Plugin形式は必須外。対象機能/OS/Run URL/Oracle/Fail/Blockedを別記録し、新CI Workflowを作らない。→ verdict matrix の Case 定義と report の検証スコープ節で担保する。

## Evidence

- `evidence/rf262-gpui-verdict/report.md` — 証拠収集・SHA/Run 照合・判定まとめ
- `evidence/rf262-gpui-verdict/verdict-matrix.md` — A〜E Gate verdict matrix（正本）
- `evidence/rf262-gpui-verdict/handoff-419.md` — #419/#465 向け引き渡し文面
- `evidence/rf262-gpui-verdict/verification.md` — 本 slice の検証実行記録
- 参照証拠: `evidence/rf388-os-l1-ci/report.md`・`handoff-262-419.md`、`evidence/rf388-389-native-dock/report.md`、`evidence/rf388-native-dock-layout/report.md`、`evidence/rf388-native-dock-windowing/report.md`、`evidence/rf389-vst3-editor/report.md`、`evidence/rf389-session-hardening/report.md`、`evidence/rf313-os-capability-matrix/report.md`、`evidence/rf243/window-job-headless-verification.md`、`evidence/issue-243-multiwindow-verify/`、`evidence/rf465/README.md`、`docs/impl/exp/issue-532-dcomp-audit.md`

## Deferred Design Concerns

- 実 Native Dock の製品受入（Windows L3） — #532 DComp 解消 + #388/#389 残件 + SV #176 Owner。再評価 Trigger: #532 の対策決裁と実装。
- Windows L2 の残能力（IME・DPI 動的変化・a11y・物理入力・plugin 主導 resize・DPI/マルチモニタ・2 window・crash/hang） — 現行製品が利用する能力が確定した時点で Required 化。再評価 Trigger: Consumer の GUI 機能確定 / #313・#243 残スコープの Owner 割当。
- macOS/Linux の native_surface_slot backend・実 GUI L2/L3 — 利用要件発生まで Deferred。再評価 Trigger: POLICIES.md §2 の昇格条件。
- Gate C の性能次元（FPS/frame-time/CPU/GPU/memory/latency の baseline 比較） — 測定環境と予算の確定が前提。再評価 Trigger: 性能予算の Issue 化。
- Gate E の Consumer Oracle — `consumer-verify.yml` の対象 SHA run または各 Consumer の受入 Issue（AL #166 / SV #176 / rf #294）。

## Sources and Relations

- Issue #262（本 spec の source）・#419（引き渡し先）・#388/#389（Native Dock PoC）・#532（DComp 未解決）・#465（Legacy 撤去の独立判断）・#294（両 Consumer 互換）・親 #257・#384（後継 ADR）
- `POLICIES.md` §2/§3/§5（OS 別検証レベル・要求状態と実行結果の分離・ローカル検証と CI の責務分離）
- `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`（proposed、status 変更は #419 の領分）
- `evidence/rf388-os-l1-ci/`（3 OS L1 CI 証拠と #262/#419 引き渡し先例）
