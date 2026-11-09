---
id: "TSPEC-rf388DockWindowing01"
type: "test-spec"
status: "approved"
title: "Issue #388 slice 3 windowing 挙動の実機テスト仕様（Windows / VST3 / L0）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc/dock_windowing.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc.rs"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-388-native-dock-windowing-slice3.md"
    - "docs/designs/issue-388-native-surface-slot.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-388-native-dock-windowing-slice3-implementation.md"
    - "evidence/rf388-native-dock-windowing/report.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-388-native-dock-windowing-slice3.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-388-native-dock-windowing-slice3.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  defers: []
  deferred-by: []
metadata:
  issues: [388]
---

# Issue #388 slice 3 windowing 挙動テスト仕様

## 目的

`poc-native-dock-windowing` scenario が、Windows 11 実セッションで
Vinyl.vst3（および probe fallback）の実 editor に対し、
ancestor clip・keyboard/focus routing・re-dock/float 2-phase の
3 領域を `[poc] check=` 行として記録する。Pass は Dock PASS
を意味しない（matrix の残行が残るため）。

## Feature / Rule / Example

### F1 ancestor clip / overhang

- R1-1: window を slot `min_size` 未満に resize したとき、
  projection が可視部 clip を push し、`GetWindowRgnBox` が
  期待する container-local rect と一致する。
- R1-2: `surface.set_clip(slot, Some(rect))` 直叩きで
  `GetWindowRgnBox` がその rect に一致し、`WindowFromPoint`
  が clip 内では plugin 子孫 HWND、clip 外では GPUI frame を
  返す（OS hit-test で clip 効果を確認）。
  注: GPUI 0.2.2 の DirectComposition（`CreateTargetForHwnd`
  `topmost=true`）により child HWND の画素は画面に合成されない
  ため、screen pixel による clip 視覚確認は実施不能
  （`evidence/rf388-native-dock-windowing/report.md` の
  「重大な発見」参照）。hit-test を代替証拠とする。
- R1-3: clip 解除（`None` / window 復帰）で region が消える。

### F2 keyboard / focus routing

- R2-1: `request_focus` で `GetFocus == container`。
- R2-2: container focus 中の `keybd_event` が plugin tree の
  HWND に配送されない（WH_GETMESSAGE 記録で確認）。
- R2-3: plugin editor への実クリック後、focus が plugin tree
  に移り、キーが plugin tree の HWND に配送される。
- R2-4: 再度 `request_focus` で container に戻り、キーが
  container に留まる（境界の双方向性）。

### F3 re-dock / float 2-phase

- R3-1: `DockWorkspace::begin_move`→`commit_move` が
  MovePrepared/MoveCommitted event を発行し、busy 区間で
  scratch slot の `lease_parent` が `SlotBusy`、clear 後に成功。
- R3-2: `detach_view` → spec から NativeSlot 除去で slot
  Hidden・editor 非表示だが HWND/parent は生存。
- R3-3: `attach_view` → spec 復帰で同一 editor HWND が
  再表示される。

### 共通

- teardown: close/unbind/release_lease/release_slot/drain
dock.rs と同じ順序で全 check が pass。
