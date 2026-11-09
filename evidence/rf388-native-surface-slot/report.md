# RF388 実施 report — NativeSurfaceSlot 契約 Fake fixture・OS/format capability matrix・PoC 再実行

- Issue: https://github.com/scarecrow173/ruviform/issues/388
- Base: `origin/main` = `df3237e2`（RF386 merge）。作業 HEAD は `origin/main` を含むことを確認済み（merge 不要）。
- 作業 branch: `scarecrow173/rf388-native-slot`（Orca worktree の branch）。PR head は `feature/388-native-surface-slot` へ push（後述の差分参照）。
- 環境: Windows 11 x64 / Git Bash / RTX 4070 SUPER / gpui 0.2.2 / 本機に Vinyl.vst3 (iZotope, 1.12.0) を インストール済み。

## 本 increment の内容

既存資産（先行 PR #399/#402/#404-#406 で merge 済み）の上に、以下を追加した。

| 区分 | 内容 |
| --- | --- |
| Fake NativeSlot Driver | `crates/ruviform-ui-test/src/fake_native_slot.rs` — `NativeSurfaceSlot` port を `SurfaceSlotRegistry` 上に実装し、attach された child を記録として表す。attach/hide/resize/clip/z/focus/attach fault/stale/forged/teardown/busy を GPUI adapter と同じ規約で扱う（id 解決 → capability ゲート → attach 失敗時の lease 解放） |
| 契約 fixture | `crates/ruviform-ui-test/tests/native_slot_contract.rs` — 13 ケース（NSC-01〜13）。うち NSC-12/13 は実 `DockWorkspace` の 2-phase move（begin/abort/commit）を slot の `mark_busy`/`clear_busy` と結線して rollback を検証 |
| task | `mise.toml` — `test-ui-native-slot`（`cargo test -p ruviform-ui-test --test native_slot_contract`） |
| 仕様 | `docs/test-specs/issue-388-389-native-dock-contract.md` に NSC-01〜13 を追記 |

既存の契約（`crates/ruviform-ui/src/native_slot.rs`、GPUI adapter `crates/ruviform-ui-gpui/src/gpui_driver/surface.rs`、PoC `crates/ruviform-poc-native-dock`）は変更していない。

## 検証（実行コマンド・exit・結果）

本 increment の working tree で実行（commit 内容と同一）。

| command | exit | 結果 |
| --- | --- | --- |
| `mise run test-ui-native-slot` | 0 | 13 passed / 0 failed（rollback 2 件含む） |
| `mise run fmt-check` | 0 | 整形差分なし（`mise run fmt` 後に再確認） |
| `mise run check` | 0 | `cargo check --workspace --all-targets` |
| `mise run lint` | 0 | `clippy --workspace --all-targets -- -D warnings` |
| `mise run test` | 0 | workspace 全 suite ok（`test result: ok` 175 block、FAILED 0） |
| `mise run check-ui-driver-boundary` | 0 | 依存グラフ・public signature 漏出なし |
| `mise run test-ui-driver-contract` | 0 | RF390 driver contract fixture |
| `mise run test-ui-component-view` | 0 | RF387 component/view fixture |
| `mise run check-ui-headless` | 0 | `tsc --noEmit` |
| `mise run test-ui-headless` | 0 | 67 pass / 0 fail |
| `mise run check-ui-gpui` | 0 | gpui feature check |
| `mise run lint-ui-gpui` | 0 | gpui feature clippy `-D warnings` |
| `mise run test-ui-gpui` | 0 | 34 passed / 0 failed |
| `mise run poc-native-dock` | 0 | 実 GPUI window。ログ: `poc-native-dock-probe-win11.txt` |
| `RVF_POC_VST3_PATH=… mise run poc-vst3-editor` | 0 | 実外部 VST3（Vinyl）。ログ: `poc-vst3-editor-vinyl-win11.txt` |

`test-ui-test-msrv`（MSRV 1.89 clippy）は本 increment で未実行（理由: 1.89 toolchain の有無を未確認。NotRun）。

OS handle 非露出の確認: `native_slot.rs`・`fake_native_slot.rs`・`native_slot_contract.rs` を `HWND|NSView|gpui|wry|tao|raw_window|X11|XID|windows_sys` で検索し、ヒットは「含まない」と書いた module doc のみ。`check-ui-driver-boundary` も exit 0。

## PoC 実機結果（Windows 11、本 increment の HEAD）

probe（`poc-native-dock`、in-process 決定論 probe plugin）と実外部 VST3（`poc-vst3-editor`、Vinyl 1.12.0）で同一の観測列を得た。

