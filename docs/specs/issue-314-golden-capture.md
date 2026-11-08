---
id: "SPEC-034Ug8RuRab8De5zcQCjTB"
type: "spec"
status: "proposed"
title: "Issue #314 slice 3 — GPUI/OS capture backend と能力の正直な配線"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/464"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/426"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-314-golden-capture.md"
      - type: "file"
        path: "docs/designs/issue-314-golden-capture.md"
      - type: "file"
        path: "docs/plans/issue-314-golden-capture.md"
      - type: "file"
        path: "docs/tasks/issue-314-golden-capture.md"
      - type: "file"
        path: "docs/test-specs/issue-314-golden-capture.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/capture.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/golden_capture.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/scenario/visual.rs"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-314-golden-core.md"
    - "docs/designs/issue-314-golden-core.md"
    - "docs/specs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-scenario.md"
    - "docs/specs/issue-243-gpui-realwin-windows.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-314-golden-scenario.md"
    - "docs/designs/issue-314-golden-core.md"
    - "docs/specs/issue-464-consumer-neutral-gui-scenario-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-314-golden-scenario.md"
    - "docs/specs/issue-314-golden-core.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-314-golden-capture.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 3 — GPUI/OS capture backend と能力の正直な配線

## What Should Be Built

slice 2（`docs/specs/issue-314-golden-scenario.md`）で固定した
`CaptureOutcome`/`VisualGuiTest`/`golden_check` 契約に対し、**実在する
pixel 取得経路**を GPUI adapter に実装する。調査の結果、実在する経路は
1 本だけ確定した:

- `GpuiDriver::run` が開く**実 OS window** の client area を、Windows
  では `PrintWindow`（window 自身の描画内容）を第一経路、screen
  `BitBlt`（表示中の画面 rect）を fallback として GDI/DIB 経由で
  RGBA8 化する `GpuiDriver::capture_window` → `OsCapturedFrame`。

`gpui-test`（`GpuiGuiTest`/`GpuiObserver`）には pixel readback が存在
しないことが確認済みであり、scenario adapter の `capture_frame` は
`Unsupported` のまま維持する — 実 window driver は `SemanticGuiTest`
adapter ではないため、`VisualCapture` capability を scenario adapter
が主張することもない。実 window 側の golden e2e は driver 層の
`capture_window` → `RgbaImage` → `BaselineStore` で検証する。

## Why Now

- slice 1（Golden Utility）・slice 2（Scenario Contract 接続）は
  main 入り済みだが、`Captured` outcome を返す経路が存在しない。
  契約だけが先に固定された状態では、capture の実装形（target pin・
  verdict 語彙・evidence）が検証できない。
- #243（PR #482）で実 window 経路（`GpuiDriver::run` +
  `Slot::raw_parent` の OS handle 捕捉）は既に存在するため、capture
  backend はその handle を消費するだけで実装できる — 新規の windowing
  基盤は不要。
- 「実 OS pixel が取れる」ことを本 slice で実測・固定しないと、
  slice 4（#426 consumer fixture 接続）が golden verdict を
  `unsupported` のまま埋め込むことになる。

## Users and Value

- golden 検証の利用者: `GpuiDriver::capture_window` で実 window の
  RGBA8 pixel を取得し、slice 1 の `BaselineStore` で baseline 比較・
  candidate/promote・evidence emit まで一気通貫で回せる。
- 将来の real-OS scenario adapter（`ScenarioDriver::RealOs`、#313
  系）の前提: window pixel 取得の OS backend が driver 層に存在する
  ことが、scenario contract への `capture_frame` 実装の土台になる。
- Issue #314 全体: slice 4 で consumer story fixture を接続する際の
  「pixel が実際に取れる」証拠を本 slice が担う。

## Scope

### In Scope

- `crates/ruviform-ui-gpui/src/gpui_driver/capture.rs` 新設:
  `OsCapturedFrame` DTO、`GpuiDriver::capture_window`、Windows
  backend（`PrintWindow` primary / screen `BitBlt` fallback）、
  非 Windows の honest `Unsupported` backend。
