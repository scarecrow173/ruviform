---
id: "DESIGN-rf388DockWindowing01"
type: "design"
status: "approved"
title: "Issue #388 slice 3 設計 — slot projection clip 修正と windowing PoC scenario"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-388-native-dock-windowing-slice3.md"
    - "docs/designs/issue-388-native-surface-slot.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-388-native-dock-windowing-slice3.md"
    - "docs/test-specs/issue-388-native-dock-windowing-slice3.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-388-native-dock-windowing-slice3.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-388-native-dock-windowing-slice3.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 388
---

# Issue #388 slice 3 設計

## 1. slot projection の clip 計算修正（render.rs）

現行 `project_slot_frame` は `window.content_mask().bounds`
（window-local 座標）を `window.bounds()`（screen 座標）と比較して
おり、座標系が異なるため常に不一致 → clip が常に
`Some(viewport 全域)` になる。実害は `SetWindowRgn` が全域
no-op になるだけだが、ledger 上「clip あり」と記録され、
`None`（非 clip）状態を表現できない。

修正: clip を **要素 bounds ∩ content mask** とし、それが要素
bounds を狭めないとき `None` とする。純粋関数
`slot_clip(bounds, mask) -> Option<LogicalRect>` に切り出して
単体テスト可能にする。

- 要素が mask 内に完全に収まる → `None`（port は非 clip、
  OS の親 client-area clip に委ねる）。
- 要素が viewport/mask をはみ出す → `Some(可視部)` →
  `clip_local` → `SetWindowRgn` が実 clip を適用。
- 要素が mask の外に完全にある → `Some(空 rect)` →
  clip で完全に隠す（負サイズは 0 に clamp）。

`surface.rs` の `set_clip`/`clip_local`/`SetWindowRgn` 経路は
既に正しく、変更しない。

## 2. PoC scenario `dock-windowing`（poc/dock_windowing.rs）

`RVF_POC_SCENARIO=dock-windowing`、mise task
`poc-native-dock-windowing`。dock.rs と同じ Panel→Tabs
`NativeSlot("dockw.editor")` 構成で、以下の stage を逐次実行:

1. **attach** — declare/bind/await projection/lease/open
   （dock.rs と同経路、check 名は `dockw_` 系で区別）。
2. **clip_overhang** — `drv.resize` で window を slot の
   `min_size` 未満に縮め、projection が `Some(可視部)` を
   push したことを `GetWindowRgnBox(container)` で検証 +
   screenshot。window を戻して clip 解除を確認。
3. **clip_direct** — `surface.set_clip(slot, Some(部分 rect))`
   を port 直叩きし、region box と pixel で clip が効くことを
   検証後 `set_clip(None)` で解除。
4. **focus_keys** — `request_focus` → container focus 確認 →
   `keybd_event` でキー送出 → `WH_GETMESSAGE` hook の
   `(hwnd,msg)` 記録で container 宛を確認 → plugin editor
   配下へ実クリック（`SetCursorPos`+`mouse_event`）→ focus が
   plugin tree に移ることを確認 → キー送出で plugin tree 宛を
   確認 → `request_focus` で container に戻し境界を再確認。
5. **redock** — 実 `DockWorkspace`:
   `register_window`/`attach_view` → `begin_move`+
   `commit_move`（MovePrepared/MoveCommitted event 確認、
   busy 区間で scratch slot の `mark_busy`→`lease_parent` が
   `SlotBusy`、`clear_busy` 後に成功）→ `detach_view` →
   `update_view` で NativeSlot を除いた spec に差し替え
   （slot Hidden・editor 非表示・HWND 生存を確認）→
   `attach_view` で戻し spec 復帰（同一 editor HWND が再表示）。
6. **teardown** — close/unbind/release/drain events。

### WH_GETMESSAGE hook

`SetWindowsHookExW(WH_GETMESSAGE, proc, hmod,
GetWindowThreadProcessId(自 window))` で自スレッドの
message queue のみ観測。`MSG.hwnd`/`MSG.message` を
`Mutex<Vec>` に記録し、key 送出後に drain して
container/plugin tree への配送を判定する。hook は
scenario 終了時に `UnhookWindowsHookEx` で解除。

### plugin tree 判定

`first_child_of(container)` = editor HWND（probe は
`child_hwnd`、VST3 は列挙）。`EnumChildWindows(editor)` で
descendants を集め `{editor} ∪ descendants` を plugin tree
とする。`GetFocus`/`hook hwnd` がこの集合に含まれるかで
「plugin 側に届いた」を判定。

## 3. 証拠・文書

- `evidence/rf388-native-dock-windowing/report.md` +
  生ログ + screenshot。
- design doc `issue-388-native-surface-slot.md` D6 matrix の
  Windows/VST3/L0 行を実測結果で更新（未実測は維持）。
- `docs/impl/ir/` に実装記録、`docs/test-specs/` に本 slice の
  test-spec。
