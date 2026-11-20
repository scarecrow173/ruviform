# Issue #532 Option-A 実施結果 — patched gpui fork 適用 + real lane 再検証

Issue #532（GPUI 0.2.2 DirectComposition 経路でネイティブ子 HWND の画素が
画面に届かず `real-dcomp-pixel` が fail する既知問題）に対する Option-A
スライスの実施記録。先行スライスで確定した判定に従い、`gpui` を
`CreateTargetForHwnd(hwnd, true)` → `false` へ修正した fork へ切り替え、
`RVF_SLOT_HOLE` PoC 機構を撤去し、real lane を再実走した。
**結果: `real-dcomp-pixel` は引き続き fail。Issue #532 は open のまま。**

- Date: 2026-11-20
- OS: Windows 11 (x86_64)、対話セッション（interactive desktop）
- Branch: `feature/532-gpui-composition-target`（base `06a2b414`）
- Lane: `mise run test-ui-plugin-dock-real`（exit 1 — `real-dcomp-pixel`
  fail のみによる正規の失敗。verdict の弱化・隠蔽なし）
- Lane JSONL: `evidence/rf631/plugin-dock-real-scenario.jsonl`
  （本 run が末尾に追記、repo_sha = `06a2b414`）

## 依存切替

`crates/ruviform-ui-gpui/Cargo.toml` の `gpui` を crates.io `0.2.2` から
git fork tag へ変更（`[patch.crates-io]` ではなく git dep — consumer
workspace が patch を継承しないため）。crate 名 `gpui`・version 制約
`=0.2.2` は維持。

- `gpui = { git = "https://github.com/scarecrow173/gpui",
  tag = "v0.2.2-rf1", version = "=0.2.2", optional = true }`
- 解決 commit: `f77e44e64b66c1bcc4757e68ee5dfc903fb96139`
- `Cargo.lock` の `gpui` source は git ソースへ更新済（registry checksum
  なし）。連鎖差分は `num_enum_derive` 内 `proc-macro-crate` の再解決
  （`3.5.0`→`1.3.1`、req `>=1, <=3` 内の既存版への再指し）のみで、
  追加・削除パッケージなし。
- fork 側の修正箇所: `src/platform/windows/directx_renderer.rs` で
  `CreateTargetForHwnd(hwnd, false)`（checkout 内 line ~796、コメントと
  `PATCHES.md` 付き）。
- `cargo tree` 確認: `gpui v0.2.2
  (https://github.com/scarecrow173/gpui?tag=v0.2.2-rf1#f77e44e6)`。

## real lane 結果（fork 適用後、canonical `test-ui-plugin-dock-real`）

| case | verdict |
|---|---|
| real-window-mount / real-native-slot / real-editor-attach / real-capture | pass |
| **real-dcomp-pixel** | **fail**（green_fraction 0.000、1,019,844 px 中 green=0） |
| real-dcomp-foreground / real-dcomp-present / real-dcomp-uncovered | pass |
| real-multiwindow-focus / real-os-input / real-command-receipt | pass |
| real-window-dpi / real-window-close / real-editor-resize / real-editor-redock / real-editor-visibility | pass |
| real-session-terminate / -degraded / -recovery / -timeout | pass |
| real-window-ime / real-session-crash / real-session-hang | unsupported（環境・ケース条件による既知の unsupported、fail ではない） |

**19 pass / 1 fail / 3 unsupported — baseline と verdict 構成が完全一致。**

## 実測から得られた構造的理解（重要）

fork の適用はコンパイル・リンク・起動まで確認済みだが、遮蔽オラクルには
**観測可能な変化が一切なかった**。`real-dcomp-pixel` / `real-dcomp-foreground` /
`real-dcomp-uncovered` の記録から:

1. **子 HWND は正しく描画している。** `uc_capture` で editor child への
   `PrintWindow`（flags 0 / `PW_CLIENTONLY`）は green_fraction=**1.000**
   （1,019,844/1,019,844）。DComp 無効 run では `GetDC`+`BitBlt` でも
   1.000。子の raster に green は確実に存在する。
