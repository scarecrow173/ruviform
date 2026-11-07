---
id: "TSPEC-0f0PqMFNljVvAz5wOtJEQu"
type: "test-spec"
status: "approved"
title: "Issue #388/#389 Native Dock 契約テスト仕様（slot/lease/session の headless 検証 + L0-L1 写像）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/tests/native_slot.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/editor_session.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/native_slot_contract.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/fake_native_slot.rs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-388-389-native-dock-contract.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/386"
  defers: []
  deferred-by: []
metadata:
  issues: [388, 389]
---

# Issue #388/#389 Native Dock 契約テスト仕様

## 目的

`SurfaceSlotRegistry`/`SlotLease`/`PluginEditorSession`/
`PluginEditorSessions`/`EditorSessionOps`（L0 `InProcessEditorOps`・
L1 `L1EditorOps`）の **driver-neutral 契約**を、実 GPUI fixture・
実プラグイン・実 OS を用いず headless モデルで検証する。本書の
green は「契約・状態機械・lease/session 安全性」の証拠であり、
**実 Native Dock PASS ではない**。

## 対象外（実機 PoC 段へ defer — 本 dispatch では実行不可）

- GPUI Main Dock 内での実ネイティブ Editor 動作・clip/tab/resize/
  focus/入力/遮蔽（実 dock layout 未実装 → Dock PASS 行は継続
  `NotRun`。window 内 region での実 attach/geometry/visibility/focus/
  release は第二 increment で実機観測済み —
  `evidence/rf388-389-native-dock/`）。
- CLAP/VST3 `open_gui` adapter 経由の実 binary Editor open（実 binary
  未入手 → `NotRun`。in-process probe による同一 OS 経路は第二
  increment で実機観測済み）。
- macOS/Linux 実機・実 binary・DPI/2Window/IME/popup/a11y。
- L1 wire 越境 GUI（現行 `Unsupported` — pin テストは既存 e2e が継続）。

## 既存テストとの関係（非回帰）

- `l1/mod.rs` の GUI `Unsupported` short-circuit を pin する e2e
  （`gui_defined_rejection` 系）: 変更せず green を維持 — L1 は
  依然 `Unsupported`。
- `dock.rs`/`window.rs`/`render.rs`/`boundary.rs`/`wire.rs` 既存
  テスト: 変更対象外のため不変。

## `native_slot` 契約ケース（`crates/ruviform-ui/tests/native_slot.rs`）

| TestID | ケース | 期待 |
| --- | --- | --- |
| NS-01 | `declare_slot` → `slot()` | `SlotState::Declared`・spec 保持 |
| NS-02 | `lease_parent(slot)` → `validate_lease` | `Ok(LeaseInfo{slot,generation})` |
| NS-03 | 同一 slot への 2 度目 `lease_parent` | `Err(LeaseBusy)`（live lease 重複拒否） |
| NS-04 | `release_lease` → 再 `lease_parent` | 成功・`seq` 更新で旧 lease は `LeaseRevoked` |
| NS-05 | `validate_lease` に未知 seq/未知 index | `Err(InvalidLease)`/`UnknownSlot` |
| NS-06 | `release_slot` 後に `validate_lease` | `Err(StaleGeneration)` or `LeaseRevoked` |
| NS-07 | `release_slot` → 同 index 再 `declare` | generation++・旧 id/lease 無効 |
| NS-08 | `mark_busy`（Dock move 中）→ `lease_parent` | `Err(SlotBusy)`・`clear_busy` で解除 |
| NS-09 | `release_lease`/`release_slot` 済み slot への `set_geometry` | `Err(UnknownSlot)`/`Released` |
| NS-10 | `drain_events` | `LeaseRevoked`/`GeometryChanged`/`VisibilityChanged`/`Destroyed` が順序付き・`Destroyed` は終端 |
| NS-11 | `capabilities` 未対応旗艦 | `SlotError::Unsupported`（fake 成功を PASS にしない） |
| NS-12 | `SlotLease::encode`/`decode` 往復 | 同一値復元・他人 index/seq 不一致は decode 側で無効化可能 |

## `editor_session` 契約ケース（`crates/ruviform-plugin-host/tests/editor_session.rs`）

fake `EditorSessionOps`（応答を制御）+ 実 `InProcessEditorOps`
（stub `ErasedPlugin` で `open_gui`=`Ok`/`Unsupported` を切替）で検証。

