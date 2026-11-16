---
id: "SPEC-034YnxIOAi2baOZkdyPKlx"
type: "spec"
status: "approved"
title: "Issue #532 DComp 遮蔽層診断 slice（query-only exstyle/z-order/WindowFromPoint による根因二分）"
created: "2026-11-16"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-532-dcomp-occlusion.md"
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
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "docs/designs/issue-532-dcomp-pixel.md"
    - "docs/impl/ir/issue-532-dcomp-pixel.md"
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/specs/issue-631-real-window-dock.md"
    - "evidence/rf532/gpui-public-surface-audit.md"
    - "evidence/rf388-native-dock-windowing/report.md"
    - "evidence/rf631/verification.md"
  refines:
    - "docs/specs/issue-532-dcomp-pixel.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "docs/designs/issue-532-dcomp-pixel.md"
  derived-by:
    - "docs/designs/issue-532-dcomp-occlusion.md"
    - "docs/plans/issue-532-dcomp-occlusion.md"
    - "docs/test-specs/issue-532-dcomp-occlusion.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-dcomp-occlusion.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 DComp 遮蔽層診断 slice（query-only exstyle/z-order/WindowFromPoint による根因二分）

## What Should Be Built

Issue #532 の**根因二分 slice**。前回 slice（`issue-532-dcomp-pixel`、
PR `#677` merge `b6b72dbd`）で固定した `real-dcomp-pixel` regression
を入力に、docked `test_editor` 実 child HWND の画素が画面合成に
出ない**遮蔽層**を query-only の Win32 診断で切り分ける:

- (a) GPUI root HWND の extended style 検査
  （`GetAncestor(GA_ROOT)` → `GetWindowLongPtrW(GWL_EXSTYLE)` →
  `WS_EX_NOREDIRECTIONBITMAP` の有無）— env var
  `GPUI_DISABLE_DIRECT_COMPOSITION` が GPUI 初期化に実効したかの
  run ごとの証拠（audit 確定の window.rs:409-411 分岐により、
  同 flag は `disable_direct_composition` 時にのみ外れる）。
- (b) root の子 HWND 列挙 + sibling z-order
  （`GetWindow(GW_CHILD/GW_HWNDNEXT)` で direct children を
  z-order 順に走査）— GPUI 自身の子 window や container より上の
  sibling による WM 級遮蔽の排除/特定。
- (c) `WindowFromPoint` で docked child rect 内の代表点における
  実 topmost HWND の取得と root/container/child/他 window への
  識別 — **WM 級遮蔽 vs 合成器レベル遮蔽の決定的二分**。

診断は `real-dcomp-pixel` case の note 級 step row として記録し、
verdict 意味論・oracle 判定は一切変更しない（診断は証拠であり
gate ではない）。baseline（env 未設定）と env-on
（`GPUI_DISABLE_DIRECT_COMPOSITION=1`）の両条件を interactive
session で実走し、遮蔽層の二分結論と方式 A patch・B-region・
B-dcomp2・C への viability 帰結を doc chain に記録する。

## Why Now

前回 slice の実機観測で「env-on では child backing surface は
`windowdc` green fraction = 1.0 に満ちるのに screen 合成は
0.0 のまま」が確定し、方式 A'（`GPUI_DISABLE_DIRECT_COMPOSITION=1`）
単独では画素が回復しないことが実証された。遮蔽が DComp topmost
visual だけとは限らない — HWND flip-model swapchain の提示経路
（iFlip/DirectFlip/MPO plane）由来の可能性があり、その場合
方式 A の `topmost=false` patch では救えない。方式採否・本 fix
の前に遮蔽層を特定するのが本 slice であり、残存課題「env var が
GPUI 初期化に実際に効いたかの内部確認手段なし」も exstyle 診断で
解消する。

## Users and Value

- Issue #532/#388/#262 の Owner: 遮蔽層（DComp visual /
  flip-model 提示 plane / WM 級 z-order）の二分結果が方式
  A/A'/B/C 採否の決定的入力になる。
