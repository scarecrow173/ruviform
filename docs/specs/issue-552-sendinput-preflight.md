---
id: "SPEC-034VeaErc3izqznL8Ycsk6"
type: "spec"
status: "approved"
title: "Issue #552 — real-OS input lane SendInput preflight（注入→実効 read-back で EnvironmentBlocked と GUI Fail を区別）"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/552"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-552-sendinput-preflight.md"
      - type: "file"
        path: "docs/test-specs/issue-552-sendinput-preflight.md"
      - type: "file"
        path: "docs/plans/issue-552-sendinput-preflight.md"
      - type: "file"
        path: "docs/tasks/issue-552-sendinput-preflight.md"
      - type: "file"
        path: "docs/impl/ir/issue-552-sendinput-preflight.md"
      - type: "file"
        path: "evidence/rf552-sendinput-preflight/report.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_smoke.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_pointer_smoke.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_multiwindow_smoke.rs"
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
    - "docs/specs/issue-313-windows-realos-smoke.md"
    - "docs/specs/issue-313-windows-pointer-smoke.md"
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
    - "evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md"
  defers: []
  deferred-by: []
---

# Issue #552 — real-OS input lane SendInput preflight（注入→実効 read-back で EnvironmentBlocked と GUI Fail を区別）

## What Should Be Built

3 つの real-OS input lane（`real_os_smoke` keyboard / `real_os_pointer_smoke`
pointer / `real_os_multiwindow_smoke` keyboard+pointer+chord）が共通して使う
**SendInput preflight** を `ruviform_ui_gpui::os_input` に実装し、各 lane 本体の
前に実行する。preflight は無害な注入（カーソルの小移動→復帰、または
修飾キー不要の単発キー down/up）を行い、`GetCursorPos` / `GetAsyncKeyState`
等の **SendInput とは独立した API** で実効を read-back 確認する。
SendInput の戻り値だけでは成功と判定しない。

preflight が不通（注入は受理されたが実効が確認できない、または注入 API 自体が
失敗）の場合、lane は本体を実行せず、verdict JSON に
`environment_blocked` を記録して専用の exit code で終了する。これにより
「環境要因で注入経路が死んでいる」状態と「GUI 実装が Fail した」状態を
verdict で機械的に区別できる。

## Why Now

2026-11-10 の SendInput 注入障害調査（`evidence/issue-243-multiwindow-verify/
environment-blocked-sendinput.md`）で、`SendInput` が戻り値成功
（`sent=1`/`sent=2`）を返しながら注入イベントの実効がゼロになる環境障害が
観測された（`LLMHF_INJECTED` 付きイベントのみがフィルタされ、
`mouse_event`/`keybd_event`/`SetCursorPos` は動作）。この状態では lane 本体が
実行されても click/focus/keystroke が一切届かず、結果は GUI 実装の Fail と
区別がつかない `fail` verdict になる — #243 の post-refactor 再検証が
まさにこの形で潰れた。Riot Vanguard 等の常駐ソフトや OS 状態により再発
し得るため、「注入が実際に効くか」を lane 本体の前に安価に検証し、
環境ブロックを正直な verdict として分離する必要がある。

## Users and Value

- Ruviform の検証実施者・consumer: real-OS lane の `fail` が
  「製品側の不具合」なのか「実行環境の注入ブロック」なのかを
  verdict JSON と exit code だけで判別でき、環境回復後の再実行要否を
  即断できる。
- 証拠の誠実性: environment-blocked な実行を GUI Fail として記録しない
  ことで、Issue #313/#243 系の capability 証拠が環境ノイズで
  毀損されるのを防ぐ。

## Scope

### In Scope

- `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs`: preflight API
  （scope 指定・read-back 検証・構造化レポート）と Win32 実装、
  および mock probe による blocked 分岐のユニットテスト。
- 3 つの real-OS example への preflight 配線と verdict/exit code の
  拡張（`environment_blocked` / exit 3）。
