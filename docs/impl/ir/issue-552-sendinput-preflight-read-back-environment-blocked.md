---
id: "IMPL-034VeXFI5DET4CkhWJrBln"
type: "impl"
status: "in-progress"
title: "Issue #552 SendInput preflight（注入→実効 read-back で environment_blocked を区別）"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/552"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_smoke.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_pointer_smoke.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_multiwindow_smoke.rs"
      - type: "file"
        path: "docs/specs/issue-552-sendinput-preflight.md"
      - type: "file"
        path: "docs/designs/issue-552-sendinput-preflight.md"
      - type: "file"
        path: "docs/test-specs/issue-552-sendinput-preflight.md"
      - type: "file"
        path: "docs/plans/issue-552-sendinput-preflight.md"
      - type: "file"
        path: "docs/tasks/issue-552-sendinput-preflight.md"
      - type: "file"
        path: "evidence/rf552-sendinput-preflight/"
    modified:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-552-sendinput-preflight.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-552-sendinput-preflight.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-552-sendinput-preflight.md"
    - "docs/designs/issue-552-sendinput-preflight.md"
    - "docs/test-specs/issue-552-sendinput-preflight.md"
    - "docs/plans/issue-552-sendinput-preflight.md"
    - "evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-552-sendinput-preflight.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #552 SendInput preflight（注入→実効 read-back で environment_blocked を区別）

## Summary

`SendInput` がイベント受理（戻り値 = 期待数）を返しながら実効ゼロに
なる環境障害（2026-11-10 観測、証拠:
`evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md`）
を、real-OS lane が GUI `fail` と誤分類しないよう、注入の実効を独立
OS API の read-back で検証する共通 preflight を `os_input` に追加し、
3 lane（keyboard / pointer / multiwindow）の lane 本体前に実行する。
preflight 不通時は verdict `environment_blocked`・exit code `3` で
終了し、lane 本体（window 生成・注入シーケンス）は走らない。

## Context

Issue #552。spec → design → test-spec → plan → task の文書チェーン
（`docs/{specs,designs,test-specs,plans,tasks}/issue-552-sendinput-preflight.md`）
に従う。`SendInput` の戻り値だけでは注入の実効を保証できないという
不変条件違反を、`os_input` 層の preflight で修正した。

## Implementation

### Changes

- `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs`
  - 公開 API 追加: `PreflightScope`（`Pointer` / `Keyboard` /
    `PointerAndKeyboard`、`#[non_exhaustive]`）、`PreflightCheck`
    （family / effective / detail）、`PreflightReport`（effective /
    checks）、`preflight(scope) -> PreflightReport`。
  - 内部 `Probe` trait（cursor_position / move_to / virtual_screen /
    sleep / inject_key / key_down_state）を OS seam として導入し、
    アルゴリズム（`preflight_inner` / `probe_pointer` /
    `probe_keyboard` / `probe_target` / `cursor_near`）を
    プラットフォーム非依存に実装。Windows `imp::OsProbe` が
    `GetCursorPos` / `GetSystemMetrics` / `SendInput` /
    `GetAsyncKeyState` で実装し、非 Windows は `UiError::Unsupported`
    を返す既存方針を維持。
  - pointer probe: カーソル原点を `GetCursorPos` で読み、
    `GetSystemMetrics` の virtual screen 内側へ絶対移動 16px
    （`MOUSEEVENTF_ABSOLUTE|VIRTUALDESK` — 相対移動は pointer
    acceleration の影響を受けるため不採用）→ `GetCursorPos` を
    25ms 間隔・最大 80 回（約 2 秒）poll し ±8px 以内を確認 →
    成否に関わらず原点へ復帰を試行。
  - keyboard probe: `VK_F24`（0x87、割当なし・修飾なし）を
    key-down 注入 → `GetAsyncKeyState` high bit を同様に poll →
    read-back の成否に関わらず key-up を必ず送信（押しっぱなし防止）。
  - `normalized` の virtual screen 取得を `virtual_screen()` に
    切り出して probe と共有。
  - mock `Probe` によるユニットテスト 8 件（blocked/effective、
    scope 選択、key-up 必達、restore 試行、`probe_target` の
    端処理・負座標 virtual screen、read-back tolerance）。
