# Issue #532 GPUI DComp public-surface 監査 — 監査手法の試行記録（2026-11-15）

`docs/tasks/issue-532-gpui-dcomp-surface.md` の docs-only 監査で
試行した手法と結果の記録。コード変更を伴わないため、実験は「監査
手法の選択と補正」に限定される。正本の監査結果は
[`docs/discovery/issue-532-gpui-dcomp-surface.md`](../../discovery/issue-532-gpui-dcomp-surface.md)
を参照。

## 試行 1: `CreateTargetForHwnd` の全出現箇所の特定

- **目的**: `topmost` 引数の全サイトを漏れなく列挙する（監査項目 1・
  Option A の patch 面積見積もりの前提）。
- **試行**: vendored `gpui-0.2.2` の `src/` 全体を
  `CreateTargetForHwnd|CreateVisual|SetContent|SetRoot|Commit|
  DCompositionCreateDevice|CreateSwapChainForComposition|
  CreateSwapChainForHwnd|MakeWindowAssociation|IDComposition` で
  grep。
- **観測**: 全ての DComp/DXGI API 呼び出しは
  `src/platform/windows/directx_renderer.rs` 内の各 1 箇所に限定。
  `CreateTargetForHwnd` は 793 行目の `CreateTargetForHwnd(hwnd, true)`
  のみ。`IDComposition*` 型参照も同ファイルのみ。`vsync.rs` は
  コメント言及のみ。`src/platform/blade/` は linux/freebsd/macos
  限定で Windows 非対象。
- **結論**: `topmost` literal は crate 全体で 1 箇所・`true` 1 語。
  Option A の patch 面積は最小（literal 1 token）。採用
  （discovery F1・F4 に反映）。

## 試行 2: env var の読み取りサイトと受理値の特定

- **目的**: `GPUI_DISABLE_DIRECT_COMPOSITION` の意味論を正確に記録
  する（監査項目 2）。
- **試行**: `GPUI_DISABLE_DIRECT_COMPOSITION`・
  `DISABLE_DIRECT_COMPOSITION`・`env::var` を crate 全体で grep。
  `platform.rs` の読み取り箇所と `directx_renderer.rs` の const 定義を
  確認。
- **観測**: const 定義は `directx_renderer.rs:29`（`pub(crate)`）。
  読み取りは `platform.rs:138-139` の `WindowsPlatform::new` 内で
  1 回のみ。受理値は `"true"` と `"1"` のみ
  （`is_ok_and(|v| v == "true" || v == "1")`）。process-wide・
  platform 初期化時に確定・runtime 切替不可。
- **結論**: env var は window 単位の切替ではなく process-wide の
  起動時スイッチ。非 DComp 経路は `CreateSwapChainForHwnd` +
  `WS_EX_NOREDIRECTIONBITMAP` なし + `DXGI_SCALING_NONE`/
  `ALPHA_MODE_IGNORE`（DComp 経路は `SCALING_STRETCH`/
  `PREMULTIPLIED`）。採用（discovery F2 に反映）。

## 試行 3: public vs `pub(crate)` の到達可能性の判定

- **目的**: downstream crate（ruviform）から公開 API 経由で gpui の
  DComp object に触れられるか判定する（監査項目 3・Option B の前提）。
- **試行**: `src/gpui.rs`・`src/platform.rs`・
  `src/platform/windows.rs`・`src/platform/windows/*.rs` の
  `mod`/`pub`/`pub(crate)`/`pub use` を追跡。`HasWindowHandle`/
  `HasDisplayHandle` impl を確認。ruviform 側の
  `crates/ruviform-ui-gpui/src/gpui_driver/surface.rs` の HWND 取得
  経路を確認。
- **観測**: `gpui::platform` は private mod。`Platform`/
  `PlatformWindow` trait・`WindowsPlatform`/`WindowsWindow`/
  `WindowsWindowInner`/`WindowsWindowState`/`WindowCreationInfo`/
  `DirectXRenderer`/`DirectXRendererDevices`/`DirectComposition`/
  `DISABLE_DIRECT_COMPOSITION` const は全て `pub(crate)` 以下。
  `Window` の `HasWindowHandle` は `Win32WindowHandle` を返し
  実 HWND に到達可能（ruviform は既に `surface.rs` で使用中）。
  `HasDisplayHandle` は `WindowsWindow::display_handle` =
  `unimplemented!()` へ転送され Windows で呼ぶと panic。
