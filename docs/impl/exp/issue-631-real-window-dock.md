# Issue #631 real-window native-slot dock lane — 実装実験記録（2026-11-15）

`docs/tasks/issue-631-real-window-dock.md` の second slice
（real-window native-slot dock lane）実装で試行・観測した事項の記録。
実装結果の正本は
[`docs/impl/ir/issue-631-real-window-dock.md`](../ir/issue-631-real-window-dock.md)、
イベントログは
[`issue-631-real-window-dock.jsonl`](issue-631-real-window-dock.jsonl)
を参照。

## 試行 1: slot ops の戻り値型（`private_interfaces` 回避）

- **目的**: `RealOsGuiTest` に declare/bind/lease/validate/release/
  state/container_token/redeem の slot ops を `UiJob` 経由で追加する。
- **観測**: `gpui_driver` module は private で `lib.rs` の `pub use`
  に新 error 型を足すには `lib.rs` 変更が必要 — 本 slice の編集範囲外。
  公開 enum を `real_window.rs` に置くと `private_interfaces` で
  `-D warnings` に抵触する。
- **結論**: slot ops は `Result<T, ScenarioError>` を返し、
  `SlotError` は `ScenarioError::Driver` に
  `native slot \`{op}\`: {error}` prefix で写像する。consumer は
  `ruviform-ui` の型（`SlotSpec`/`SurfaceSlotId`/`SlotLease`/
  `LeaseInfo`/`SlotState`）だけを扱い、GPUI 型も専用 error 型も
  境界を越えない。prefix により slot 拒否と session 失敗を
  evidence 上で区別できる。

## 試行 2: `load_plugin` の借用と move の競合（E0505）

- **観測**: `library.descriptors()` の借用を持つ `descriptor` を
  使いながら `library` を tuple に move しようとして E0505。
- **結論**: descriptor の 4 field を先に owned `String` tuple に
  展開してから `library` を move する形に修正。`PluginArtifactEvidence`
  の構築は borrow 終了後に行う。

## 試行 3: capability gating の条件

- **観測**: `supported_capabilities()` に `NativePluginDock` を追加する
  際、無条件 contains への書き換えは禁止（受入条件）。
- **結論**: `surface::BACKEND_AVAILABLE && !cfg!(feature = "gpui-test")`
  の predicate（`native_dock_backend_available()`）で gate。
  `gpui-test` feature は native capture/container を stub するため
  real container HWND を mint できず、capability を名乗れない。
  既存 test の not-contains assert をこの predicate 連動に更新した。

## 試行 4: stale/forged lease の拒否証拠

- **目的**: `real-native-slot` case で lease lifecycle の honesty を
  実測する — 正規 lease の validate → payload 偽造 → validate/redeem
  拒否 → release。
- **結論**: `ParentSlotLease` の payload は opaque `u64`。forged
  payload（`payload ^ 0xFF` 等）を `redeem_native_lease` に流すと
  canonical `SharedSurface` ledger が拒否し、`InProcessEditorOps`
  redeem も `HostError` に落ちる。拒否自体を evidence detail に
  記録し、fake success を主張しない。

## 試行 5: headless fail-closed

- **目的**: 非対話環境で lane を実行した場合に偽 Pass を出さない。
- **結論**: `RealOsGuiTest::run` の window 生成失敗を
  `EnvironmentBlocked`/`NotRun` に写像する blocked-report helper を
  用意。case verdict は `ScenarioVerdict` のまま、JSONL には
  `observed="environment_blocked"` 相当の detail を残す。
  `EnvironmentKind::RealWindow` は実走経路でのみ emit する。