- `lib.rs` への最小 wiring: `gpui_driver` 内の `pub mod capture;`
  宣言と `OsCapturedFrame` の root re-export のみ（#479 worker の
  focus/keystroke 配線領域には触れない — coordinator 承認済み）。
- `crates/ruviform-ui-gpui/tests/golden_capture.rs` 新設:
  unknown/unopened window の error path test と、実 window で
  capture → `RgbaImage` → compare → candidate → promote →
  `pass/match` → evidence JSONL の e2e test（Windows + `gpui`
  feature のみで実行）。
- `crates/ruviform-ui-test/src/scenario/visual.rs` の module doc
  更新（slice 3 で driver 層 backend が実装された事実の反映。
  契約・コードの動作変更なし）。
- 本 slice の文書チェーン（spec/design/plan/task/test-spec）。

### Out of Scope

- `gpui-test` / Fake adapter の `capture_frame` 実装 — pixel
  pipeline が存在しないため `Unsupported` を維持する（honest
  capability 契約）。`VisualCapture` capability の宣言も行わない。
- 実 window を対象にした `SemanticGuiTest`/`VisualGuiTest` adapter
  — `GpuiDriver::run` は呼び出し thread を platform event loop が
  占有する構造であり、scenario contract への adapter 化は
  `ScenarioDriver::RealOs`（#313 系）の範囲。
- macOS / Linux の capture backend — 本環境は Windows のみ実測可能。
  非 Windows は `Unsupported` を返す backend stub を置く。
- consumer story fixture 接続（AudioLens / Synvelis — slice 4、
  #426）。
- `Cargo.toml` 変更 — `windows-sys` の既存 feature で必要な API は
  揃っており、`PrintWindow` は `extern "system"` 宣言で回避する
  （`Win32_Storage_Xps` feature 追加は file boundary 外）。
- `crates/ruviform-ui/`（ViewHost 契約）・`Cargo.toml` メンバー定義・
  `.github/` の変更。

## Requirements

- R1: `GpuiDriver::capture_window(window) -> Result<OsCapturedFrame,
  UiError>` が、`run` で開かれた実 OS window の client area RGBA8
  pixel を返す。`OsCapturedFrame` は `width`/`height`/
  `scale_factor`/`pixels`/`backend` の driver-neutral な DTO で、
  OS handle 型を公開 signature に含めない。
- R2: capture は未知・未 open・破棄済み window に対して pixel を
  捏造せず `UiError::Host`/`UiError::Unsupported` を返す。
  `gpui-test` platform（OS handle なし）は `Unsupported`。
- R3: Windows backend は `PrintWindow(PW_CLIENTONLY |
  PW_RENDERFULLCONTENT)` を第一経路とし、API 失敗時のみ screen
  `BitBlt` に fallback する。どちらの経路で取得したかを
  `OsCapturedFrame::backend` に記録し、golden evidence の
  `backend_detail` に反映できるようにする。
- R4: semantic observation・draw plan・observer 出力を pixel 証拠へ
  昇格させない — capture は OS compositor/GDI 経路の実 pixel のみ。
- R5: capture 失敗（dead handle・minimized/hidden・空 client area・
  GDI 失敗）は `UiError` であり、verdict や `pass` に化けない。
- R6: `unsafe` は `#[expect(unsafe_code)]` の局所許可 + SAFETY
  コメントで既存 `surface.rs` の慣行に倣う。`#[allow]`・warning
  抑制・`-Werror` 緩和は行わない。
- R7: e2e test は static view（animation なし）で settle 後に
  capture し、`blocked/baseline_missing` → candidate → promote →
  `pass/match` → evidence 2 行を検証する。frame 間の pixel が
  deterministic であることを前提条件として文書化する。
- R8: 文書 — 本 slice の責務境界・capture 経路の選定理由・
  未検証環境（macOS/Linux・実 GPU 差分）を docs に記録する。

## Acceptance Criteria