2. **HWND z-order 上は子が最上位になり得る。** `fg_observe_topmost` /
   `uc_observe_topmost` では `HWND_TOPMOST` 適用後、子矩形内の全
   `WindowFromPoint` プローブが `RuviformTestEditor`（editor_child）を
   返し、`uc_uncovered` は `uncovered: true` と判定した。
3. **それでも画面は変わらない。** topmost 化で子がヒットテストを独占した
   状態でも screen `BitBlt` は green=0.000。**さらに topmost ステージの
   画面 capture SHA256 が baseline run と fork run で完全一致**
   （`7dd8975d5b09…`）— 注入ビジュアルの z-order 変更は最終合成出力に
   一切現れなかった。
4. **先行証拠との整合。** `real-dcomp-present` / `-uncovered` の既存証跡
   （`presentation-probe.md`）は、`GPUI_DISABLE_DIRECT_COMPOSITION=1`
   （`WS_EX_NOREDIRECTIONBITMAP` 不付与 + DComp 経路不使用）でも
   `real-dcomp-pixel` が fail・screen green=0 であることを示済み。
   alpha-hole PoC も「注入ビジュアルを透過しても現れるのは背後の
   ウィンドウであり、子画素はバンド内に存在しない」を実測済み。

つまり遮蔽は「注入ビジュアルが子より上に挿入される」問題**ではない**。
注入ビジュアルの z-order を下げても、DComp 経路をまるごと外しても、
親のリダイレクションサーフェスが存在しても、子の画素は画面に届かない。
遮蔽主体は GPUI ウィンドウの **presentation plane 自体**（swapchain の
合成・flip 経路が子 HWND を含むバンドを上書きする）と見るのが現時点で
全証拠と整合する解釈であり、`CreateTargetForHwnd` の `topmost` 引数は
遮蔽機構ではなかった。

## 次候補（Issue #532 follow-up への入力）

- GPUI の swapchain 生成・present 経路（`DXGI_SWAP_EFFECT_FLIP_*`、
  `DXGI_ALPHA_MODE`、scaling/tearing フラグ）が子 HWND を含むバンドを
  覆う plane を形成していないか（independent flip / MPO 相当の遮蔽）。
- 子 HWND を GPUI ウィンドウの client 直下に置く現行方式をやめ、
  slot を別 top-level（owned/layered）ウィンドウとして追従配置する
  airspace 回避方式の評価。
- `RVF_SLOT_HOLE` 機構は本スライスで撤去済（後述）。PoC 資産（JSONL・
  画素証跡）は `evidence/rf532/` に残置。

## 本スライスで撤去した機構

`RVF_SLOT_HOLE`（`none|region|alpha|both`）関連を `render.rs` /
`gpui_driver/surface.rs` から全撤去 — `HoleMode`・`hole_mode`・
`subtree_has_bound_slot`・`AppliedRegion`・`sync_surface_regions`・
region 計算・alpha 抑制・2 件の決定論テスト・PoC 専用 import。
`NativeSurfaceSlot` 契約・`set_clip` 等の slot 既存機能は変更なし
（両ファイルは PoC 導入前の状態に復元）。

## 検証一覧（本スライス実施分）

| lane | 結果 |
|---|---|
| `check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` | pass（git fork で初回コンパイル確認） |
| `cargo check/clippy/test -p ruviform-ui-gpui`（default / `gpui` / `gpui-test uitest`） | pass |
| `gui-boundary-guard --offline` / native slot contract 13 / fake scenario 17 / gpui scenario 7 | pass |
| `test-ui-plugin-dock` / `test-ui-plugin-dock-console` / `test-ui-plugin-dock-real-headless` | pass |
| `test-ui-plugin-dock-real` | exit 1 — `real-dcomp-pixel` fail のみ（上記）。その他 19 case は全 pass、回帰なし |

Issue #532 は open のまま（`Refs #532`）。