- Ruviform の後続実装者: `real-dcomp-pixel` の note 診断が
  今後の修正検証でも同じ二分を再現可能にする。
- Synvelis #176 の Owner: 実 Native Dock の OS 受入に必要な
  Ruviform 側前提（DComp/合成経路の未解消事実とその機序）を
  最新 SHA で確認できる。

## Scope

### In Scope

- `crates/ruviform-plugin-dock-test` の `real-dcomp-pixel` case
  への query-only 遮蔽層診断 3 系の追加（note 級 step row、
  verdict 不変）。
- 新規 sibling module（`src/osoccl.rs` 相当 — `oschild`/`ospix`
  と同じ query-only・plain 値境界・u64 HWND 規約）または
  `ospix.rs` 拡張 — 配置は design で確定。
- baseline（env 未設定・canonical `mise run
  test-ui-plugin-dock-real`）と env-on
  （`GPUI_DISABLE_DIRECT_COMPOSITION=1` 前置・直接 `cargo run`
  + 別 `--out` の補助実走 — canonical JSONL を上書きしない）
  の両条件の実機実走と記録。
- exit-127 観測時の stderr/exit code 捕捉と仮説 H1-H5 分類
  （前回未観測 — 両 run exit 1）。
- 遮蔽層の二分結論・方式 viability 帰結・観測手法の limitation
  （`WindowFromPoint` が GPU 合成経路下で返す答えの決定性は
  未検証等）を `evidence/rf631/verification.md` 追記節・
  JSONL step detail・doc chain に記録。
- 文書チェーン（spec/design/plan/task/test-spec/impl-ir/impl-exp
  + companion `issue-532-dcomp-occlusion-exp.jsonl`）と
  `docs/discovery/issue-532-dcomp-occlusion.md` の作成。
- `real_dock.rs` 冒頭 case 表・crate README・bin doc の診断
  追加と整合しない記述の更新（lease 内のみ）。
- 触った `docs/<type>/` dir の index 再生成は Coordinator 統合時に
  行う（本 slice では生成しない — PR 本文に明記）。

### Out of Scope

- 方式 A patch・B-region・B-dcomp2・C の採否判定・本 fix
  （`ruviform-ui-gpui` private adapter の合成経路変更・
  `[patch]`/fork・`SetWindowRgn`/`SetWindowPos` 等の変異系
  probe・env var 恒常化の製品経路化 — 全て後続 slice）。
- Focus/Resize/ReDock/入力の実 Windows 実走（Issue 受入の
  修正後 Pass 部分）。
- `mise.toml` の変更（lane 追加・task 化・stale 化する lane
  コメント「the editor-pixel question stays deferred」
  L166-169 付近の修正を含む — lease 外。PR 本文「未対応の
  作業」に明記して Coordinator に渡す）。
- `docs/**/README.md` index の再生成・手編集（Coordinator が
  統合時に再生成）。
- `Cargo.lock`・ルート `Cargo.toml`（共有ファイル — 差分が
  出た場合は再生成のみ・統合時処理として PR 本文に明記）。
- `evidence/rf532/`・`evidence/rf388-*/`（lease 外・参照のみ）。
  本 slice の証拠は `evidence/rf631/` と docs チェーンに置く。
- `crates/ruviform-ui/`・`crates/ruviform-ui-test/`・
  `crates/ruviform-ui-gpui/`・`crates/ruviform-plugin-test-vst3/`
  他全 crate（製品 crate zero-diff — 参照のみ）。
- merge-SHA CI 証拠・macOS/Linux L1 対応付け・consumer 通知・
  Issue auto-close・`*-msrv` lane・3OS 対応。

## Requirements

