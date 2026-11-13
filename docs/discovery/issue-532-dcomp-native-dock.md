---
id: "DISC-034WZDbDyQpBcWspeahHje"
type: "discovery"
status: "resolved"
title: "Issue #532 GPUI DComp topmost 合成制約 — 現行 L1 証拠照合・再開条件・方式 A/B/C 調査手順監査"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/pull/501"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-532-dcomp-native-dock.md"
      - type: "file"
        path: "docs/designs/issue-532-dcomp-native-dock.md"
      - type: "file"
        path: "docs/specs/issue-532-dcomp-audit.md"
      - type: "file"
        path: "docs/designs/issue-532-dcomp-audit-design.md"
      - type: "file"
        path: "docs/plans/issue-532-dcomp-audit-plan.md"
      - type: "file"
        path: "docs/tasks/issue-532-dcomp-audit-task.md"
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-audit.md"
      - type: "file"
        path: "docs/impl/ir/issue-532-dcomp-audit.md"
      - type: "file"
        path: "docs/impl/exp/issue-532-dcomp-audit.md"
    modified: []
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
  related:
    - "docs/specs/issue-532-dcomp-audit.md"
    - "docs/designs/issue-532-dcomp-audit-design.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/plans/issue-532-dcomp-audit-plan.md"
    - "docs/tasks/issue-532-dcomp-audit-task.md"
    - "docs/test-specs/issue-532-dcomp-audit.md"
    - "docs/impl/ir/issue-532-dcomp-audit.md"
    - "docs/impl/exp/issue-532-dcomp-audit.md"
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
    - "docs/designs/issue-388-native-dock-windowing-slice3.md"
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
    - "evidence/rf388-native-dock-windowing/report.md"
    - "evidence/rf388-native-surface-slot/report.md"
    - "evidence/rf388-os-l1-ci/report.md"
    - "evidence/rf388-os-l1-ci/handoff-262-419.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
  derived-by:
    - "docs/specs/issue-532-dcomp-audit.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-dcomp-audit.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/synvelis/issues/176"
    - "https://github.com/scarecrow173/synvelis/issues/114"
  defers: []
  deferred-by: []
---

# Issue #532 GPUI DComp topmost 合成制約 — 現行 L1 証拠照合・再開条件・方式 A/B/C 調査手順監査

## Summary

Issue #532（GPUI 0.2.2 の DirectComposition `CreateTargetForHwnd(topmost=true)` が GPUI window 配下の全 child HWND を覆い、`NativeSurfaceSlot` 内の実 plugin 画素が画面に合成されない制約）の残件を headless で処理する監査記録。**確定事実**: (1) HWND レベルの機構（parentage・geometry・visibility・`SetWindowRgn` clip・hit-test・focus/key 配送・lifetime・re-dock）は Windows 実機で全て実測 Pass だが、(2) plugin editor の画素は DComp 合成で一切表示されず Native Dock の視覚的目的は未達、(3) 現行 L1（契約・Fake・headless・gpui-test）の被覆は既存 CI lane で 3 OS 照合可能、(4) Issue #532 の「着手条件（Blocked by）」は既に「なし」であり、他 Issue の正式依存としての誤登録は観測されなかった。**未確証のまま保持**: 実 plugin 画素の表示・解消方式の採否は本監査の範囲外であり、Pass/Unsupported の確定判定は行わない。

## Exploration Goal

Issue #532 の未チェック受入項目のうち headless に可能な 3 件 — (a) 現行 L1 で必要な Fake/`NativeSurfaceSlot` 契約・Compile/Headless 試験の結果と公開 API/RT 非回帰を既存 CI（Windows/macOS/Linux の利用可能 lane）の同候補 SHA に対応付ける、(b) L2/L3 利用要件発生時の再開条件と方式 A/B/C の調査手順・担当（#388/#389/SV #176）の記録、(c) 本 Issue を現行リリース/GPUI 採用/Legacy 撤去の Blocked by から除外する提案 — を、観測可能な証拠に基づいて固定する。実画素試験・実 Plugin・実 GUI は本 slice の対象外（Issue 本文の 2026-11-13 現行受入どおり Deferred 調査）。