| gate | probe | Vinyl VST3 | 判定 |
| --- | --- | --- | --- |
| module load / instantiate / initialize | — | ok | Pass |
| declare_slot → lease_parent（container HWND 生成） | ok | ok | Pass |
| open（attached → editor HWND 生成、`parenting_ok`） | ok | ok | Pass |
| set_geometry（editor rect 追従） | (360,192,400,300)→(400,242,400,300) | (360,192,553,400)→(400,242,553,400) | Pass |
| hide / show（child_visible） | false / true | false / true | Pass |
| request_focus | container HWND のみ | container HWND のみ | Pass（container 限定。editor への key/IME は NotRun） |
| close → session Closed・child 破棄 | ok | ok | Pass |
| release_lease / release_slot・event 列 | ok | ok | Pass |

いずれも GPUI の `Dock` layout 配下ではない。L0 の container HWND を GPUI top-level 内に置いた確認であり、**Dock PASS ではない**（下記 matrix 参照）。

## OS/format capability matrix（全行）

列の意味: `embedding API` = 規格上の埋込 API、`parent` = 親 window の所有者、`process` = プロセス境界、`resize` = リサイズ交渉、`popup/IME`、`rendering & z-order`、`observed` = 本リポジトリで実測した結果、`verdict` = 判定（Pass/Fail/Unsupported/NotRun）、`limitation` = 制限。
「現行 build」は `BACKEND_AVAILABLE = cfg!(target_os="windows")`（surface.rs）と各 plugin bridge の `open_gui` 実装状況に基づく。

| OS / format | L0/L1 | embedding API | parent | process | resize | popup/IME | rendering & z-order | observed | verdict | limitation |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Windows / VST3 | L0 | `IPlugView::attached(HWND)`（`isPlatformTypeSupported`→`setFrame`） | host の per-slot container HWND（GPUI top-level の子） | 同一プロセス | `set_geometry`→editor rect 追従 Pass。plugin 主導 `resizeView` は `kResultFalse` で拒否（defined refusal） | IME: 現行 caps で `Unsupported`。popup 未試験 | container z-order のみ。GPUI swapchain 遮蔽は未検証 | Vinyl 1.12.0 で attach/geometry/hide/show/focus(container)/close/release 全 Pass（本 round） | **attach Pass / Dock PASS NotRun** | GPUI dock layout 未接続。editor への key/IME・popup・swapchain 遮蔽は NotRun |
| Windows / VST3 | L1 | 越境 `WS_CHILD`/`SetParent` | host（別 proc） | 別プロセス | — | 入力キュー結合で hang 波及 | — | L1 GUI は `L1EditorOps` で全 op `Unsupported`（wire 不変） | **Unsupported** | issue-115: 越境 SetParent は入力キュー暗黙結合で脆弱。L1 GUI 未実装 |
| Windows / CLAP | L0 | `clap_window.win32` HWND | host container 子 | 同一プロセス | — | — | — | `ruviform-plugin-clap` に `open_gui` override なし（既定 `Unsupported`） | **Unsupported（現行 build）** | CLAP bridge の open_gui 未実装。実 CLAP binary 未入手 |
| Windows / CLAP | L1 | 越境 `WS_CHILD` | — | 別プロセス | — | — | — | L1 GUI Unsupported | **Unsupported** | 同上 |
| macOS / VST3 | L0 | `IPlugView::attached(NSView*)` | host view 子 | 同一プロセス | — | responder chain 未検証 | Metal/layer 未検証 | 実機なし・NSView backend 未実装（`BACKEND_AVAILABLE`=false） | **NotRun（未試験）** | macOS 実機なし。backend 未実装のため現行 build は Unsupported |
| macOS / VST3 | L1 | 越境 NSView reparent の公認 API なし | — | 別プロセス | — | — | `CAContext`/`CALayerHost` は合成専用（入力転送なし） | issue-115 で不可確定 | **Unsupported** | 公認越境 view 機構なし |
| macOS / CLAP | L0 | `clap_window.cocoa` NSView | host view 子 | 同一プロセス | — | — | — | CLAP bridge open_gui 未実装 | **NotRun（未試験）** | 実機なし・CLAP open_gui 未実装 |
| macOS / CLAP | L1 | 越境 NSView 不可 | — | 別プロセス | — | — | — | — | **Unsupported** | 同上 |
| macOS / AU | L0 | AU view（AUv2 `NSView` / AUv3 XPC） | host view 子 | 同一プロセス or AUv3 別 proc | — | AU 独自 | AU 独自 | AU host adapter はリポジトリに未実装 | **NotRun（未試験）** | AU adapter 未実装・実機なし |
| macOS / AU | L1 | 越境 NSView 不可 + AUv3 XPC view | — | 別プロセス | — | — | — | — | **Unsupported** | 公認越境 view 機構なし |
| Linux X11 / VST3 | L0 | `IPlugView` `"X11EmbedWindowID"` → XEmbed | host XID 親 | 同一プロセス | — | X 入力/ICCCM 未検証 | X z-order 未検証 | 実機なし・X11 backend 未実装 | **NotRun（未試験）** | X11 session 実機なし。backend 未実装のため現行 build は Unsupported |
| Linux X11 / VST3 | L1 | XEmbed 越境（GtkSocket/GtkPlug、GTK3 deprecated） | host XID（別 proc） | 別プロセス | — | — | — | — | **Unsupported** | 旧式機構のみ理論上成立。GTK4 除去・GPUI 経路なし |
| Linux X11 / CLAP | L0 | `clap_window.x11` XID → XEmbed | host XID 親 | 同一プロセス | — | — | — | CLAP open_gui 未実装 | **NotRun（未試験）** | 実機なし |
| Linux X11 / CLAP | L1 | XEmbed 越境 | 同上 | 別プロセス | — | — | — | — | **Unsupported** | 同上 |
| Linux XWayland / VST3 | L0 | XEmbed over XWayland | host XID 親 | 同一プロセス | — | XWayland 経由の IME/clip 差 | compositor 依存 | 実機なし | **NotRun（未試験）** | XWayland 実機なし |
| Linux XWayland / VST3 | L1 | XEmbed 越境 over XWayland | 同上 | 別プロセス | — | — | — | — | **Unsupported** | 同上 |
| Linux XWayland / CLAP | L0 | `clap_window.x11` over XWayland | 同上 | 同一プロセス | — | — | — | CLAP open_gui 未実装 | **NotRun（未試験）** | 同上 |
| Linux XWayland / CLAP | L1 | 同上 | 同上 | 別プロセス | — | — | — | — | **Unsupported** | 同上 |
| Linux Wayland Native / VST3 | 任意 | VST3 GUI 規格に wayland platform type なし | — | — | — | — | `xdg-foreign` は toplevel hint のみ | 規格上 embed 機構なし | **Unsupported** | 規格未対応 |
| Linux Wayland Native / CLAP | 任意 | `clap_window` に wayland 型なし | — | — | — | — | 同上 | 規格上 embed 機構なし | **Unsupported** | 規格未対応 |

