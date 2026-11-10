---
id: "DESIGN-034VedvLujxpzyWOjxyoD4"
type: "design"
status: "approved"
title: "Issue #552 — SendInput preflight 設計（注入→実効 read-back・EnvironmentBlocked verdict）"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/552"
  changes:
    added:
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
  implements:
    - "docs/specs/issue-552-sendinput-preflight.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-313-windows-realos-smoke.md"
    - "docs/designs/issue-313-windows-pointer-smoke.md"
    - "docs/designs/issue-243-windows-multiwindow-smoke.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-552-sendinput-preflight.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-552-sendinput-preflight.md"
  references:
    - "evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md"
  defers: []
  deferred-by: []
---

# Issue #552 — SendInput preflight 設計（注入→実効 read-back・EnvironmentBlocked verdict）

## Context

`os_input`（`crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs`）は
Win32 `SendInput` の薄い注入層で、`send()` が「受理イベント数 == 期待数」
のみを成功条件とする。2026-11-10 の障害調査で、`SendInput` が全イベントを
受理しながら実効ゼロになる環境（injected-flag フィルタ型）が実在することが
確認された。この場合 lane は注入呼び出し自体は成功し、最終的に
「intent が届かない」= `fail` verdict となり、環境ブロックと GUI 実装
Fail を区別できない。

preflight は lane 本体の前段に置く **環境ゲート** であり、製品コードの
挙動には一切関与しない。責務は「この OS セッションで SendInput 注入が
実効を持つか」を独立 API で確認し、結果を構造化して返すことだけ。

## Scope

### In Scope

- `os_input` への preflight API 追加（公開型・関数・Windows 実装・
  非 Windows honest fallback）。
- preflight アルゴリズムと probe seam（ユニットテスト可能な構造）。
- 3 example の呼び出し配線・verdict/exit code 拡張。
- verdict JSON スキーマの版管理判断（v1→v2）と影響記録。

### Out of Scope

- 既存注入関数（`press_key`/`move_to`/`mouse_down`/`mouse_up`/
  `cursor_position`）の契約変更・内部実装変更。
- example-local chord injector（multiwindow）の変更 — 同じ SendInput
  経路を通るため keyboard probe で代表される。
- `ruviform-ui-test`・`mise.toml`・他 crate。

## Components and Boundaries

- `os_input::preflight(scope: PreflightScope) -> PreflightReport`
  — 唯一の公開入口。scope で検証する注入系（pointer / keyboard /
  両方）を選ぶ。戻り値は失敗しない（`Result` ではなく report）—
  「検証できなかった」自体が診断対象の情報であり、呼び出し側が
  `effective` で分岐する。
- `PreflightScope`（`#[non_exhaustive]` enum）: `Pointer` /
  `Keyboard` / `PointerAndKeyboard`。
- `PreflightReport { effective: bool, checks: Vec<PreflightCheck> }`、
  `PreflightCheck { family: &'static str, effective: bool,
  detail: String }` — example が verdict JSON に転記するための
  プレーンなデータ。serde には依存しない（`os_input` は現状
  serde 非依存のまま）。
- 内部 `Probe` trait — preflight アルゴリズムが依存する最小の
  OS seam（`cursor_position`/`move_to`/`virtual_screen`/`sleep`/
  `inject_key(vk, up)`/`key_down_state(vk)`）。Windows `imp` が
  Win32 で実装し、`#[cfg(test)]` の mock が blocked/effective を
  演符する。公開 API は増やさない。

## Data and Control Flow

### Pointer probe

1. `cursor_position()`（`GetCursorPos`）で原点を取得。
2. `virtual_screen()`（`GetSystemMetrics` 4 値、`normalized()` と
   共通化して重複を排除）で論理デスクトップ範囲を取得。
3. 目標点 = 原点 + 各軸 16px。軸ごとに「+16 が画面内に収まるか」
   を判定し、収まらなければ -16、それも無理なら probe 失敗
   （画面が異常に小さい環境のみ）。
4. `move_to(target)`（既存の絶対座標 SendInput 経路）を注入。
5. read-back ループ: `sleep(poll)` → `cursor_position()` を最大
   `timeout/poll` 回繰り返し、`|actual - target| <= 8px`（両軸）
   になった時点で effective。既定 timeout 2s / poll 25ms。
6. 結果に関わらず `move_to(origin)` でカーソルを復帰（best-effort）。

絶対移動を選ぶ理由: 相対移動（`MOUSEEVENTF_MOVE` 非絶対）は
ポインタ加速設定の影響を受け、read-back の判定閾値を安定して
置けない。絶対移動は正規化座標で決定的。

### Keyboard probe

1. `inject_key(VK_F24, down)` — `VK_F24`（0x87）は一般に未割当の
   ファンクションキーで、修飾キーを伴わず実害がない。foreground
   window があっても F24 の keydown は通常どのアプリも処理しない。
2. read-back ループ: `sleep(poll)` → `key_down_state(VK_F24)`
   （`GetAsyncKeyState` の high bit）を timeout まで繰り返す。
   `GetKeyState` ではなく `GetAsyncKeyState` を使う理由:
   `GetKeyState` は呼び出しスレッドのメッセージキュー経由でしか
   更新されず、メッセージポンプを回さない example 主スレッドでは
   実効を観測できない。`GetAsyncKeyState` はセッション全体の
   非同期キー状態を返し、SendInput 注入分も反映する。