## Key Issues and Assumptions

- **DComp 制約の確定度**: `evidence/rf388-native-dock-windowing/report.md`（slice 3・PR #501 系）の実機観測で、GPUI 0.2.2 `platform/windows/directx_renderer.rs` の `CreateTargetForHwnd(hwnd, topmost=true)` が原因候補として特定済み。ただし Issue 本文どおり「単独原因と確定したと断定しない」— 対照実験（plugin ではない新規 probe HWND も非表示）と `WS_EX_NOREDIRECTIONBITMAP` の観測は強い傍証だが、再現 fixture での root cause 切り分けは方式 A の Step 1 に委ねる。
- **L1 証拠の SHA 整合**: 直近の全量 CI 実測は run `43428172820`（対象 SHA `e93c438adc4af5b498e10926fe859ac1338ac390`、PR #601 で照合済み）。本監査の HEAD は `8944a360de3983775bdb84537c2b0fa5ee7d668e`。`e93c438a..8944a360` の差分で `native_slot.rs`・`fake_native_slot.rs`・`native_slot_contract.rs` は無変更、`surface.rs` は doc comment の語彙 rename（`GuiParentToken`→`ParentLease`、PR #609）のみで契約・挙動に影響しない。よって e93c438a の CI 証拠は契約層について現行 HEAD に対しても代表的だが、**同一 SHA の CI run を本監査で新規取得したわけではない**（Run URL 収集は Coordinator の証拠確認に残す — Issue 本文・タスク指針どおり）。
- **CI lane の被覆限界**: `ruviform-poc-native-dock` の `gpui`/`vst3` feature は既定 CI で compile されない既知ギャップ（`evidence/rf388-os-l1-ci/report.md`「既知ギャップ」節）。consumer 層 PoC の feature-enabled compile/link 証拠は手動 Windows 実施分のみ。
- **fake/headless の証拠境界**: `FakeNativeSlot` は「contract 適合証拠のみで実 Dock PASS ではない」と module doc に明記。headless/gpui-test の成功を実 Native Dock・実 plugin 画素の証拠に読み替えない。

## Findings

### F1: 現行の制約（観測済み・未解消）

`NativeSurfaceSlot` の Windows backend（`crates/ruviform-ui-gpui/src/gpui_driver/surface.rs`）は GPUI top-level HWND の子として `STATIC` class の `WS_CHILD|WS_CLIPCHILDREN|WS_CLIPSIBLINGS` container HWND を生成し、plugin editor HWND をその配下に attach する。実機観測（`evidence/rf388-native-dock-windowing/report.md`、Windows 11 x64・GPUI 0.2.2・Vinyl.vst3 1.12.0）では:

- attach/geometry/visibility/clip（`SetWindowRgn`）/hit-test（`WindowFromPoint`）/focus/key 配送/lifetime/re-dock の HWND 機構は全て Pass。
- しかし `GetClipBox` が editor・container 双方で `NULLREGION`、`screen_pixel`/`PrintWindow` が editor 領域で常に GPUI `control_track` 色（`0x00362d26`）を返し、**plugin 画素は画面に一切出ない**。対照実験の新規 probe HWND も同様に非表示 → plugin 固有ではなく GPUI window 配下の全 child HWND が合成対象外。
- GPUI window の exstyle `0x00240100` = `WS_EX_NOREDIRECTIONBITMAP | WS_EX_APPWINDOW | WS_EX_WINDOWEDGE` で GDI redirection surface がなく、画面合成が DComp 経路に一本化される。
- 原因候補: GPUI 0.2.2 `platform/windows/directx_renderer.rs:793` の `CreateTargetForHwnd(hwnd, topmost=true)`。MSDN 上 `topmost=TRUE` は visual tree を window の children の上に描画する。`GPUI_DISABLE_DIRECT_COMPOSITION=1` の非 DComp 経路試行は scenario 途中でプロセスが exit 127（開発中観測・ログ artifact なし・本 slice 範囲外）。