補足:

- 「NotRun（未試験）」は実機・実 binary・backend のいずれかが無く、試験を行っていない行。PASS に繰り上げていない。
- 「Unsupported（現行 build）」は規格上は可能性があっても、この build に実装がない行。silent no-op ではなく `SlotError::Unsupported` / `EditorOpError::Unsupported` で返す。
- Windows VST3 L0 の `attach Pass` は GPUI top-level 内 container への実 attach であり、Dock layout 管理下の実証ではない。

## 受入（Issue #388 DoD）対応表

| 項目 | 判定 | 根拠 / 未達理由 |
| --- | --- | --- |
| Driver-neutral `NativeSurfaceSlot` 契約（型・port・registry）が文書化・実装済み | **Pass（headless + fake）** | `native_slot.rs`、`test-ui-native-slot`（13）、`native_slot` 既存 11 |
| OS handle（HWND/NSView/X11/GPUI/Graphics）が公開 API に露出していない | **Pass** | `check-ui-driver-boundary` exit 0、grep 検査（本 report の検証節） |
| Fake NativeSlot 契約 fixture（attach/move/hide/resize/error/stale/teardown/rollback） | **Pass** | NSC-01〜13（fake は contract 適合証拠のみ。実 Dock PASS ではない） |
| 2-phase Dock move の failure rollback（元 View/lease 保持） | **Pass（contract・DockWorkspace モデル）** | NSC-12/13。実 window 上の rollback は未検証（下記未達） |
| OS/format capability matrix 全行記録（根拠付き） | **Pass（記録）** | 上表 20 行。Pass は Windows VST3 L0 attach のみ、他は NotRun/Unsupported |
| 実 plugin の GPUI main dock 内 Native Dock PASS | **未達** | GPUI dock layout（tab/clip/dock 管理）に接続しておらず、observed は container 上の attach まで |
| Windows CLAP の実 editor | **未達（NotRun）** | CLAP bridge に open_gui 未実装・実 CLAP binary なし |
| plugin 主導 resize（`resizeView`）の受理 | **未達（Unsupported）** | 現行は `kResultFalse` の拒否のみ |
| keyboard/IME/popup/accessibility 入力 routing | **未達（NotRun）** | container focus まで。editor への配送は未実装・未検証 |
| 2 Window / DPI / monitor 切替・float/re-dock 実機 | **未達（NotRun）** | 未実施 |
| Window/Plugin host crash/hang の隔離（RT 非干渉） | **未達（NotRun）** | L1 未実装。L0 では隔離の対象外 |
| GPUI swapchain 遮蔽 | **未達（NotRun）** | 未検証 |
| macOS VST3/CLAP/AU の L0 | **未達（NotRun）** | macOS 実機・backend なし |
| Linux X11/XWayland・Wayland Native の行 | **未達（NotRun/Unsupported）** | Linux 実機なし。Wayland は規格上 Unsupported |
| L1 隔離（#389 と共同検証） | **未達（Unsupported）** | L1 GUI は現行 Unsupported。#389 の別 owner と共同検証が必要 |
| 採否 Gate（#262 ADR で採用/不採用と OS 境界を決定） | **未達** | 本 PR は判定を下さない。採否は #262 ADR で決定 |
| `mise run fmt-check`/`check`/`lint`/`test` green | **Pass** | 上表 |

