# RF532 S1 — vendored gpui 0.2.2 public-surface 監査証拠ノート

Issue #532 / 方式 A/B/C 調査の技術入力となる、gpui 0.2.2 Windows platform 層の
upstream public-surface 監査記録。**docs-only slice。コード変更・patch/vendor
作業は一切行っていない。** 監査対象は cargo registry の vendored source
（読み取り専用）:

- path: `C:/Users/akiyoshi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-0.2.2/`
- version: `gpui 0.2.2`（`Cargo.toml` `version = "0.2.2"`、crates.io publish
  2025-10-22。ruviform 側 pin は `crates/ruviform-ui-gpui/Cargo.toml:54`
  `gpui = { version = "0.2.2", optional = true }`）
- 監査実施日: 2026-11-15 / 監査者: dispatched worker（task_2f08875deff5）

以下の行番号は全て上記 vendored source に対する実測値（先行監査の行番号を
独立に再検証済み — drift なし）。

## 監査項目 1: DComp オブジェクトグラフの列挙

### モジュール到達経路（全て private）

```
gpui.rs:31            mod platform;            // private
gpui.rs:89            pub use platform::*;     // platform.rs の pub item のみ再輸出
platform.rs:24        mod windows;             // private（cfg target_os = "windows"）
platform.rs:83        pub(crate) use windows::*;
platform/windows.rs   mod directx_renderer;    // private
                     mod directx_devices;      // private
                     mod window;               // private
                     ...（全 submodule private）
                     pub(crate) use directx_renderer::*;  // 他、全て pub(crate)
```

### 生成・保持・所有の連鎖

1. `WindowsPlatform::new`（`platform/windows/platform.rs:90` 付近〜162）:
   message-only HWND を `CreateWindowExW` で生成（115-130）、
   `DirectWriteTextSystem::new`（104）、platform window class 登録（107）。
   env var を読み `disable_direct_composition: bool` を保持（42, 138-139, 158）。
2. `generate_creation_info`（platform.rs:182-195）→ `WindowCreationInfo`
   （`pub(crate)` struct、platform.rs:818-829。`disable_direct_composition`
   フィールド 827、`directx_devices` 828）。
3. `WindowsWindow::new`（window.rs:361-）→ `CreateWindowExW` で実 window HWND
   生成（439-454）。exstyle に `WS_EX_NOREDIRECTIONBITMAP` を
   `if !disable_direct_composition` で付与（window.rs:409-411）。
4. `WindowsWindowInner::new`（window.rs:208-236）→ `WindowsWindowState::new`
   （window.rs:80-147）→ `DirectXRenderer::new(hwnd, directx_devices,
   disable_direct_composition)`（window.rs:108-109）。
5. `DirectXRenderer::new`（directx_renderer.rs:127-168）:
   - `DirectXRendererDevices::new`（99-124）: `dxgi_device` は
     `device.cast::<IDXGIDevice>()` で、`disable_direct_composition` 時は
     `None`（110-114）。
   - `DirectXResources::new`（655-697）: swapchain を分岐生成（663-672）。
   - `DirectComposition::new`（790-801）: `!disable` のときのみ
     （147-156）。`get_comp_device` → `DCompositionCreateDevice(dxgi_device)`
     （1017-1019）→ **`comp_device.CreateTargetForHwnd(hwnd, true)`（793 —
     crate 全体で唯一の出現）** → `comp_device.CreateVisual()`（794）。
   - `composition.set_swap_chain(&resources.swap_chain)`（152-154）:
     `comp_visual.SetContent(swap_chain)`（805）、
     `comp_target.SetRoot(&comp_visual)`（806）、`comp_device.Commit()`（807）。
6. 保持: `DirectXRenderer.direct_composition: Option<DirectComposition>`
   （directx_renderer.rs:46）。`DirectComposition` は private struct
   （93-97: `comp_device: IDCompositionDevice`, `comp_target:
   IDCompositionTarget`, `comp_visual: IDCompositionVisual`）。
   `DirectXRenderer` 自体は `pub(crate)`（39）だが private module 配下のため
   crate 外からは命名不能。