- R1: `real-dcomp-pixel` case の note 級診断に 3 系を追加する —
  (a) root HWND exstyle（`GA_ROOT` + `GWL_EXSTYLE` +
  `WS_EX_NOREDIRECTIONBITMAP` 有無を run ごとに記録）、(b) root
  の子 HWND 列挙 + sibling z-order（HWND・class 名・rect 等の
  plain 値で記録）、(c) `WindowFromPoint` による docked child
  rect 内代表点の実 topmost HWND 取得と root/container/child/
  他 window の識別。診断値は整数・文字列ラベル等の plain 値で
  JSONL step row に記録し、GPUI/Win32 型を schema・公開 API に
  出さない（HWND は u64/整数）。
- R2: 診断は query-only（window の作成/破棄/移動/属性変更/
  メッセージ送信をしない — `oschild`/`ospix` と同じ境界）。
  `SetWindowRgn`・`SetWindowPos`・`SetWindowLongPtrW` 等の
  変異系 probe は対象外。
- R3: verdict 意味論は不変 — 診断は note であり oracle 判定に
  使わない。既知 DComp `Fail` をそのまま維持し、lane exit 1 は
  期待される正直な信号として記録・緩和しない。他 case
  （`real-window-mount`/`real-native-slot`/`real-editor-attach`/
  `real-capture`）の verdict を動かさない。
- R4: baseline・env-on の両条件を interactive session で実走し、
  両条件の診断結果を記録する。exit-127 観測時は仮説 H1-H5 に
  分類して正直に記録（前回未観測・両 run exit 1）。headless
  host では全 case `NotRun`+`environment_blocked`・exit 0 の
  fail-closed をそのまま記録し、実走したと偽らない。
- R5: 遮蔽層（DComp visual / flip-model 提示 plane（iFlip・
  DirectFlip・MPO）/ WM 級 z-order）の二分結論を観測から導き、
  方式 A patch・B-region・B-dcomp2・C の viability への帰結を
  doc chain（design/discovery/impl-ir）に記録する。判明した
  事実と推測を区別し、未確定なら未確定のまま残す。方式採否の
  決裁は本 slice ではしない。
- R6: `GPUI_DISABLE_DIRECT_COMPOSITION` env var の有無（run
  ごと・JSONL 生値）・interactive session の有無・repo SHA・
  OS/GPU/DPI・観測・診断手法とその limitation を
  `evidence/rf631/verification.md` 追記節と JSONL
  `environment`/step detail に記録する。診断
  screenshot/artifact が得られたら `evidence/rf631/` 配下に
  保存する。
- R7: headless lane（`test-ui-plugin-dock`）の verdict 表・
  `test-ui-plugin-dock-console` の exit 0/1/2 意味論・
  `test-ui-scenario-contract`・`check-ui-driver-boundary` を
  非回帰に保つ。default feature および `--features real-dock`
  の check/lint/test green 維持（catalog↔entries 一致 test
  含む）。`REAL_CATALOG` の metadata を変える場合は
  `real_catalog.rs` も更新する（診断追加で required capability
  は変えない見込み）。
- R8: 新規 dependency・feature flag は追加しない（対象 API は
  既存 `windows-sys` optional edge の `Win32_UI_WindowsAndMessaging`
  /`Win32_Foundation` feature 内で揃う）。必要になった場合は
  追加せず確認する。`unsafe` は Win32 FFI 境界の
  `#[expect(unsafe_code)]` + SAFETY コメントの局所許可のみ。
  公開 enum 追加時は `#[non_exhaustive]`。

## Acceptance Criteria

- [ ] `real-dcomp-pixel` case に query-only 遮蔽層診断 3 系
  （root exstyle・子 z-order・`WindowFromPoint`）が note 級
  step row として追加され、verdict 意味論・oracle 判定が
  不変。
- [ ] baseline（env 未設定）と env-on
  （`GPUI_DISABLE_DIRECT_COMPOSITION=1`）の両条件が interactive
  session で実走され、両条件の診断結果が JSONL step row に
  記録される。canonical `plugin-dock-real-scenario.jsonl` を
  env-on run で上書きしない。
- [ ] exit-127 観測の有無と H1-H5 分類が記録される（未観測なら
  未観測と記録）。