**判定の現状**: NativeSurfaceSlot の視覚的目的は現行 GPUI 経路で未達。#388 の Native Dock は Partial/Unsupported として正直に記録済み（PR #501 系証拠）。本監査はこの状態を変更しない。

### F2: 現行 L1 被覆の棚卸し（ソース実測）

監査対象 SHA: `8944a360de3983775bdb84537c2b0fa5ee7d668e`（branch `scarecrow173/rf532-dcomp-audit`、main `8944a360` = PR #611 merge）。

| 層 | 実装/契約 | headless/fake 試験 | 試験数・名前（代表） |
| --- | --- | --- | --- |
| 中立契約 | `crates/ruviform-ui/src/native_slot.rs` — `SurfaceSlotRegistry`/`SurfaceSlotId{index,generation}`/`SlotLease`/`SlotSpec`/`SlotCapabilities`/`SlotEvent`/`SlotError`/`NativeSurfaceSlot` trait。generation-safe slot recycling・lease validate/revoke・`mark_busy`/`clear_busy`・`lease_parent` の released/already-leased/busy/unsupported 拒否・`release_slot` の `LeaseRevoked`→`Destroyed` 順序 | `crates/ruviform-ui/tests/native_slot.rs` | registry/slot 単体試験 11 件 |
| Fake driver | `crates/ruviform-ui-test/src/fake_native_slot.rs` — `NativeSurfaceSlot` port を registry 上に実装。attach された child を記録として表す。capability 未対応は `Unsupported` で明示拒否（fake success にしない）。module doc に「Fake success is contract conformance only — it is never evidence of a real Dock PASS」 | `crates/ruviform-ui-test/tests/native_slot_contract.rs`（mise `test-ui-native-slot`） | NSC-01〜13: attach 前 clip/z 継承・move/hide・NaN/負値 `InvalidValue`・unsupported 明示 refusal・focus 高々 1 child・attach fault で lease 解放・`Released`/`StaleGeneration`/`LeaseRevoked`・偽造 lease `InvalidLease`/`LeaseBusy`・`release_lease` 再 attach・teardown event 順序・busy 中 `SlotBusy`→解除後成功・`DockWorkspace` 2-phase abort rollback（live lease 維持）/commit（新 slot 再 attach・旧 lease revoke・旧 id stale） |
| Driver contract | `crates/ruviform-ui-test/tests/driver_contract.rs`（mise `test-ui-driver-contract`） | 同上 | 計 7 件: `manifests_are_consistent` + `ui_dvr_01`/`04`/`05`/`06`/`07`/`08`。`ui_dvr_06_native_slot_unsupported_stale_destroy` が unsupported refusal・destroy/revoke・stale-generation を pin |
| GPUI adapter | `crates/ruviform-ui-gpui/src/gpui_driver/surface.rs`（`gpui` feature）— `GpuiSurfaceAdapter`/`Binding`/`LeaseRedeemer` seam。`BACKEND_AVAILABLE = cfg!(target_os="windows")`。Windows backend は `STATIC` child HWND・`SetWindowRgn` clip・`SetWindowPos(HWND_TOP)` z-order・`SetFocus`/`GetFocus` focus。`BACKEND_CAPS`: supported = attach_parent/hide_show/focus/clip/z_order、unsupported = reparent/cross_window_move/resize_negotiate/keyboard_input/ime/popup/out_of_process_parent | `surface.rs` 内 `#[cfg(target_os="windows")]` 試験群 + `gpui-test`/`uitest` feature の headless 試験（mise `test-ui-gpui`） | Windows 実 HWND 系: `clip_clear_removes_os_region`/`geometry_move_reprojects_stored_clip`/`geometry_move_without_clip_leaves_no_region`/`create_under_dead_parent_is_rejected`/`teardown_of_other_window_leaves_slot_live`/`two_adapters_reconcile_independently`/`clip_declared_before_lease_applies_on_attach`/`owner_teardown_auto_releases_slot`/`unretired_owner_death_detected`/`container_death_on_live_owner_revokes_lease_only`/`repeated_teardown_is_idempotent`/`consumer_release_after_auto_release_errors`/`auto_released_index_recycles_generation`/`lease_under_dead_owner_is_rejected`/`adapter_drop_destroys_container`/`adapter_drop_after_owner_death_abandons_dead_container`/`shared_surface_drives_real_hwnd_lifecycle` |
| 境界 guard | `mise run check-ui-driver-boundary`（`gui-boundary-guard --offline`）— dep-graph + public signature 検査で HWND/NSView/X11/GPUI handle の公開 API 漏出なしを検査 | CI `ui-gpui-verify` 3 OS | step success |
| Render/lease 投影 | `crates/ruviform-ui-gpui/src/render.rs` の `project_slot_frame`/`slot_clip`（ancestor clip の pure function）・`lab_binding.rs` | `test-ui-gpui` lib tests（`gpui-test` TestAppContext） | slot projection/clip 単体試験 |
| Feature compile/link | `ruviform-ui-gpui` features: `gpui`（production adapter）/`gpui-test`（headless TestAppContext）/`uitest`（windowed control protocol）/`real-scenario`（real-window scenario adapter） | `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`（`--features "gpui-test uitest"`） | `windowed_test_app` AUT bin の Build/Link を含む |