- `examples/real_os_smoke.rs`: `PreflightScope::Keyboard` を
  supported 判定直後・`run_smoke` 前に実行。不通時は
  `blocked_doc`（verdict `environment_blocked`、schema v2、
  `preflight` 記録、`run: null`）を書き exit `3`。通過時は
  `Smoke.preflight` に保持し verdict JSON の `preflight` に記録。
  schema `rf313-real-os-smoke/v1` → `/v2`。
- `examples/real_os_pointer_smoke.rs`: 同上
  （`PreflightScope::Pointer`、schema
  `rf313-real-os-pointer-smoke/v1` → `/v2`）。preflight は
  `cursor_home` 取得より前に実行。
- `examples/real_os_multiwindow_smoke.rs`: 同上
  （`PreflightScope::PointerAndKeyboard` — lane は pointer click と
  keyboard/chord の両方を注入し、ローカル chord injector も同一
  SendInput 経路を使う。schema
  `rf243-real-os-multiwindow-smoke/v1` → `/v2`）。
- verdict スキーマ判断: `verdict` 文字列に `environment_blocked`
  を追加し `category` フィールドは増やさない。理由: 既存呼び出し側は
  `verdict` 文字列で pass/fail/unsupported を判定しており、
  新しい top-level 値は既存値と衝突しない（design §verdict 記録）。
  schema version を v1→v2 に上げ、`preflight` オブジェクト
  （effective + checks[{family,effective,detail}]）を全 verdict に
  追加。unsupported 時は `preflight: null`。

### Decisions During Implementation

- read-back API は `GetAsyncKeyState` を採用（`GetKeyState` は
  呼び出しスレッドの message queue 依存で注入イベントを観測でき
  ない場合がある）。
- keyboard probe の key-up は read-back 失敗時も必ず送信。
  key-up 自体の注入失敗は detail に記録するが effective 判定は
  down の実効確認に基づく（F24 は無害・未割当）。
- pointer probe の target は virtual screen 内側方向へ各軸独立に
  選択（モニタ端・負座標 origin 対応）。画面が小さすぎる場合は
  `effective=false`（detail に理由）。
- timeout は 25ms × 80 = 約 2 秒。実害のない注入のみ
  （カーソル 16px→復帰、F24 down/up）。

## Related Experiments

### Adopted

なし（実装方針は design で確定済み。blocked 分岐は mock Probe の
ユニットテストで検証し、実環境での自然発生は NotRun 記録とする）。

### Rejected

なし。

## Validation

- `cargo test -p ruviform-ui-gpui --features gpui-test os_input`:
  8 tests passed（補助証拠・canonical は `mise run test-ui-gpui`）。
- `mise run lint-ui-gpui`: Passed。
- `mise run fmt-check`: Passed。
- 残りの mise 検証・実機 lane は検証節の最終結果を参照
  （本レコードは検証完了時に更新する）。

## Risks

- `GetAsyncKeyState` は物理キー状態も反映するため、probe 中に
  ユーザーが F24 を物理的に押すと false positive になり得るが、
  F24 は一般に未割当・物理キーボードにも存在しないため実害は
  ほぼない。
- preflight 通過後〜lane 本体の間に環境が変化する可能性は残る
  （preflight は時点のスナップショット）。lane 本体の失敗は従来
  どおり `fail` で報告される。
- schema v1→v2 の変更は verdict JSON を消費する既存証拠
  スクリプトに影響し得る — `preflight` は追加フィールドであり
  既存フィールドは不変、`verdict` の新値 `environment_blocked` は
  既存値と衝突しない。

## Follow-ups

- 実機 lane の verdict JSON を `evidence/rf552-sendinput-preflight/`
  に保存（実行可能な範囲で）。
- preflight 不通の実環境再現は自然発生待ち（強制的な障害注入は
  行わない — Task Spec の指示どおり）。
