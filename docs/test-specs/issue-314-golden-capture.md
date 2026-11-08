---
id: "TSPEC-034Ug943CVpTG4Br56eTfE"
type: "test-spec"
status: "approved"
title: "Issue #314 slice 3 — GPUI/OS capture backend test spec"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-314-golden-capture.md"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/golden_capture.rs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-314-golden-capture.md"
    - "docs/designs/issue-314-golden-capture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-314-golden-scenario.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-314-golden-capture.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-314-golden-capture.md"
    - "docs/designs/issue-314-golden-capture.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 3 — GPUI/OS capture backend test spec

## Purpose

実 OS window pixel capture（`GpuiDriver::capture_window`）が
「実在する pixel のみを返し、取得不能時に正直な error を返し、
golden e2e で `pass/match` に到達する」ことを固定する。
semantic/draw-plan の証拠昇格・pixel 捏造・fake `pass` を
構造的に排除することが目的。

## Feature

GPUI real-window OS pixel capture and golden verification

## Rules

- R1: `capture_window` は `run` で open 済みの実 OS window の
  client area RGBA8（`w*h*4` bytes、alpha=255、top-down）を返す。
- R2: 未知 window id・`run` 前（未 open）の window は
  `UiError::Host`、OS handle を持たない window（`gpui-test`
  platform 相当）は `UiError::Unsupported` — いずれも pixel を
  返さない。
- R3: Windows backend は `PrintWindow(PW_CLIENTONLY|
  PW_RENDERFULLCONTENT)` を第一経路、screen `BitBlt` を
  fallback とし、使用経路を `OsCapturedFrame::backend` に記録
  する。
- R4: minimized/hidden window・dead handle・空 client area・
  GDI 失敗は `UiError::Host` — verdict や `pass` に化けない。
- R5: capture → `RgbaImage` → `BaselineStore` の e2e は、
  settle 後の deterministic frame に対して `blocked/
  baseline_missing`（初回）→ candidate → promote → `pass/match`
  （2 回目）を返し、evidence JSONL が 2 行出力される。
- R6: `gpui-test`/Fake adapter の `capture_frame`・
  `capabilities()` は `Unsupported`・`VisualCapture` 非宣言を
  維持する（非回帰）。

## Examples

- Given `GpuiDriver` が window を `run` で open し `Ready` に
  達した後 30 tick settle した状態、
  When `capture_window` を呼ぶと、
  Then `OsCapturedFrame`（1280×800 程度、`pixels.len() ==
  w*h*4`、alpha=255）が返り、`backend` が
  `win32-printwindow` または `win32-bitblt-screen`。
- Given 同じ window に対し 2 回連続で capture した場合、
  When `BaselineStore` で初回 compare → candidate → promote →
  2 回目 compare を行うと、
  Then 初回は `blocked/baseline_missing`、2 回目は `pass/match`
  で `mismatched_pixels == 0`。
- Given 存在しない `WindowId` または `create_window` で
  予約済みだが `run` 未実行の window、
  When `capture_window` を呼ぶと、
  Then `Err(UiError::Host)` が返り pixel は得られない。

## Guarantees

- semantic observation・`DrawnPlan`・observer 出力が pixel
  証拠として `Captured` に昇格しない。
- capture 失敗が `GoldenVerdict::Pass` に化けない
  （`UiError` → caller が `NotRun`/`Unsupported`/`Driver`
  error として記録）。
- `unsafe` は backend module の `#[expect(unsafe_code)]` +
  SAFETY コメントに限定され、`#[allow]`・warning 抑制なし。
- 公開 signature に OS handle 型（HWND/HDC 等）が現れない。

## Non-goals

- macOS/Linux backend — 本 spec は Windows のみ検証。
- `gpui-test`/Fake adapter の pixel capture — pixel pipeline が
  存在しないため `Unsupported` が正しい動作。
- 実 window `SemanticGuiTest`/`VisualGuiTest` adapter —
  `ScenarioDriver::RealOs`（#313）の範囲。
- animated/動的 view の golden — static view のみで検証
  （Spectrogram/TimeAxis 等の per-frame 差分は対象外）。
- `PrintWindow` 出力と表示内容の完全一致保証 — window class
  依存の差異は残り得る（fallback `BitBlt` が real pixel を
  提供）。

## Risk

- settle tick 数（30）が環境依存で flaky になる可能性 —
  frame 間 0 diff を実測確認したが、GPUI の描画完了 event が
  ないため固定待機に依存。環境差で失敗した場合は待機時間の
  再実測が必要。
- `PrintWindow` が特定 window class で失敗/不正内容を返す
  可能性 — fallback `BitBlt` が real pixel を返すため capture
  自体は成立するが、ambient 混入の可能性が `backend` tag で
  識別可能。
- e2e test が実デスクトップ window を開くため、CI の
  ヘッドレス環境では実行不可 — `#[cfg(all(windows,
  feature = "gpui"))]` gate + `#[ignore]` で opt-in とし、
  `cargo test --features gpui --test golden_capture --
  --ignored` での明示実行を前提とする（error path test は
  window を開かないため通常 lane で実行される）。