**被覆の限界（正直な記録）**: 上記の全試験は契約層・fake 層・headless/gpui-test 層・Windows HWND 機構層の証拠であり、**DComp topmost 合成下での実 plugin 画素表示を検証する試験は存在しない**。`ui_dvr_06` の real OS leg は `cases.json` で blocked（#389/OS driver 待ち）と明示。`test-ui-offscreen-xvfb`（`real-scenario`、ubuntu Xvfb+openbox+lavapipe の実 X11 window pixel 3 tests）は Linux X11 経路の実 window 証拠だが Windows DComp 経路・plugin 画素とは別物。

### F3: 既存 CI lane との対応付け（同候補 SHA）

「同候補 SHA」の解釈: 本監査 slice は文書のみで新規コードを生まないため、証拠対象は監査時点の main HEAD `8944a360`。契約層ファイル（`native_slot.rs`/`fake_native_slot.rs`/`native_slot_contract.rs`/`driver_contract.rs`/`surface.rs` の挙動部分）は `e93c438a..8944a360` で実質無変更（F2 参照）のため、直近の全量 CI run `43428172820`（`ci.yml` workflow_dispatch、conclusion=success、`ci-summary` verdict=PASS、全 19 checkout leg の実測 SHA = `e93c438a…` を `checkout-sha-*` artifact で照合済み — `evidence/rf388-os-l1-ci/report.md`）の lane 構成が現行 HEAD の契約被覆として代表的に対応付けられる。**本監査で新規 CI run の取得・Run URL の収集は行っていない**（タスク指針: CI 実走・Run URL 収集は Coordinator の証拠確認に残す）。