- `evidence/rf552-sendinput-preflight/`: 実走 verdict JSON と報告。
- 本 Issue 名の docs チェーン（spec/design/test-spec/plan/task/IR）。

### Out of Scope

- `mise.toml` の lane 追加・変更（タスク契約で禁止）。
- macOS/Linux backend、`mouse_event`/`keybd_event` 等の代替 API への
  移行、`BlockInput` 異常の診断。
- #243/#313 の GUI 実装・scenario 契約の変更。
- `ruviform-ui-test` への shared helper 昇格（両 crate で自然に共有
  できる場合のみ検討 — 今回は gpui crate 内に閉じる）。
- 公開 API の非互換変更（`os_input` は検証専用モジュールであり、
  追加のみで既存関数の契約は変えない）。

## Requirements

- SendInput の戻り値だけで成功と判定しない: 注入→実効の確認には
  `GetCursorPos`（ポインタ系）・`GetAsyncKeyState`（キー系）等の
  独立 API による read-back を使う。
- 無害な注入に限定する: カーソルは現在位置から小さい相対/絶対移動を
  行い直後に復帰させる。キーは修飾キーを伴わない未割当キー
  （F24）の down→read-back→up とし、実害を残さない
  （キー up は read-back 結果に関わらず必ず送る）。
- read-back の待機には現実的な timeout を設ける。
- preflight は lane 本体の前に実行し、不通なら本体を走らせず
  `environment_blocked` で終了する。
- preflight 失敗と lane 本体の失敗が verdict JSON で区別される
  （`verdict` 値と専用 exit code）。
- 既存の `pass`/`fail`/`unsupported` verdict・exit code 0/1/2 の
  意味を変えない。

## Acceptance Criteria

- [ ] 3 つの real-OS input lane が preflight で注入の実効を検証し、
  不通時に `environment_blocked` verdict と専用 exit code で終了する。
- [ ] preflight 失敗と lane 本体の失敗が verdict JSON で区別される
  （`environment_blocked` vs `fail`/`pass`）。
- [ ] preflight 通過時は lane 本体が従来どおり実行され、verdict JSON
  に preflight 結果が記録される。
- [ ] blocked 分岐がユニットテスト（mock probe）で検証される。
- [ ] `mise run fmt-check` / `check` / `lint` / `test` /
  `test-ui-gpui` / `check-ui-driver-boundary` が通る。

## Deferred Design Concerns

- 代替注入 API（`mouse_event`/`keybd_event`/`SendInput` 以外）への
  フォールバック | Issue #552 は「戻り値でなく実効を見る」ことが
  目的であり代替 API 移行は不要と明記 | 環境障害が恒久化した場合に
  別 Issue で検討 | SendInput が恒久的に死んだ環境では lane が
  永続的に environment_blocked になる。
- `BlockInput(TRUE)` 常時失敗の診断 | Issue #552 本文で別の診断事項と
  明記 | 別 Issue で扱う | 入力キュー全体のブロックと
  injected-flag フィルタの切り分けができない。
- preflight の macOS/Linux 対応 | 本 slice は Windows のみ
  （`os_input` の backend が Windows のみ） | 各 OS backend 実装時 |
  非 Windows では従来どおり `unsupported`。

## Sources and Relations

- Issue #552 本文（受入条件 2 行: preflight で注入実効を検証し
  不通時 EnvironmentBlocked / preflight 失敗と lane 本体失敗の
  verdict 区別）。
- `evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md`
  — SendInput 戻り値成功・実効ゼロの環境障害の実測記録と
  「`SendInput → GetCursorPos` preflight」の再検証条件。
- `docs/specs/issue-313-windows-realos-smoke.md` /
  `docs/specs/issue-313-windows-pointer-smoke.md` /
  `docs/specs/issue-243-windows-multiwindow-smoke.md` — 対象 lane の
  仕様・verdict schema の正本。
- `crates/ruviform-ui-test/src/driver_lab.rs` — driver-lab の判定
  語彙（`blocked` を含む 5 値）との関係整理は design で扱う。