- **結論**: DComp device/target/visual/swapchain/renderer/window
  exstyle のいずれも公開 API なし。downstream の到達可能な合成経路
  スイッチは env var のみ。採用（discovery F3 に反映）。

## 試行 4: gpui version/アップグレード情勢の確認

- **目的**: gpui 0.2.2 が最新 publishable 版か、zed repo 現行版が
  publish 可能かを確認する（監査項目 6）。
- **試行**: crates.io `gpui` の version history を確認。zed repo の
  gpui publish 状況を `` `zed-industries/zed#55271` `` で確認。
- **観測**: crates.io `gpui` 最新 publishable = 0.2.2（2025-10-22
  publish）。ruviform pin（`gpui = "0.2.2"`）と一致。zed repo 現行
  gpui は publish 不能 — `gpui_macros` が自身の manifest で
  `publish = false`、`gpui_shared_string`/`gpui_util` が workspace
  `publish = false` を継承し `cargo publish -p gpui` が失敗。
- **結論**: 0.2.2 脱却の手段は zed git dep / fork+`[patch]` /
  現行 0.2.2 上の回避（env var・private interposition）のみ。
  採用（discovery F6 に反映）。

## 試行 5: exit-127 のコードパス仮説の列挙

- **目的**: `GPUI_DISABLE_DIRECT_COMPOSITION=1` 時の exit 127 について
  コードパスから原因仮説を列挙する（監査項目 7）。**仮説のみ**とし
  実 OS 検証は PENDING。
- **試行**: gpui Windows 経路の `process::exit`/`abort`/`panic!`/
  `unwrap`/`expect` を grep。非 DComp 経路の分岐
  （`directx_renderer.rs:663-672`・`1073-1074`・`window.rs:
  409-411`）の失敗伝播を追跡。ruviform/PoC 側の exit code を確認。
- **観測**: gpui Windows 経路に `process::exit`/`abort` の明示
  呼び出しなし（crate 内の `process::exit` は mac/metal_renderer.rs
  :139 `exit(1)` のみ）。非 DComp 経路の失敗候補は全て
  `anyhow::Result` で Err 伝播し exit 127 を直接生まない。伝播先は
  `open_window` Result → caller unwrap → panic（通常 exit 101）。
  ruviform/PoC は exit(127) を呼ばない（PoC は 1/2/3、plugin host
  test worker は既定 3）。
- **結論**: exit-127 はコードパスから一意に特定不能。仮説 H1
  （wrapper/launcher の exit code 混同）・H2（WndProc 内 panic/abort）・
  H3（DXGI 失敗の別終了経路）・H4（子 process の exit code 混同）・
  H5（device-lost は回復機構に吸収）を列挙。**実 OS での
  stderr/exit code 捕捉再現が必要で本 slice では close しない**。
  採用（discovery F5 に反映・open 維持）。

## 試行 6: 文書生成スクリプトの出力先と rename

- **目的**: `new_discovery.js`/`new_spec.js`/`new_design.js`/
  `new_plan.js`/`new_task.js`/`new_test_spec.js`/`new_impl_record.js`/
  `new_experiment_log.js` で skeleton を生成し、所定のファイル名に
  揃える。
- **試行**: 各 script を `--dir` で実行。`new_impl_record.js`/
  `new_experiment_log.js` は `--name` を取らず title から slug を
  生成するため、生成後に `mv`/`rm` で所定名に揃えた。
- **観測**: `new_impl_record.js` は `issue-532-gpui-dcomp-public-surface.md`
  を生成（title slug）→ `issue-532-gpui-dcomp-surface.md` に rename。
  `new_experiment_log.js` は `issue-532-gpui-dcomp-public-surface.jsonl`
  を生成（jsonl 形式）→ 削除し、`docs/impl/exp/
  issue-532-gpui-dcomp-surface.md` を `.md` 試行記録として手動作成
  （issue-532-dcomp-audit の precedent に倣う）。
- **結論**: skeleton 生成 → 内容記述 → rename/手動作成の流れを採用。
  `.md` は試行記録、`.jsonl` は実験ログの raw data として区別。
  採用。