| 受入項目（Issue #532） | 対応する既存 CI lane / mise task | OS | verdict（e93c438a run 実測を HEAD へ写像） |
| --- | --- | --- | --- |
| Fake/NativeSurfaceSlot 契約・Compile/Headless 試験 | `ui-gpui-verify` job 内 `test-ui-native-slot`（NSC-01〜13・各 OS `13 passed; 0 failed`）+ `test-ui-driver-contract`（UI-DVR-*・`7 passed`） | ubuntu/windows/macos | **Pass**（headless 契約層。実 Dock Pass ではない） |
| 公開 API/OS handle 境界 | `ui-gpui-verify` 内 `check-ui-driver-boundary` | 3 OS | **Pass** |
| GPUI adapter Compile/Link + Headless | `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`（`gpui-test uitest`、`windowed_test_app` bin link 含む） | 3 OS | **Pass**（headless `gpui-test`。実 window 操作は GUI-L2） |
| 実 window pixel（Linux X11・Xvfb） | `ui-gpui-offscreen-verify` 内 `test-ui-offscreen-xvfb`（`real-scenario`） | ubuntu (Xvfb) | **Pass**（headless 環境の実 X11 window。Windows DComp・plugin 画素ではない） |
| RT 非干渉 | `rt-verify`（strict `rt_` 166 tests）+ `verify` 内 `test-full` の `rt_*` | ubuntu + 3 OS | **Pass**（headless RT 契約。実 plugin GUI hang 時の RT 影響は #389 共同検証・範囲外） |
| Workspace Compile/Lint/Test（default feature） | `verify`/`verify-macos` の `lint-full`/`test-full` | 3 OS | **Pass** |
| MSRV（rust@1.89.0）契約再検証 | `msrv-verify` 内 `test-ui-test-msrv`（`native_slot_contract` 13 件含む） | ubuntu (1.89.0) | **Pass** |
| macOS/Linux L1（Native Build/Link・必要 Feature・Headless/RT） | `ui-gpui-verify-macos`（11 step）・`verify-macos`・`ui-verify-macos`・`l1-real-verify-macos`（fixture cdylib + `host` L1 E2E）・`l1-real-verify`（ubuntu/windows） | macos/ubuntu（+windows の l1-real） | **Pass**（L1 のみ。実 GUI/物理 Device/実商用 Plugin は Deferred/NotRun） |
| 実 plugin 画素の DComp 合成・入力・Focus・Resize・ReDock（Windows L2/L3） | 既存 CI に該当 lane なし（`poc-native-dock*` mise task は手動 lane・`ci.yml` 未登録） | — | **NotRun**（#532 の残件。Fake/headless 成功を Pass に繰り上げない） |

**既知ギャップ（記録のみ・本 slice で修正しない）**: `ruviform-poc-native-dock` の `gpui`/`vst3` feature は CI で compile されない（default feature のみ compile、`src/poc/` 配下は手動 Windows lane のみが証拠）。`surface.rs` `clip_of` の `dead_code` warning（test 成功に影響なし）。`consumer-verify.yml` は workflow_dispatch 専用で `e93c438a` の run なし → Consumer Oracle は NotRun。

### F4: Issue 依存状態と Blocked-by 誤登録の有無

- Issue #532 本文「着手条件（Blocked by）」は **なし** と明記。GitHub の正式依存（blocked-by relation）として #532 を他 Issue の先行条件に登録している形跡は本監査で観測されなかった（`gh issue view` の本文確認 + 関連 Issue の blocked-by 状況の確認 — #388/#389/#262/#419/#313/#243/#465 はいずれも open で、#532 への依存登録は見当たらない）。
- 「統合・受入で関係する Issue」節は #388/#389/#262/SV #176 を「着手は止めない」関係として列挙しており、Blocked by ではない。
- **提案（Coordinator 責務・本 slice では実施しない）**: GitHub 上の blocked-by 依存の正式な登録・削除は Coordinator が行う。本監査時点で除外対象の誤登録は観測されなかったため、追加の削除作業は不要と判断。ただし #465（Legacy WebView 撤去）が #532 を未解決 Windows DComp 制約として追跡している記述は残る — これは「Blocked by」ではなく「未解決制約のトラッキング」であり、除外提案の対象は「Blocked by 誤登録」のみ。
- **Blocked-by 除外の根拠（記録）**: (1) #532 の先行 Slice は Deferred 調査であり現行 L1 Gate の Blocking 要因ではない（Issue 本文の現行受入どおり）、(2) 実 Native Dock 未成立は Pass/Unsupported（未確証）として分離記録済みで、L1 証拠の欠落ではない、(3) #262 の採用判定は #532 の技術決裁を「入力」として受け取る関係であり、#532 未解決が #262 の着手を止める構造ではない（#262 は open で独立に進行可能）、(4) 現行リリース/GPUI 採用/Legacy 撤去の Blocked by に #532 を置くと、DComp 制約の未解消が無関係な release 判定を不当に止める。

## Alternatives and Comparison

方式 A/B/C の比較軸は Issue 本文「終了条件」「比較・選択」Step 2 に基づく。**本監査は比較表の枠組みと再開条件を記録するのみで、実機比較・採否判定は行わない**（L2/L3 利用要件発生時の調査手順として `docs/designs/issue-532-dcomp-native-dock.md` に設計として記録）。