7. 所有/lifetime: `WindowsWindowState.renderer: DirectXRenderer`
   （window.rs:51、`pub` field だが struct は `pub(crate)`）
   → `WindowsWindowInner.state: RefCell<WindowsWindowState>`（209-236）
   → window 寿命と一致。
8. 使用: `renderer.draw(scene)`（window.rs:845）→ batch 描画 →
   `swap_chain.Present(0, DXGI_PRESENT(0))`（directx_renderer.rs:205）。
   resize は events.rs:207 `lock.renderer.resize(device_size)`。
9. device-lost: platform.rs `handle_gpu_device_lost`（1033-）が
   `WM_GPUI_GPU_DEVICE_LOST` を全 window へ broadcast → events.rs:1187-1191
   → `renderer.handle_device_lost`（directx_renderer.rs:209-284）→
   `handle_device_lost_impl`（226-284）で devices/resources/pipelines/
   **DirectComposition を全て再生成**（245-268）。再生成時の
   `disable_direct_composition` は env var ではなく
   `self.direct_composition.is_none()` から推定（227）— 起動時の mode を
   維持する一貫した設計。
10. Drop: `impl Drop for DirectXRenderer`（997-1006）が ManuallyDrop
    フィールドを手動 drop。`direct_composition` は通常 field のため
    struct drop 時に COM Release（windows crate の Drop impl）。
    `DirectXResources` Drop（1008-1014）が render_target を drop。
    COM refcount により解放順序は安全。

### `CreateTargetForHwnd` / `topmost` の全出現

- `CreateTargetForHwnd`: crate 全体で **1 箇所のみ**
  （directx_renderer.rs:793、`topmost` 引数は literal `true`）。
- `CreateVisual`: 794 の 1 箇所。
- `SetContent`/`SetRoot`/`Commit`: 805-807 のみ。
- `DCompositionCreateDevice`: `get_comp_device`（1017-1019）の 1 箇所。
- `CreateSwapChainForComposition`: 1044 の 1 箇所。
- `CreateSwapChainForHwnd`: 1073 の 1 箇所。
- `MakeWindowAssociation`: 1074 の 1 箇所（`DXGI_MWA_NO_ALT_ENTER`）。
- `IDComposition*`/`DComposition*` への参照は `directx_renderer.rs` のみ
  （`vsync.rs` はコメント内の言及のみ）。`src/platform/blade/` は
  linux/freebsd/macos cfg 限定で Windows には compile されない
  （platform.rs:16-18）。

## 監査項目 2: `GPUI_DISABLE_DIRECT_COMPOSITION` の意味論

- const 定義: `pub(crate) const DISABLE_DIRECT_COMPOSITION: &str =
  "GPUI_DISABLE_DIRECT_COMPOSITION"`（directx_renderer.rs:29）。
- 読み取り: `platform.rs:138-139` — Windows platform 層で唯一の
  `std::env::var` 読み取り。
  `is_ok_and(|value| value == "true" || value == "1")` — 受理値は
  **`"true"` と `"1"` のみ**。未設定・その他の値（`"yes"`/`"0"`/空等）は
  全て false（= DComp 有効）。
- 読み取りタイミング: `WindowsPlatform::new`（platform 初期化）で **1 回
  のみ**。window ごとの変更・runtime 切替は不可。プロセス起動前に env を
  設定する必要がある。