| ID | 条件 | 検証 |
| --- | --- | --- |
| AC-1 | `capture_window` が実 window の client area RGBA8 を返し、`pixels.len() == w*h*4`・alpha=255・top-down を満たす | `golden_capture.rs` e2e test |
| AC-2 | 未知 window id・`run` 前の window は `UiError::Host`、OS handle なし（gpui-test 相当）は `UiError::Unsupported` | `capture_window_rejects_unknown_and_unopened_windows` test |
| AC-3 | `PrintWindow` primary / `BitBlt` fallback の 2 経路が実装され、使用経路が `OsCapturedFrame::backend` に記録される | code review + e2e test の backend assert |
| AC-4 | capture → `RgbaImage` → `BaselineStore::compare`/`write_candidate`/`promote`/`append_evidence` の e2e が `blocked` → `pass/match` で完走し、evidence JSONL が 2 行出る（実 window を開くため `#[ignore]` 付き・`--ignored` で明示実行） | `real_window_capture_feeds_golden_compare_to_pass` test |
| AC-5 | `gpui-test` adapter・Fake adapter の `capture_frame`/`capabilities()` が変更なく `Unsupported`・`VisualCapture` 非宣言を維持する | `mise run test-ui-scenario-contract*` 非回帰 |
| AC-6 | 非 Windows では `capture_client_area` が `Unsupported` を返し、build が通る | `mise run check`（host=Windows では cfg 経路の compile のみ）+ code review |
| AC-7 | 公開 signature に OS handle 型が現れず、`lib.rs` 変更が mod 宣言 + re-export のみ | diff review + `mise run check-ui-driver-boundary` |
| AC-8 | `unsafe` が局所 `#[expect(unsafe_code)]` + SAFETY コメントで、warning/lint 違反がない | `mise run lint` / `mise run check` |

## Deferred Design Concerns

- macOS / Linux の capture backend（ScreenCaptureKit / X11 等）|
  本環境は Windows のみ実測可能 | macOS/Linux 実機環境の確保時 |
  非 Windows での golden capture が `Unsupported` のまま残る
- 実 window 対象の `SemanticGuiTest`/`VisualGuiTest` adapter
  （`ScenarioDriver::RealOs`）| `GpuiDriver::run` の event-loop
  占有構造と scenario contract の適合は #313 系の設計課題 | #313
  real-OS driver 整備時 | scenario 層からの capture 呼び出しが
  driver 層 API の直接利用に留まる
- `PrintWindow` が返す内容と表示内容の差異（DWM 合成・
  occluded window）| `PW_RENDERFULLCONTENT` で DWM 合成面を
  取得するが、全 window class で表示と一致する保証はない |
  実利用で差異が観測された時 | fallback `BitBlt` との差分調査が
  必要になる可能性
- #426 Story fixture 接続（AudioLens / Synvelis）| slice 4 の範囲 |
  slice 4 | consumer golden の実適用が未着手のまま残る

## Sources and Relations

- Issue #314（slice 3 の対象 Issue。Owner 範囲: pixel screenshot
  取得・環境正規化・baseline 比較・差分承認・証拠）。
- Issue #464（consumer-neutral Scenario Contract — slice 2 が接続した
  契約層。本 slice はその `CaptureOutcome` の `Captured` 経路を
  driver 層で実装する）。
- Issue #390（Driver Contract Lab — verdict/evidence 語彙の owner）。
- Issue #426（Story fixture 接続 — slice 4、本 slice の直接対象外）。
- Issue #243（GPUI real-window Windows 検証 — `GpuiDriver::run`・
  `Slot::raw_parent` の OS handle 捕捉の先行実装。PR #482）。
- `docs/specs/issue-314-golden-core.md`・
  `docs/designs/issue-314-golden-core.md`（slice 1 — Golden
  Utility。PR #481 / merge `2230018e`）。
- `docs/specs/issue-314-golden-scenario.md`・
  `docs/designs/issue-314-golden-scenario.md`（slice 2 — Scenario
  Contract 接続。PR #483 / merge `7ae916b3`）。
