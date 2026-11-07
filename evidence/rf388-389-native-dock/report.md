# RF388/389 実施 report — NativeSurfaceSlot + PluginEditorSession + Windows 実 native attach PoC

- Issue: https://github.com/scarecrow173/ruviform/issues/388 / https://github.com/scarecrow173/ruviform/issues/389
- Branch: `feature/native-surface-plugin-editor-poc`
- Base: `49de02c` + RF386 candidate `d907bfc` merge(`62e317d`) → main `5c0f16b` merge(`86895a3`、RF386/PR394 + RF390 guard/lab + RF384/385 統合済みを取込)

## 実施内容(本 increment)

| 区分 | 内容 |
| --- | --- |
| 中立契約 | `ruviform-ui::native_slot` — `SurfaceSlotRegistry`/`SlotLease`(opaque packed token・registry-global u32 seq・u16::MAX generation tombstone)/`SlotSpec`/`SlotCapabilities`/`SlotEvent`/`SlotError`(+`Backend` 変種)。gpui/DOM/wry/OS handle 不含有 |
| session 契約 | `ruviform-plugin-host::editor_session` — `PluginEditorSession`(u32 generation)/`EditorSessionOps`/`InProcessEditorOps`/`EditorCapabilities`/`AttachMode`/`EditorOpError`(+`Display`/`Error`)。`begin_open` は concluded/terminated/live session を拒否、`end_session` は closed session のみ retire。L1 は `L1EditorOps` で明示 Unsupported |
| redeemer seam | `InProcessEditorOps::with_redeemer` — consumer 側 `LeaseRedeemer` 注入で opaque `ParentSlotLease` → `GuiParentToken` を adapter 検証経由で交換(plugin↔ui 相互依存禁止の baseline に従い結線は利用側) |
| GPUI adapter | `ruviform-ui-gpui::GpuiSurfaceAdapter`(`gpui` feature)— `NativeSurfaceSlot` 実装。`open_window` 時に `HasWindowHandle` で GPUI top-level の raw handle を捕捉し、slot ごとに Win32 child container HWND を生成。`WindowCapabilities::native_surface_slot` は backend 存在時のみ Supported(Windows のみ、他 OS は Unsupported) |
| consumer PoC | `crates/ruviform-poc-native-dock` — 非 default `gpui` feature の手動 real-window lane。in-process probe plugin の `open_gui` が redeemed token(container HWND)直下に実 `WS_CHILD` editor を生成する構成 |

## 実機実行証拠(Windows 11)

実行 tuple:

| 項目 | 値 |
| --- | --- |
| TestID | rf388-389-poc-attach-win11 |
| OS | Windows 11 x64(Git Bash) |
| GPU | NVIDIA GeForce RTX 4070 SUPER / driver 32.0.16.1088 |
| GPUI revision | gpui 0.2.2(crates.io)/ blade-graphics 0.7.1 |
| format | in-process probe(format binary は NotRun) |
| plugin binary/version | `org.ruviform.poc.native-editor-probe` 0.1.0(in-process) |
| process boundary | L0 in-process |
| parent | GPUI top-level HWND → per-slot Win32 container HWND → plugin editor HWND(`GetParent` で実親確認) |
| command | `cargo run -p ruviform-poc-native-dock --features gpui` → 追加で `./target/debug/ruviform-poc-native-dock.exe` 直接実行×2(再現確認) |
| exit | 0 |
| 証拠 path | `evidence/rf388-389-native-dock/poc-run-win11.log` |

観測された実効果(`poc-run-win11.log` より):

- `native_surface_slot: Supported` — backend 存在時のみ Supported を確認
- `declare_slot`/`lease_parent` → container HWND 実生成(例 `0x1341a72`)
- `open` → `LeaseRedeemer` → `GuiParentToken` → probe `open_gui` → 実 `WS_CHILD` editor HWND 生成、`GetParent(child) == container` で **実親子関係確認**
- `set_geometry` → editor rect が (360,192)→(400,242) に実追従(logical 40/40→80/90、scale 1.0 で +40/+50)
- `set_visibility` Hidden/Visible → `IsWindowVisible` が false/true に実追従
- `request_focus` → `GetFocus()` が container HWND を返却(実 focus)
- session `close` → `close_gui` → editor HWND 破棄(`IsWindow` false)
- `release_lease` → container HWND 破棄、`release_slot` → slot 破棄
- slot event 列: Declared→Leased→GeometryChanged→VisibilityChanged×2→LeaseRevoked→Destroyed(確定的)
- 直接実行 2 回とも同一列を再現

## 検証(実 command/exit)

merge `86895a3`(main `5c0f16b` 取込後)の最終 head で実施:

