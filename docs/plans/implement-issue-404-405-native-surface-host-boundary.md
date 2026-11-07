---
id: "PLAN-ECzI54BQngMiQpOP23flK3"
type: "plan"
status: "draft"
title: "Issue #404/#405 NativeSurfaceSlot host 境界修復 実装計画"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/404"
    - "https://github.com/scarecrow173/ruviform/issues/405"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-404-405-native-surface-host-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-404-405-native-surface-host-boundary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-388-389-native-dock-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-404-405-native-surface-host-boundary.md"
  derived-by:
    - "docs/tasks/issue-404-405-native-surface-host-boundary.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-404-405-native-surface-host-boundary.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [404, 405]
---

# Issue #404/#405 NativeSurfaceSlot host 境界修復 実装計画

正本の設計: `docs/designs/issue-404-405-native-surface-host-boundary.md`。
変更は `crates/ruviform-ui-gpui/src/gpui_driver/surface.rs` に閉じる
（`lib.rs`・`ruviform-ui` neutral 契約・PoC harness 無変更）。

## ファイルマップ

| ファイル | 変更 | 内容 |
| --- | --- | --- |
| `crates/ruviform-ui-gpui/src/gpui_driver/surface.rs` | modified | `Binding.id`・`binding_dead`/`sweep_dead`・`&self` 実効クエリ・`set_clip`/`set_geometry`/ledger 順序・`Container::set_clip` Result+NULL・`Container::create` 親 `IsWindow` 検査・stub backend 同 signature・`#[cfg(all(test, windows))]` test mod |

## タスク分解（RED→GREEN）

1. **T-1 RED test（#404）**: `#[cfg(all(test, windows))] mod tests`
   に非表示 test parent HWND helper（`STATIC` class・`WS_POPUP`・
   `WS_VISIBLE` なし）と、adapter 結合 helper（`GpuiDriver::new()` →
   `create_window` → `inner.windows[id].raw_parent = Some(hwnd)` —
   module 内子孫なので private field に到達可能）。
   `set_clip(None)` で `GetWindowRgn` が region を返さないこと、
   `set_geometry` 後に `GetRgnBox` が `clip_local(C, G2)` である
   ことを確認する test を先に書き、現行コードで失敗（RED）する
   ことを確認する。
2. **T-2 GREEN（#404）**: `Container::set_clip` を
   `Result<(), SlotError>` 化し `None` で `SetWindowRgn(NULL)` を
   発行・`Some` の `CreateRectRgn`/`SetWindowRgn` 失敗を `Backend`
   で返す。`GpuiSurfaceAdapter::set_clip` を「OS 成功後に
   `binding.clip` 記録」順へ、`set_geometry` に clip 再投影を
   追加、`lease_parent` re-apply 失敗を rollback + `Err` にする。
   stub backend を同 signature に追従。
3. **T-3 RED test（#405）**: declare/lease → test parent を
   `DestroyWindow` → `crate::gpui_driver::retire(&driver.inner, w)`
   → `drain_events`/`state()`/各 op/`redeem`/`container_token`/
   `bindings` の収束、declared-only slot、repeated destroy、
   release-after-destroy、index reuse、multi-window、multi-adapter。
   現行コードで失敗（RED）を確認。
4. **T-4 GREEN（#405）**: `Binding.id` 追加・`binding_dead`/
   `sweep_dead` 実装・全 `&mut` op + `drain_events` 入口へ
   `sweep_dead` 呼出・`state`/`container_token`/`validate_lease`/
   `redeem` に実効死亡反映・`Container::create` に `IsWindow(parent)`
   検査追加。
5. **T-5 検証**: `mise run fmt-check`/`check`/`lint`/`test`/
   `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`（jobs 1-2）。最終
   `origin/main` 取込・`Cargo.lock` 差分なし確認。

## リスクとロールバック

- **test HWND が OS session を要求**: `cargo test` は実 Windows
  デスクトップ session で実行（本 worktree の canonical
  `test-ui-gpui` も同条件）。非表示 window のみ生成し focus/
  taskbar を触らない。CI では `gpui` feature lane が Windows 実機
  である点は既存運用と同じ。
- **`retire` 直接呼出 test**: module 内 private fn のため外部からは
  不可 — test は `gpui_driver::surface` 子孫 `mod tests` に置き
  `crate::gpui_driver::retire` を呼ぶ。driver 公開 API は増やさない。
- **RF387 merge 競合**: `surface.rs` の同一ファイル改修 —
  `Binding` への `id` 追加と `bound`/`spec` 追加は別領域だが
  共有ファイルであることを status で root へ報告する。
- **ロールバック**: 変更は 1 クレート 1 ファイル + 本 docs のみ。
  `git revert` で独立に戻せる単位で commit を分ける。

## 検証マトリクス

| AC | 検証 | 入口 |
| --- | --- | --- |
| AC-404-1 | `clip_release_clears_region` | `test-ui-gpui` |
| AC-404-2 | `geometry_move_reprojects_clip` | `test-ui-gpui` |
| AC-404-3 | `set_clip` ledger-order code review + `lease_parent` rollback test | `test-ui-gpui` |
| AC-405-1 | `owner_destroy_releases_slot*` | `test-ui-gpui` |
| AC-405-2 | 上記内の各 op typed error assert | `test-ui-gpui` |
| AC-405-3 | `state`/`container_token`/`redeem`/`validate_lease` assert | `test-ui-gpui` |
| AC-405-4 | index reuse/bindings 収束 assert | `test-ui-gpui` |
| AC-X-2 | `bound`-equivalent standalone `new(&driver)` 経路で全 test（canonical は同一 `inner` を観測 — RF387 merge 時に自然継承） | `test-ui-gpui` |
| AC-X-3 | 正規 mise 入口 | fmt/lint/test 系 |
| AC-X-4 | multi-window/multi-adapter/repeated/reuse tests | `test-ui-gpui` |

非 Windows（macOS/Linux）: stub backend により該当 test は
`#[cfg(windows)]` で未実行 — `Unsupported` 記録として扱い、
他 OS 実機未検証を Pass に転用しない。