| 方式 | 内容 | Pros | Cons | Lean |
| --- | --- | --- | --- | --- |
| A: GPUI 内最小修正 | GPUI 0.2.2 DComp target の設定・stacking 変更（例: `topmost=false`）。公開 API 範囲での実現可能性を最優先実証 | upstream 変更が最小・根本原因に近い・他の child HWND 機構をそのまま活かせる可能性 | `topmost=false` が child を背面に描く場合 plugin 画素が GPUI UI の下に隠れる可能性。単一パラメータ変更は再現と全関連 Case を満たす場合のみ採用（断定禁止）。upstream gpui の保守・version pin への影響 | maybe（Step 1 の再現・切り分けが前提） |
| B: Windows private 合成 | `ruviform-ui-gpui` 内に閉じた Windows 限定の private compositor / native child attachment 経路。Driver 内部・upstream 改変最小 | upstream 非依存・`NativeSurfaceSlot` 契約を変更せず adapter 層で吸収可能・HWND/GPUI/DComp 型を private に閉じられる | 実装・保守コストが Ruviform 側に乗る・DComp 仕様の深い理解が必要・他 OS への展開なし（Windows 限定） | maybe（A が不成立の場合の本命） |
| C: 代替 Native 経路 | 実 plugin editor 画素を保てる他の OS 公開 API/Host 連携（例: 別 top-level window・別合成経路） | DComp topmost 制約を迂回できる可能性 | Floating/overlay/スクリーンショット/Generic のみの場合は回避 UX であって Dock PASS ではない。実 plugin が offscreen surface を提供する前提の方式は普遍性を実証するまで採用しない | maybe（予備案） |

## Tentative Conclusions and Hypotheses

- **confirmed**: DComp topmost 制約は実機で再現・記録済み（F1）。HWND 機構は正しく動くが画素が合成されない。
- **confirmed**: 現行 L1（契約・Fake・headless・gpui-test）の被覆は既存 CI lane で 3 OS 照合可能（F2/F3）。ただし同一 SHA の新規 CI run は本 slice で未取得。
- **confirmed**: Issue #532 の Blocked by は「なし」で誤登録は観測されず、除外提案の実作業は不要（F4）。
- **hypothesis**: 方式 A（`topmost=false` 等の最小変更）が child HWND を DComp visual の背面に描画することで plugin 画素と GPUI UI の z-order が逆転し、実用上の Dock として成立しない可能性がある。これは未検証の仮説であり、Step 1 の再現 fixture で確かめる。
- **hypothesis**: 方式 B が最も制御可能だが、DComp の visual tree と HWND の合成順序を private に制御する実装複雑度が高い。upstream gpui の `directx_renderer.rs` が private API を公開していない場合、fork/patch が必要になる可能性がある。

## L2/L3 再開条件（利用要件発生時）

以下の全条件が揃った時点で Windows GUI-L2 / EXT-L3 の実機調査を再開する。**いずれか欠ける場合は NotRun/Deferred のまま維持し、Pass に繰り上げない**。