- 非 DComp 経路が実際に行うこと（全て `disable_direct_composition` 分岐）:
  1. window exstyle から `WS_EX_NOREDIRECTIONBITMAP` を外す
     （window.rs:409-411 が skip される）→ DWM が通常の redirection
     surface を割り当てる。
  2. `DirectXRendererDevices.dxgi_device` を作らない（110-114）。
  3. `DirectComposition` object を一切作らない（147-156）。
  4. swapchain を `create_swap_chain`（1047-1076）で生成:
     `CreateSwapChainForHwnd(device, hwnd, &desc, None, None)`（1073）+
     `MakeWindowAssociation(hwnd, DXGI_MWA_NO_ALT_ENTER)`（1074）。
     desc 差分: `Scaling: DXGI_SCALING_NONE`・`AlphaMode:
     DXGI_ALPHA_MODE_IGNORE`（1056-1071）。対して composition 経路は
     `DXGI_SCALING_STRETCH`・`DXGI_ALPHA_MODE_PREMULTIPLIED`（1027-1043）。
     共通: `DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL`・`BufferCount=3`
     （BUFFER_COUNT=3、1389）・`RENDER_TARGET_FORMAT=B8G8R8A8_UNORM`（30）。
  5. `Present`・vsync・描画 batch は両経路で同一（205、vsync.rs は
     `DwmFlush` のみ使用 — DComp 非依存）。
- 効果（コードからの推論・要実機検証）: window が通常の DWM
  redirection surface を持ち、flip-model swapchain がそれへ present
  される。child HWND は DWM の通常 z-order 合成に復帰するため、
  **plugin 画素が表示される可能性がある**。これは `topmost` literal を
  patch せずに済む **zero-code 変更の代替経路**であり、唯一の blocker は
  未説明の exit-127（監査項目 7）。

## 監査項目 3: public vs `pub(crate)` の到達可能性

### downstream（ruviform）から到達不能なもの

- `gpui::platform` module 自体が private（gpui.rs:31）。
- `platform::windows` module が private（platform.rs:24）。
- 全 Windows 実装型は `pub(crate)` 以下:
  - `Platform` trait（platform.rs:163 `pub(crate) trait Platform`）
  - `PlatformWindow` trait（platform.rs:460 `pub(crate)`、
    `HasWindowHandle + HasDisplayHandle` 継承、`get_raw_handle() ->
    windows::HWND` は 528）
  - `WindowsPlatform`・`WindowsWindow`・`WindowsWindowInner`・
    `WindowsWindowState`・`WindowCreationInfo`（platform.rs:818）
  - `DirectXRenderer`（directx_renderer.rs:39 `pub(crate)`）・
    `DirectXRendererDevices`（52 `pub(crate)`）・`DirectXDevices`
    （directx_devices.rs:44 `pub(crate)`）・`DirectXAtlas`・
    `DirectWriteTextSystem`
  - `DirectComposition`（directx_renderer.rs:93、`pub` 修飾なしの
    private struct — module が private なので実質 crate 内限定）
  - `DISABLE_DIRECT_COMPOSITION` const（29 `pub(crate)`）
- したがって **ruviform は gpui の DComp device/target/visual/swapchain/
  renderer のいずれにも公開 API 経由で触れない**。`topmost` 引数・
  swapchain 種別・window exstyle を downstream から変更する術はない。

### downstream から到達可能なもの

- **HWND**: `gpui::Window` が `raw_window_handle::HasWindowHandle` を
  実装（src/window.rs:4845-4849）→ `platform_window.window_handle()` →
  `WindowsWindow` の impl（window.rs:484-492）が
  `Win32WindowHandle::new(hwnd)` を返す。**実 HWND が取得可能**。
  ruviform は既にこれを使用中
  （`crates/ruviform-ui-gpui/src/gpui_driver/surface.rs:132-138`:
  `HasWindowHandle::window_handle(window)` →
  `RawWindowHandle::Win32(w) => w.hwnd`）。
- **`HasDisplayHandle` の注意**: `Window` の impl（window.rs:4851-4857）
  は `platform_window.display_handle()` へ転送するが、
  `WindowsWindow::display_handle` は **`unimplemented!()`**
  （window.rs:495-498）— Windows で呼ぶと panic。ruviform は未使用。
- **env var `GPUI_DISABLE_DIRECT_COMPOSITION`**: 唯一の downstream
  到達可能な合成経路スイッチ（process-wide、window 生成前に設定）。