## 実施していない検証（NotRun）

- macOS / Linux / Wayland の全行（実機なし）。
- `test-ui-test-msrv`（MSRV 1.89 clippy）。
- 2 Window・DPI/monitor・float/re-dock・hide/tab 切替の連続操作・crash/hang 注入・GPU/RAM/遅延の計測。
- 実 CLAP plugin、VST3 以外の Windows 実 binary。
- editor への keyboard/IME/popup/accessibility 入力。

## Review protocol

独立サブエージェント（read-only、`subagent_explore`、本 worker の実装者とは別コンテキスト）が `fake_native_slot.rs`・`native_slot_contract.rs`・test-spec の NSC 表・本 report を照合した。

- P1（正しさ）: 指摘なし。error 分類（Released/StaleGeneration/UnknownSlot）、capability ゲート、attach fault 時の lease 漏れなしを確認。
- P2（doc 正確性）: 指摘なし。NSC-01〜13 と test 名・assert の対応を確認。Pass 判定は fake/Windows L0 attach に限定され、NotRun/Unsupported 行の Pass 繰り上げなし。
- P3（nit、反映済み）: `capabilities` が backend 交差を行わない点を module doc に明記、attach fault の発火条件（次の成功 mint）を doc に明記。
- 判定: NO BLOCKERS。

## 追記: main `3f6e6a8b` での再検証（2026-11-09・dispatch `task_08bbbaae8be2`）

本スライス（driver-neutral `NativeSurfaceSlot` 型契約・依存方向・2-phase move/rollback・headless fixture）の成果物は先行 PR で merge 済みであることを確認し、現行 `origin/main`（`3f6e6a8b`、branch `feature/388-nativesurface-contract`、コード差分なし）で受入条件を再検証した。

| 成果物 | merge PR | 現行 main での状態 |
| --- | --- | --- |
| driver-neutral `NativeSurfaceSlot` 契約（`crates/ruviform-ui/src/native_slot.rs`）と `SurfaceSlotRegistry` | #399 | 変更なし・存続 |
| Fake NativeSlot Driver（`fake_native_slot.rs`）+ 契約 fixture NSC-01〜13（`native_slot_contract.rs`）+ `mise run test-ui-native-slot` | #424 | 変更なし・存続 |
| GPUI driver 側 windowing（ancestor clip・focus/key routing・2-phase re-dock、`render.rs`/`poc/dock_windowing.rs`） | #501 | 契約面（`native_slot.rs`・fake・contract test）は不変 |

再検証コマンドと結果:

| command | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | 整形差分なし |
| `mise run check` | 0 | `cargo check --workspace --all-targets` |
| `mise run lint` | 0 | `clippy --workspace --all-targets -- -D warnings` |
| `mise run test` | 0 | workspace 全 suite ok（`test result: ok` 192 block、FAILED 0） |
| `mise run test-rt` | 0 | RT 契約試験 green |
| `mise run test-ui-native-slot` | 0 | 13 passed / 0 failed（NSC-12/13 の 2-phase rollback 含む） |
| `mise run test-ui-test-msrv` | 0 | MSRV 1.89 で `ruviform-ui-test` 全 target green（`native_slot_contract` 13 件を含む）— 本 report で NotRun だった同 lane を今回実行し Pass |
| `mise run check-ui-driver-boundary` | 0 | dep-graph / signature / allowlist pass（OS/GPUI private handle の公開 API 非露出を再確認） |

受入対応の再確認: 型契約・依存方向・2-phase move/rollback（`DockWorkspace` begin/commit/abort と slot busy の結線、NSC-12/13）・headless fixture は全て現行 main で成立。実 OS Dock Pass/Fail・実機 fixture・#389 共同検証・ADR 採用条件の実測部分は本スライス範囲外であり、上記 matrix の NotRun/Unsupported 記録を維持する（Pass への繰り上げなし）。コード差分がないため本追記のみの docs commit とする。