| TestID | ケース | 期待 |
| --- | --- | --- |
| ES-01 | `begin_session` → `open(Docked)` → `Attached{Docked}` | `EditorSessionEvent::Attached`・id mint |
| ES-02 | Attached → `set_visibility(false)` → `Hidden` → `(true)` → `Attached` | 遷移イベント列 |
| ES-03 | `begin_detach` → `reattach(lease2)` → `Attached` | `Detaching`→`Attached`・lease 差替 |
| ES-04 | `close` → `Closing` → `Closed` | `Closed` 終端・`event::Closed` |
| ES-05 | 既 `Attached`/`Creating` 中の `open` | `EditorSessionError::InvalidState`（二重 open 拒否） |
| ES-06 | `Closed` の `close`/`set_size` | `InvalidState`（二重 close/非 live 拒否） |
| ES-07 | stale request（完了 req ≠ 現行 req）の `complete` | `StaleRequest` で無視・state 非変化 |
| ES-08 | `notify_terminated`（instance destroy/host crash/window lost） | `Terminated{cause}` 1 回・`Closed` 収束 |
| ES-09 | `notify_terminated` 2 回目・終端後 callback | 重複 event なし・無視 |
| ES-10 | `Failed{cause}` → `recover` → `Recovering` → `Attached` or `Failed` | recover 経路・再帰的 fail も収束 |
| ES-11 | `ops.open`=`Err(Unsupported)` | `Failed{Unsupported}` + event・`Closed` 到達可 |
| ES-12 | `ops.open`=`Err(LeaseRejected)` | `Failed{LeaseInvalid}`（lease 検証拒否が session へ写像） |
| ES-13 | `ops` timeout（fake clock） | `Failed{Timeout}`・request 世代一致の場合のみ |
| ES-14 | `PluginEditorSessions` 同 instance 2 度目 `begin_session` | `Err(SessionBusy)`（≤1 live） |
| ES-15 | `notify_terminated(instance)` で全 live session 収束 | 全 `Terminated`・`Closed`・DSP 独立 |
| ES-16 | `InProcessEditorOps` + stub plugin `open_gui`=`Ok` | `Attached`・`GuiParentToken::new` が内部経由 |
| ES-17 | `InProcessEditorOps` + stub plugin `open_gui`=`Err(Unsupported)` | `Failed{Unsupported}`・`HostError`→`EditorOpError` 写像 |
| ES-18 | `L1EditorOps` 全 ops | 全て `EditorOpError::Unsupported`・wire 不変・L0 退化なし |

## Fake NativeSlot driver 契約ケース（`crates/ruviform-ui-test/tests/native_slot_contract.rs`）

`FakeNativeSlot`（`NativeSurfaceSlot` port を `SurfaceSlotRegistry` 上に
実装し、attach された child を記録だけで表す fake）で検証する。
GPUI adapter と同じ id 解決・capability ゲート・attach 失敗時の lease
解放規約に従う。**fake 成功は port 契約の適合証拠であり、実 Dock PASS
ではない**（child は window ではなく記録）。`mise run test-ui-native-slot`。

| TestID | ケース | 期待 |
| --- | --- | --- |
| NSC-01 | attach 前の `set_clip`/`set_zorder` → `lease_parent` | child が clip・z・geometry・visibility を継承して attach |
| NSC-02 | attach 済み child に `set_geometry`（move）・`set_visibility(Hidden)` | child へ反映・lease は維持（hide は detach ではない） |
| NSC-03 | NaN 幅・負の高さの `set_geometry` | `InvalidValue`・child 不変 |
| NSC-04 | `capabilities` 未対応の `set_clip`/`set_zorder`/`set_visibility`/`lease_parent` | `Unsupported`（silent no-op・fake 成功にしない）・slot は `Declared` |
| NSC-05 | `request_focus`: child 無し → `Backend`、attach 後 → 成功・別 slot へ移動で旧 focus 解除 | focus は常に高々 1 child |
| NSC-06 | attach fault（`arm_attach_fault`）→ `lease_parent` | `Backend`・lease を解放し `Declared`・child 無し・次の attach は成功 |
| NSC-07 | `release_slot` → 同 index 再 `declare_slot` | 解放直後の旧 id は `Released`、再利用後は `StaleGeneration`・旧 lease は `LeaseRevoked` |
| NSC-08 | 偽造 lease（未発行 seq）・live lease への二重 `lease_parent` | `InvalidLease` / `LeaseBusy`・live lease は維持 |
| NSC-09 | `release_lease` → child 除去・再 attach | child 消滅・新 lease は旧と別値 |
| NSC-10 | `release_slot`（teardown）→ drain | `[LeaseRevoked, Destroyed]` の順・child 消滅・`Released` 終端 |
| NSC-11 | `mark_busy` 中の `lease_parent`（lease 解放後）→ `clear_busy` | `SlotBusy` → 解除後に成功 |
| NSC-12 | `DockWorkspace::begin_move` → `mark_busy` → `abort_move` → `clear_busy` | 元 window に view 残留・live lease 維持・lease 検証 OK（rollback） |
| NSC-13 | `begin_move` → `mark_busy` → `commit_move` → 旧 lease 解放・旧 slot 解放 → 新 window で再 declare・lease | 新 child が attach・旧 lease は `LeaseRevoked`・旧 id は `StaleGeneration` |

## 非機能ゲート

- `mise run fmt-check`/`check`/`lint`（`-D warnings`）/`test` 全 green。
- `unsafe` 不使用・公開 enum 全 `#[non_exhaustive]`・外部依存追加なし。
- `ruviform-ui`/`ruviform-plugin-host` に gpui/wry/tao/OS handle 型・
  plugin↔ui 相互依存を持ち込まない（`cargo tree`/import 監査）。

## 判定

- 上記全ケース green = **契約層 pass**（実 Dock PASS ではない）。
- 実機行は `docs/designs/issue-388-native-surface-slot.md` D6 の
  マトリクスで `NotRun`/`Blocked`/`Unsupported` を維持 — 本 dispatch
  では実値 PoC を発生させない。