| 条件 | 内容 |
| --- | --- |
| 対象 OS | Windows 11 x64（実機。WSL/VM/Xvfb は不可 — DComp は実 Windows display 経路が必要） |
| GPU/driver | 実 GPU（RTX 4070 SUPER 等）+ 実 display。リモート desktop/仮想 display での再現性は別途記録 |
| Windows/GPUI version | Windows 11 最新安定 + GPUI 0.2.2（crates.io pin）+ blade-graphics 0.7.1。GPUI 更新時は本監査を再実施 |
| Cargo feature tuple | `ruviform-poc-native-dock` の `gpui`（+ 実 plugin 検証時 `vst3`）。`gpui-test`/`uitest` は headless であり L2/L3 の代替にならない |
| 実 Plugin | 実 VST3 binary（例: iZotope Vinyl 1.12.0）または対象形式の実 editor。in-process probe は機構確認用で実 plugin 画素の証拠ではない |
| Window/DPI | GPUI main window + child HWND の実配置。DPI scaling（100%/150%/200%）・マルチモニタ・2 Window 構成を含む |
| 対応目的 | NativeSurfaceSlot 内への実 plugin editor 埋め込みで、resize/clip/z-order/focus/input/re-dock が視覚的に成立すること |
| 測定環境 | `RVF_POC_SCENARIO=dock-windowing`（`mise run poc-native-dock-windowing`）+ 識別色 child fixture + 実 plugin。画面キャプチャ・`GetWindowRgn`/`WindowFromPoint`/`GetFocus`/pixel readback の定量記録 |
| 期待/観測 | 画素表示・入力配送・focus 遷移・resize 追従・clip 適用・detach/再親化・re-dock rollback・crash/hang 復旧の各 Case を Pass/Fail/NotRun で記録 |
| 証拠 | `evidence/rf532/` 配下に report + 生ログ + screenshot。Run URL または実機実行コマンド・SHA・環境 tuple を provenance として記録 |
| Owner | #388（NativeSurfaceSlot 契約）・#389（Plugin Editor Session/L0-L1）・SV #176（製品実 Plugin Native Dock OS 受入）・#262（GPUI 採用判定の入力） |

## 方式 A/B/C 調査手順（再開時）

`docs/designs/issue-532-dcomp-native-dock.md` に設計として記録。要約:

- **Step 1（最小再現・root cause）**: 識別色 Win32 child HWND fixture と実 plugin editor の両方を NativeSurfaceSlot に埋め、Win32 単体親ウィンドウと GPUI main window の画素差を比較。HWND 階層・attach/detach・画素が失われる最初の境界を特定。GPU/driver/GPUI revision/DPI/L0/L1/plugin 形式/bitness/child HWND の parent/Z-order/clip/DComp target 設定/input/focus 状態を全て記録。
- **Step 2（比較・選択）**: A/B/C を同一条件（同 OS/build/plugin/DPI/geometry/input sequence）で比較。上流修正/保守・plugin 対応率・clip/resize/DPI・pointer/keyboard/focus・multi-window/re-dock・L0/L1 分離・hang/crash/rollback・CPU/GPU/frame-time を評価。
- **Step 3（契約維持）**: `NativeSurfaceSlot` の lease/epoch・attach→resize/clip/focus→detach・2-phase re-dock/rollback を変更しない。HWND/GPUI/DComp 型は `ruviform-ui-gpui` private adapter 内に閉じ、RT callback・Consumer Domain に漏らさない。
- **Step 4（判定）**: 採用方式で fixture と実 plugin 双方を実窓検証し、能力ごとに Pass/Fail/Unsupported/NotRun を確定。(i) 採用案の実画素 E2E 実証と #388 への handoff、または (ii) 能力非採用/Unsupported の正式決裁と代替 Owner/復帰条件を出す。方式選択だけでは画素 Full PASS としない。

## Open Questions