- 公開 API に存在しないもの: DComp object・swapchain・renderer・
  per-window composition flag・window exstyle/`WS_EX_NOREDIRECTIONBITMAP`
  の制御・`CreateTargetForHwnd` の `topmost` 変更手段。

## 監査項目 4: Option-A（`[patch.crates-io]`/fork で `topmost` 変更）のパッチ面積

- 変更対象 literal: **`directx_renderer.rs:793` の `true` 1 語のみ**
  （`CreateTargetForHwnd(hwnd, true)` → `false`）。
- `DirectComposition::new` の呼び出し側は 2 箇所（150 初回生成、265
  device-lost 復旧）だが、literal は `DirectComposition::new` 内部に
  あるため **両経路に自動的に適用される**。シグネチャ変更不要。
- ただし `[patch.crates-io]` は fork/vendor を必要とし、upstream の
  新しい publishable 版が存在しない（監査項目 6）ため patch base は
  0.2.2 source のみ。patch/vendor 作業自体は本 slice 対象外。
- 意味論上の注意（実機検証待ち）: `topmost=false` は GPUI visual tree
  全体を child HWND の**背面**に合成する。plugin child HWND が GPUI UI
  の上に描かれる（dock 用途には望ましい方向）のはずだが、(a) 全 child
  window にグローバルに効く、(b) `WS_EX_NOREDIRECTIONBITMAP` window で
  non-topmost target が正しく表示されるかは MSDN 意味論の確認 +
  fixture 実測が必要。

## 監査項目 5: Option-B（ruviform private 合成）の要件

前提: gpui の DComp/DXGI object は全て private で再利用不能
（監査項目 3）。公開面で使えるのは HWND（`HasWindowHandle` 経由）と
windows-sys の既存依存のみ。

- ruviform-ui-gpui の既存 Windows 依存: `windows-sys 0.61`（optional、
  `gpui` feature 時）。features は `Win32_Foundation`/
  `Win32_Graphics_Gdi`/`Win32_System_LibraryLoader`/
  `Win32_UI_Input_KeyboardAndMouse`/`Win32_UI_WindowsAndMessaging`
  （`crates/ruviform-ui-gpui/Cargo.toml:76-86`）。**DirectComposition/
  Dxgi/Direct3D11 の feature は未導入**。
- private 経路の候補（いずれも未実装・実機検証待ち）:
  - **B-env（zero code）**: `GPUI_DISABLE_DIRECT_COMPOSITION=1`。
    patch 不要だが exit-127 未解消（監査項目 7）。
  - **B-region**: GPUI hwnd への `SetWindowRgn` で dock 領域を除外し、
    DComp target の画素をその領域で clip → child HWND が透けて見える
    可能性。window region が DComp content にも適用されるかは実機検証
    が必要。ruviform は既に `SetWindowRgn`/`MoveWindow` 等の Win32
    surface 機構を保有（surface.rs:1077- 付近）。region の layout 追従
    コストが論点。
  - **B-dcomp2**: ruviform 自身の `IDCompositionDevice` +
    `CreateTargetForHwnd(hwnd, false)` を同一 HWND に追加。
    複数 device/target の同一 HWND 共存・non-topmost target が
    `WS_EX_NOREDIRECTIONBITMAP` window で描画されるかは未確認。
    必要 API は `DCompositionCreateDevice`（d3d11 device 経由）+
    `CreateTargetForHwnd`/`CreateVisual`/`SetContent`/`SetRoot`/
    `Commit` — gpui が使う `IDCompositionDevice` base interface で
    十分。windows-sys へ `Win32_Graphics_DirectComposition`/
    `Win32_Graphics_Dxgi`/`Win32_Graphics_Direct3D11` feature 追加が
    必要（system DLL のみ、新規 binary 依存なし）。
  - **B-swapchain（非現実的）**: ruviform 独自 swapchain/present —
    gpui が window・message loop・present を所有するため実現不能。
