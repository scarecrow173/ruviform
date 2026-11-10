---
id: "TSPEC-034VedyL5Ej4Yc3P62YgUl"
type: "test-spec"
status: "approved"
title: "Issue #552 — SendInput preflight 検証契約"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/552"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-552-sendinput-preflight.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-552-sendinput-preflight.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-313-windows-realos-smoke.md"
    - "docs/test-specs/issue-313-windows-pointer-smoke.md"
    - "docs/test-specs/issue-243-windows-multiwindow-smoke.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-552-sendinput-preflight.md"
    - "docs/designs/issue-552-sendinput-preflight.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-552-sendinput-preflight.md"
    - "docs/designs/issue-552-sendinput-preflight.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #552 — SendInput preflight 検証契約

## Purpose

Issue #552 の preflight が「SendInput の戻り値でなく実効を read-back で
確認する」「不通を `environment_blocked` として lane 本体失敗と区別する」
という契約を、実 OS に依存しない mock probe のユニットテストで固定する。
実機 lane の verdict 証拠（evidence/）は本 spec の対象外 — 手動検証
証拠として別途記録する。

## Feature

SendInput preflight は、注入イベントが OS に受理されても独立 API の
read-back で実効が確認できなければ `effective=false` を返し、
呼び出し側が lane 本体を実行せず `environment_blocked` で終了できる。

## Rules

- **Rule: 実効は read-back のみで判定する** — `SendInput`/`move_to`/
  `inject_key` が `Ok` を返しても、`cursor_position`/`key_down_state`
  の観測が目標に達しなければ `effective=false`。
- **Rule: blocked でも副作用を残さない** — pointer probe はカーソルを
  原点へ復帰させ、keyboard probe は read-back 結果に関わらず key up を
  送る。
- **Rule: scope は要求系のみ検証する** — `Pointer` は pointer のみ、
  `Keyboard` は keyboard のみ、`PointerAndKeyboard` は両方。
  `report.effective` は要求された全 check が effective の時のみ true。
- **Rule: 失敗理由は detail に残る** — `effective=false` の check は
  失敗段階（API エラー / read-back timeout / 復帰不可等）を `detail`
  に含む。

## Examples

- **Example: pointer 注入が受理されるがカーソルが動かない**
  Given mock probe で `move_to` が `Ok` を返すが `cursor_position`
  が常に原点を返す
  When `preflight(PreflightScope::Pointer)` を実行する
  Then `report.effective == false`、pointer check の `effective ==
  false` で `detail` が read-back timeout を示し、復帰 `move_to`
  (origin) が試行される。
- **Example: keyboard 注入が受理されるがキーが down にならない**
  Given mock probe で `inject_key(down)` が `Ok` だが
  `key_down_state` が常に false
  When `preflight(PreflightScope::Keyboard)` を実行する
  Then `report.effective == false`、keyboard check が ineffective、
  かつ `inject_key(up)` が必ず呼ばれる（キー押しっぱなしを残さない）。
- **Example: 両系統が実効する**
  Given mock probe が move/key とも実効をシミュレートする
  When `preflight(PreflightScope::PointerAndKeyboard)` を実行する
  Then `report.effective == true`、checks は pointer→keyboard の順で
  2 件。
- **Example: scope 選択**
  Given `PreflightScope::Keyboard`
  When `preflight` を実行する
  Then checks は keyboard のみ（pointer probe は呼ばれない）。
- **Example: probe_target の画面端処理**
  Given 原点が仮想画面右端・下端
  When `probe_target` を計算する
  Then 目標は内側方向（-16,-16）に取り、両軸に 16px の余裕がない
  場合のみ `None`。

## Guarantees

- `preflight` は `Result` を返さず、いかなる probe 失敗も
  `PreflightReport` の `effective=false` + `detail` として表現する
  （呼び出し側が panic/例外なしに verdict 分岐できる）。
- keyboard probe は `inject_key(up)` を read-back 結果に関わらず
  必ず 1 回送る。
- pointer probe は `move_to(origin)` による復帰を必ず試行する。
- ユニットテストは実 OS の注入・カーソル・キー状態に一切触れない
  （mock probe のみ）。

## Non-goals

- 実機での preflight 実効・環境ブロックの再現（手動 lane の証拠で
  別途扱う — 本 spec はアルゴリズムの分岐を固定する）。
- verdict JSON スキーマの機械的検証（example ごとの ad-hoc schema で
  自動 consumer が存在しない）。
- 複数回 retry・backoff・診断の高度化。

## Risk

blocked 判定ロジックが実効側に倒れる（false pass）と環境障害を
見逃して lane 本体が無駄に fail する。逆に effective 側に倒れる
（false blocked）と健全な環境で lane が起動しない。tolerance・
timeout・probe_target の端処理を mock テストで固定することで
この両方向のリスクを抑える。