- [ ] 観測から遮蔽層を二分し、方式 A patch・B-region・B-dcomp2・
  C への viability 帰結が doc chain に記録される。事実と推測が
  区別され、未確定は未確定のまま。
- [ ] `real-dcomp-pixel` verdict は正直 `Fail` のまま
  （lane exit 1 = 期待される信号）。他 4 case の verdict が
  非回帰。
- [ ] env var 有無・interactive session・repo SHA・OS/GPU/DPI・
  手法と limitation が `verification.md` 追記節と JSONL に
  明記される。
- [ ] `mise run fmt-check`・crate の check/lint/test
  （default + `real-dock` feature）・`test-ui-plugin-dock`・
  `test-ui-plugin-dock-console`・`test-ui-scenario-contract`・
  `check-ui-driver-boundary`・`prek run --all-files` が green。
- [ ] 文書チェーンと `docs/discovery/issue-532-dcomp-occlusion.md`
  が作成され、front matter relations が相互解決可能。
- [ ] 本 slice だけでは Issue #532 を close しない
  （`Refs #532` のみ）。

## Deferred Design Concerns

- 方式 A patch・B-region・B-dcomp2・C の採否判定と本 fix |
  本 slice の遮蔽層特定結果が前提 | 後続 slice | 観測なしに
  採否すると flip-model 提示 plane 系の機序を見落としたまま
  実装へ入るリスク。
- `SetWindowRgn`/`SetWindowPos`/`SetWindowLongPtrW` 等の変異系
  probe（B-region 実験） | 方式実験は adapter 側の後続 slice |
  方式採否後 | query-only 境界を越えると診断と実験の区別が
  曖昧になる。
- Focus/Resize/ReDock/入力の実 Windows 実走 | 方式採用・本 fix
  後 | Issue 受入の修正後 Pass 部分 | 未検証のまま Pass を
  主張するリスク。
- `mise.toml` L166-169 lane コメント stale 化・docs index |
  lease 外 | Coordinator 統合時 | コメントと実装の乖離が残る
  （PR 本文に明記済み）。

## Sources and Relations

- Issue `scarecrow173/ruviform#532`（本 slice 対象 — OPEN
  継続、auto-close しない）。関連: `scarecrow173/ruviform#388`・
  `scarecrow173/ruviform#630`・`scarecrow173/ruviform#631`・
  `scarecrow173/synvelis#176`（いずれも inline code 参照）。
- `docs/specs/issue-532-dcomp-pixel.md`・
  `docs/designs/issue-532-dcomp-pixel.md`・
  `docs/impl/ir/issue-532-dcomp-pixel.md`（前回 slice —
  regression case・env-on 実機観測・残存課題の出所）。
- `docs/discovery/issue-532-gpui-dcomp-surface.md` F5（exit-127
  仮説 H1-H5）・`evidence/rf532/gpui-public-surface-audit.md`
  （env var 意味論・`WS_EX_NOREDIRECTIONBITMAP` 分岐・非 DComp
  swapchain `CreateSwapChainForHwnd`+`FLIP_SEQUENTIAL`+
  `SCALING_NONE`+`ALPHA_MODE_IGNORE`・exit-127 PENDING）。
- `docs/designs/issue-532-dcomp-native-dock.md`（A/B/C 採否条件・
  Step1-4・NativeSurfaceSlot 契約維持条件）・
  `docs/designs/issue-532-gpui-dcomp-surface.md`（A' 比較表）。
- `evidence/rf388-native-dock-windowing/report.md:49-81`
  （DComp topmost 覆い隠し実測・window_pixel/printwindow_pixel/
  sibling probe の画素診断手法・exstyle 実測 `0x00240100`）。
- `evidence/rf631/verification.md`（前回 slice の baseline/env-on
  実機記録 — screen green = 0.0・env-on `windowdc` = 1.0・
  exit-127 未観測）。
- `POLICIES.md`（検証レベル・Pass/Fail/NotRun/Unsupported
  規範・fail-closed）。