- 結論: Option-B の最小 private 面積は「gpui 内部再利用」ではなく
  「OS 公開 API による interposition」（env var / window region /
  第二 DComp target）であり、全て実機検証待ちの仮説段階。

## 監査項目 6: gpui バージョン/アップグレード情勢

- crates.io `gpui` の最新 publishable 版は **0.2.2**（2025-10-22
  publish、yanked 版なしでそれ以降の stable なし）。ruviform は
  `gpui = "0.2.2"` を pin（`crates/ruviform-ui-gpui/Cargo.toml:54`）。
- zed repo の現行 gpui は **publish 不能**:
  `zed-industries/zed#55271`（discussion、LeadcodeDev 2026-08-19
  コメント）によれば、`crates/gpui/Cargo.toml` は `publish = true`
  だが依存の `gpui_macros`（自身の manifest で `publish = false`）・
  `gpui_shared_string`/`gpui_util`（workspace `publish = false` 継承）
  が publish 不能で `cargo publish -p gpui` が失敗する。0.2.2 は
  platform split（`zed-industries/zed#49277`）以前の「構造的に別の
  crate」であり、crates.io 版に upstream 修正が降りてくる見込みは
  現状ない。参照は inline code `` `zed-industries/zed#55271` ``
  （auto-reference 抑制の repo 規約に従う）。
- 帰結: 0.2.2 からの脱却手段は (a) zed repo git dependency、
  (b) fork/vendor + `[patch.crates-io]`、(c) 現行 0.2.2 上での
  回避（env var / private interposition）のみ。upstream 追従可能な
  patch 経路は事実上 (b) のみ。

## 監査項目 7: `GPUI_DISABLE_DIRECT_COMPOSITION=1` 時の exit-127 仮説

先行証拠（`evidence/rf388-native-dock-windowing/report.md:72-74`）:
scenario 途中でプロセスが exit 127（開発中観測・ログ artifact なし・
「GPUI 側の別経路の問題と推定」）。本監査はコードパスから仮説を列挙
するのみで、**実 OS 検証は PENDING のまま残す**。

### コードパスから確認した事実

- gpui Windows 経路に `std::process::exit`/`abort` の明示呼び出しは
  **存在しない**（crate 全体で `process::exit` は
  `platform/mac/metal_renderer.rs:139` の `exit(1)` のみ、
  `platform/windows/platform.rs:1054` はコメントアウト済み
  `// std::process::exit(1);`）。
- ruviform 側も exit(127) を呼ぶ箇所なし（PoC は exit(1)/(2)/(3):
  `poc.rs:927`、`dock.rs:669`、`dock_windowing.rs:1256`、`main.rs:36`。
  plugin host の test worker は `--crash-exit-code=` 既定 3）。
- 非 DComp 経路の失敗候補点（全て `anyhow::Result` を返し Err として
  伝播し、exit code 127 を直接生まない）:
  - `create_swap_chain` → `CreateSwapChainForHwnd`（1073）・
    `MakeWindowAssociation`（1074）
  - `DirectXResources::new` の swapchain 分岐（663-672）
  - window 作成（`WS_EX_NOREDIRECTIONBITMAP` なし、window.rs:409-411）
- 失敗伝播: `WindowsWindowState::new` Err → `WindowsWindowInner::new`
  Err → `context.inner` に Err 格納 → `WindowsWindow::new` Err →
  `WindowsPlatform::open_window` Err（platform.rs:408-418）→
  `cx.platform.open_window`（window.rs:956）→ `App::open_window`
  の Result → 呼び出し側の unwrap/expect → panic → 通常 exit 101。
- device-lost は `handle_device_lost` が 5 回 retry（directx_devices.rs
  `try_to_recover_from_device_lost` 28-41）し、最終失敗でも log のみ
  （platform.rs:1050-1055、exit はコメントアウト）。exit しない。

### 仮説（全て未検証・PENDING）