3. read-back の結果に関わらず `inject_key(VK_F24, up)` を必ず送る
   — down が実効したのに up を送らないとキーが押しっぱなしになる。
   up の送信失敗も detail に記録する。

### 失敗の分類

probe が返す `effective=false` の detail は失敗段階を含む文字列
（例: `cursor_position: host operation failed: ...`、
`read-back timed out: cursor stayed at (x,y), target (x2,y2)`、
`SendInput accepted 0/1 events`）。verdict 側はこれを
`environment_blocked` の reasons/detail に転記するだけで、
さらに細分化しない（KISS — 判定は effective の 2 値）。

### Example 配線

各 example の `main()` で `os_input::supported()` チェックの直後・
lane 本体（`run_smoke`）の前に `os_input::preflight(scope)` を実行:

- `real_os_smoke` → `PreflightScope::Keyboard`
- `real_os_pointer_smoke` → `PreflightScope::Pointer`
- `real_os_multiwindow_smoke` → `PreflightScope::PointerAndKeyboard`
  （chord injector も同一 SendInput keyboard 経路のため keyboard
  probe で代表）

`!report.effective` なら verdict JSON を `environment_blocked` で
書き出して exit code 3 で終了。通過時は `PreflightReport` を
Smoke/State に保持し、通常 verdict の `preflight` フィールドに
転記する。

### Verdict / exit code の決定

- verdict 値: `"environment_blocked"`。既存 `pass`/`fail`/
  `unsupported` と衝突せず、Issue の用語 `EnvironmentBlocked` と
  一致。driver-lab の `blocked`（依存未解決の計画語彙）とは別系統
  の lane verdict であり、名前を分けて誤読を防ぐ。
- exit code: `3`。0=pass / 1=fail / 2=unsupported の次の空き。
  「blocked 区分」の既存 exit code 慣行は存在しないため新設し、
  各 example の doc comment・`--help`・本 design・evidence に記録
  する（`mise.toml` の説明文は編集範囲外のため更新しない）。
- schema: 各 lane の `"schema"` を `.../v1` → `.../v2` に上げ、
  `"preflight"` オブジェクト（`{effective, checks[]}`、
  unsupported 時は `null`）を追加する。verdict 語彙の拡張は
  スキーマ互換を壊す変更なので版を上げるのが誠実。自動 consumer は
  存在せず（manual verification lane）、report.md/results.jsonl は
  人手管理のため影響は文書更新のみ。

## Non-Functional Constraints

- **RT contract:** non-RT only（既存どおり — テスト/手動検証経路）。
- **非破壊性:** preflight の注入はカーソル ±16px の往復と
  未割当キー 1 回の down/up のみ。カーソルは必ず復帰を試み、
  キー up は必ず送る。
- **時間コスト:** probe あたり最悪 ~2s、PointerAndKeyboard で ~4s。
  通常時は数十 ms。
- **unsafe:** 既存 `#[expect(unsafe_code)]` + SAFETY コメントの
  局所許可パターンを踏襲（`GetAsyncKeyState` も同様）。
- **lint:** 公開 enum `PreflightScope` に `#[non_exhaustive]`。
  pedantic clippy（`cast_possible_truncation` 等）に注意。

## Risks and Trade-offs

- **ユーザーの実カーソルを一瞬動かす:** ±16px の往復で復帰するが、
  preflight 実行中にユーザーがマウスを操作すると read-back が
  目標点から外れて false blocked になり得る。発生確率は低く、
  再実行で回復する。受容（より大きな移動や SetCursorPos による
  回避は副作用が増える）。
- **画面端での移動方向:** 仮想画面（マルチモニター・負原点対応）
  の内側方向を軸ごとに選ぶことで端クランプを回避。両方向に
  16px の余裕がない異常環境では probe を失敗として正直に返す。
- **GetAsyncKeyState の信頼性:** 注入キーの down 状態を反映するが、
  物理 F24 押下や他ソフトの干渉は実質あり得ない。読み取り自体は
  必ず成功する API（失敗しない）なので read-back 側のエラー分岐は
  不要。
- **SendInput 部分受理:** 1 イベント注入で `sent != 1` は
  「0 受理」のみ意味し、down が中途半端に残ることはない。
  キー up は常に送る設計で残留リスクを潰す。
- **verdict schema v2:** v1 を読む手動手順はファイル名参照のため
  実害なし。PR 本文と evidence report に版変更を明記する。

## Deferred Design Concerns

- preflight 結果の driver-lab `blocked` 語彙への写像 | lane verdict
  と driver-lab manifest は別スキーマであり、consumer-verify の
  lane status 導出は既に `unsupported`/`not_run` を扱う。
  `environment_blocked` をどう写像するか（恐らく `not_run` 相当）
  は consumer-verify 接続時に決める | consumer-verify が
  real-OS lane を自動収集するようになった時 | 現状 manual lane の
  ため写像先が存在しない。
- preflight の retry/backoff | 一過性の遅延で blocked 誤判定する
  可能性は timeout 2s で実用上十分 | 実測で誤判定が観測されたら
  設定化を検討 | 現状 timeout は固定値。

## Verification Notes

- [x] mock `Probe` によるユニットテストで blocked/effective 両分岐・
  スコープ選択・キー up 必達を検証（design 確定前の妥当性確認を兼ねる）。
- [x] `probe_target` の画面端ケースを純粋関数として単体検証。
- [ ] 実機 lane 実行で preflight 通過→本体実行の一連を evidence 化
  （実装後）。