| command | 結果 |
| --- | --- |
| `cargo fmt --all -- --check`(mise exec) | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0(headless lane — gpui feature off で全 workspace green = feature 隔離確認) |
| `cargo test --workspace` | exit 0(166 suite 全 ok / 0 failed。`editor_session` 22/22、`native_slot` 11/11、RF390 `driver_contract` 5/5 含む) |
| `cargo check -p ruviform-ui-gpui --features gpui --all-targets` | exit 0 |
| `cargo clippy -p ruviform-ui-gpui --features gpui --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p ruviform-poc-native-dock --features gpui --all-targets -- -D warnings` | exit 0 |
| `mise run check-ui-driver-boundary`(RF390 guard) | exit 0 — dep-graph/signature/allowlist 全 pass。`ruviform-ui-gpui` の allowedDeps に `raw-window-handle`(gpui feature optional)を追加、`ruviform-poc-native-dock` に `composition-root` grant を付与 |
| `mise run test-ui-driver-contract`(RF390 lab) | exit 0(5/5) |
| `cargo run -p ruviform-poc-native-dock --features gpui` | exit 0(merged tree で実 Window 全 stage 完走・証拠列を再現) |
| `cargo run -p ruviform-poc-native-dock`(feature off) | exit 2 — `--features gpui` 要求 message を出力(honest refusal) |

merge conflict 解決: `ruviform-ui-gpui`(lib.rs/Cargo.toml)は main 版が RF386 candidate と同一のため自側 delta(surface/raw_parent/capability)を保持。`Cargo.lock` は main 版 + 本 increment 分 13 行のみ(新規 package 2 件 + dep 記述 2 行)。doc README index は union。`evidence/rf386` report は main 側(統合後再検証版)を採用。

## 状態/受入判定

| 項目 | 判定 | 根拠 |
| --- | --- | --- |
| 中立契約 RF388 | headless Pass + Windows 実機 Pass(attach/redeem/geometry/visibility/focus/release) | 上記 log |
| session 契約 RF389 | headless Pass + Windows 実機 Pass(open/close 遷移) | 同上 |
| Windows 実 native attach(実 GPUI window 内) | **Pass** — 実 HWND 親子・実 rect/visible/focus 追従を観測 | 同上 |
| **Dock PASS** | **NotRun(未達)** — GPUI 実 dock layout(tab/clip/dock 管理 widget)が未実装。本証拠は GPUI window 内 region での実埋込みであり dock 管理下ではない | 実装未存在 |
| 実 plugin binary(VST3/CLAP/AU) | NotRun — probe は in-process。`open_gui` 経路は実 binary と同一 OS API を使用 | binary 未入手 |
| L1(out-of-process) | Unsupported(明示) — `L1EditorOps` が全 op Unsupported を返す | code |
| macOS / Linux(X11/Wayland) | Unsupported(backend 未実装)/ NotRun(未検証) | `BACKEND_AVAILABLE` = cfg!(windows) |
| float/re-dock/RT 隔離/hang | NotRun | 未実装/未測定 |

## Review protocol

- Reviewer A(契約): 11 findings → 全反映(seq wrap・capability 未適用・event 欠落・`release_lease` 欠落・Display/Error 欠落・doc drift 等)
- Reviewer B(invariant attack): 2 P1(concluded session re-open 穴・live session `end_session` retire 穴)+ 補助 findings → 全反映
- Validator(runtime subagent): 本 snapshot 上で P1-2 件とも修正済みを確認。residual 指摘は `PluginHandle.generation` u16( wire u16 互換・別 issue 候補)、`SlotLease::new` doc-hidden 公開、async detach の失敗補完、no-op busy event、`end_session` 伝搬 — 後 3 件は反映済み、u16 は wire 互換のため残留(別 issue 提案)
- Validator 2nd increment(adapter+PoC): **VALIDATED-WITH-NOTES**。証拠を毀損する欠陥なし。P2 ×4 を修正反映: (1)`redeem` が owner window 死亡後も stale container HWND を返し得た → owner-window 生存 + `IsWindow` 確認を追加し `Backend` で拒否。(2)lease payload は enumerable で doc が "unforgeable" を過剰主張 → 「ledger lookup であって認証ではない・同一 process 信用境界内のみ」と doc 修正。(3)lease 前に記録された clip/z/visibility が container 生成時に未適用 → `lease_parent` で再適用。(4)`capabilities()` が consumer 宣言値をそのまま返していた → backend 実装集合との積(`intersect_caps`)へ。P3 のうち `SetWindowRgn` 失敗時 HRGN 解放・`set_clip` 座標系 doc(window 座標)・`windows-sys` optional 化・redeemer エラー写像 Unsupported 区別も反映。残 P3(任意 z 値の HWND_TOP 丸め・stale-id エラー粒度)は PoC 許容として記録。

## 残 Gate / 後続へ渡すもの

- 実 Dock PASS には GPUI 実 dock layout が必要 — 現状は window 内 region の実埋込みまで
- 実 plugin binary(VST3/CLAP/AU)の attach は plugin binary 入手後
- macOS/Layer 系(NSView)/X11/Wayland 向け backend::Container 実装は別工程
- `PluginHandle.generation` u16 wrap → 別 issue 起票候補
- RF390 共通 fixture/FakeDriver/parity が揃い次第、本 PoC fixture を差し替え候補
- SV176 へ渡す材料: session vocabulary + `ParentSlotLease`/`GuiParentToken` 語彙は境界に確定済み