- **H1（launcher/wrapper artifact）**: exit 127 は POSIX 系の
  「command not found」慣用コード。観測は開発中・artifact なしのため、
  mise task / bash wrapper / 子コマンド側の exit code が GUI process
  のものと混同された可能性。GPUI process 自体が 127 で落ちた証拠は
  ない。
- **H2（WndProc 内 panic → abort）**: `CreateWindowExW` の
  WM_CREATE/NCCREATE 処理中に `WindowsWindowState::new` が Err または
  panic した場合、WndProc 越しの panic は FFI boundary で abort になり、
  Windows の abort exit code は runtime 依存で 101 とは限らない。
  非 DComp 経路で新たに失敗しうる `CreateSwapChainForHwnd`/
  `MakeWindowAssociation` がこの候補。
- **H3（DXGI swapchain 生成失敗の別 exit 経路）**: 上記 API が Err を
  返した場合の伝播先で panic 以外の終了（例: anyhow Error を main が
  `Termination` として返す exit code 1、あるいは abort）が 127 と
  観測された可能性。caller 側の error handling 次第。
- **H4（子 process/補助 process の exit code 混同）**: PoC は
  `Command::new("powershell")` で画面 capture を行い
  （dock_windowing.rs:212-213）、plugin host は子 process を spawn
  する。子の exit code が観測値 127 の出所である可能性
  （ただし PoC 自身の exit は失敗時常に 1）。
- **H5（device-lost/Present 異常）**: HWND swapchain の `Present` が
  `DXGI_ERROR_DEVICE_REMOVED`/`DXGI_STATUS_OCCLUDED` を返す経路は
  device-lost 回復（上記）に吸収され、プロセスは終了しない。

### 判定

コードパスの静的解析では exit 127 の直接原因を一意に特定できない。
最も尤度が高いのは H1（観測 artifact 不在の wrapper 混同）または
H2（WndProc 内 abort）であり、**実 OS で `GPUI_DISABLE_DIRECT_COMPOSITION=1`
を付けて stderr/exit code を捕捉する再現実験が必要**。本 slice では
close せず PENDING として残す（先行 discovery の Open Question を
継続）。

## 参照したファイル（vendored gpui-0.2.2 内）

- `src/gpui.rs`（mod platform 宣言・`pub use platform::*`）
- `src/platform.rs`（`mod windows`、pub(crate) trait/型、env var 伝播）
- `src/platform/windows.rs`（submodule 構成・pub(crate) re-export）
- `src/platform/windows/platform.rs`（WindowsPlatform::new・env var 読み取り・
  WindowCreationInfo・open_window・device-lost broadcast）
- `src/platform/windows/window.rs`（WindowsWindow/Inner/State・
  WS_EX_NOREDIRECTIONBITMAP・HasWindowHandle/HasDisplayHandle impl・
  DirectXRenderer 生成）
- `src/platform/windows/directx_renderer.rs`（DirectXRenderer/Devices/
  Resources/DirectComposition・swapchain 2 経路・device-lost 復旧・
  Drop）
- `src/platform/windows/directx_devices.rs`（DirectXDevices・device 作成・
  try_to_recover_from_device_lost）
- `src/platform/windows/vsync.rs`（DwmFlush vsync — DComp 非依存）
- `src/platform/windows/events.rs`（WM_GPUI_GPU_DEVICE_LOST・resize 処理）
- `Cargo.toml`（version 0.2.2・windows crate 0.61・features）

## 参照した外部情報

- crates.io `gpui` version history（0.2.2 = 2025-10-22 publish、最新 stable）
- `zed-industries/zed#55271`（gpui publish 不能の技術的根拠 — inline code
  参照。auto-reference 抑制の repo 規約に従う）
- MSDN `IDCompositionDesktopDevice::CreateTargetForHwnd` の `topmost`
  引数意味論（TRUE = visual tree を window の children の上に描画、
  FALSE = window 内容の一部として合成し children が上に来うる）—
  先行証拠 `evidence/rf388-native-dock-windowing/report.md:66-69` と
  整合。実機 fixture 検証は未実施。