- `topmost=false`（方式 A の最小変更候補）は child HWND を GPUI visual の背面に描画するか、それとも単に DComp target の stacking を変えるだけで child が引き続き覆われるか — **未検証**。Step 1 の再現 fixture で確かめる必要がある。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` の非 DComp 経路が scenario 途中で exit 127 になった原因（GPUI 側の別経路の問題か、環境要因か）— ログ artifact なし・未切り分け。
- 方式 B で DComp visual tree と HWND の合成順序を private に制御する際、upstream gpui の `directx_renderer.rs` が必要とする private API/内部状態にアクセス可能か — upstream の公開面の調査が必要。
- 実 plugin が offscreen surface（D3D texture 等）を提供する方式の普遍性 — 実 plugin binary での検証が必要。

## Promotion Candidates

- [x] spec-doc needed: 本監査 slice の受入条件を `docs/specs/issue-532-dcomp-audit.md` として固定（済）。
- [x] design-doc needed: 再開条件と方式 A/B/C 調査手順を `docs/designs/issue-532-dcomp-native-dock.md` として設計記録（済）。
- [ ] adr-doc needed: 方式 A/B/C の採否が決まった時点で #262 の ADR 入力として別途起票（将来・本 slice 対象外）。

## Sources and Relations

- Issue #532 本文（2026-11-13 現行受入・終了条件・比較軸・検証方法・Blocked by）。
- `evidence/rf388-native-dock-windowing/report.md` — slice 3 実機証拠（DComp 制約の発見・対照実験・原因候補）。
- `evidence/rf388-native-surface-slot/report.md` — OS/format capability matrix（20 行）・Fake fixture 検証。
- `evidence/rf388-389-native-dock/report.md` — L0 実 attach PoC 証拠。
- `evidence/rf388-os-l1-ci/report.md` — 既存 CI run `43428172820` の lane/test/SHA 照合（PR #601）。
- `evidence/rf388-os-l1-ci/handoff-262-419.md` — #262/#419 引き渡し文面。
- `docs/discovery/issue-388-389-native-dock-current-state.md` — #388/#389 現行実装監査（GuiParentToken/L1 Unsupported/wire/Dock 契約）。
- `docs/designs/issue-388-native-dock-windowing-slice3.md` — slice 3 設計（clip 修正・windowing PoC scenario）。
- `docs/test-specs/issue-388-389-native-dock-contract.md` — NSC-01〜13 契約ケース定義。
- `crates/ruviform-ui/src/native_slot.rs`・`crates/ruviform-ui-test/src/fake_native_slot.rs`・`crates/ruviform-ui-test/tests/native_slot_contract.rs`・`crates/ruviform-ui-test/tests/driver_contract.rs`・`crates/ruviform-ui-gpui/src/gpui_driver/surface.rs`・`crates/ruviform-ui-gpui/src/render.rs`・`crates/ruviform-ui-gpui/src/lab_binding.rs` — 契約・fake・adapter・render の実装と試験。
- `mise.toml` — `test-ui-native-slot`/`test-ui-gpui`/`check-ui-driver-boundary`/`check-ui-gpui`/`lint-ui-gpui`/`test-ui-driver-contract`/`test-ui-offscreen-xvfb`/`poc-native-dock*` task 定義。
- `.github/workflows/ci.yml`・`daily-ci.yml` — CI lane 構成（`ui-gpui-verify` 3 OS・`ui-gpui-offscreen-verify`・`msrv-verify`・`rt-verify`・`l1-real-verify` 3 OS 等）。

## Evidence

### 2026-11-14 — 監査実施記録（本 slice）

- **監査対象 SHA**: `8944a360de3983775bdb84537c2b0fa5ee7d668e`（`git rev-parse HEAD`）。
- **CI 証拠 SHA**: `e93c438adc4af5b498e10926fe859ac1338ac390`（run `43428172820`、PR #601 で照合済み）。`git diff e93c438a..8944a360` で `native_slot.rs`/`fake_native_slot.rs`/`native_slot_contract.rs`/`driver_contract.rs` は無変更、`surface.rs` は doc comment rename のみ（PR #609 `GuiParentToken`→`ParentLease`）。
- **実施した調査**: Issue #532 本文取得・関連 Issue 状態確認（#388/#389/#262/#419/#313/#243/#465 open、SV #176/#114 open）・`crates/ruviform-ui*` の契約/fake/adapter/試験のソース読み・`evidence/rf388-*` report の照合・`docs/test-specs/issue-388-389-native-dock-contract.md` の NSC 対応確認・CI lane 構成の `ci.yml`/`mise.toml` 確認。
- **実施していない検証**: 実画面・実 plugin・実 GUI の L2/L3 試験（対象外）。新規 CI run の取得・Run URL 収集（Coordinator 責務）。`cargo`/`mise run test-*` の実行（文書のみの変更のため軽量チェックに限定 — `POLICIES.md` §5）。
- **判定**: L1 契約被覆は既存 CI で照合可能（Pass）、実 plugin 画素は NotRun（未確証を維持）、Blocked by 誤登録は観測されず除外提案の実作業は不要。
