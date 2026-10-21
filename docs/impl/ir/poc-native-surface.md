---
id: "IMPL-034JDpGD5vR4ITKQsHnye5"
type: "impl"
status: "completed"
title: "PoC ネイティブサーフェス合成"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/poc-native-surface.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# PoC ネイティブサーフェス合成

## Summary

task `TASK-034JfHavtGmdqmRGyDAbch`（plan `PLAN-034JfDEU8mhCDj4ldYFZUG` の T4）として、AC3「ネイティブサーフェス合成」を実装・実機検証した。`src/exp/surface.rs` に `--experiment surface --mode adjacent-parent|adjacent-child` を追加し、wgpu `Surface` と WebView 子ビューの隣接同居を段階駆動の検査マトリクス（9 項目）で評価する。両モードとも実機 `status:"passed"`（`matrix_complete` 自己終了・53 フレーム present・エラー 0）。主要な実発見: (a) WebView2 の子 HWND ツリーは wry コンテナ矩形に厳密に限定され右領域を覆わない、(b) sibling z-order は既定で WebView コンテナが上位に留まるが `HWND_TOP` raise で反転可能、(c) `SetWindowRgn` はコンテナのみで子 HWND の被覆判定を除去できない（DOM overlay 回避としては不十分）、(d) `build_as_child` + 明示 `set_bounds` で viewport 追従は正常動作（T1 の引継ぎ疑念を解消）、(e) `WindowFromPoint` は外来 top-level の混入で一次判定に使えず自ツリー被覆照会が安定。全変更は未コミット。

## Context

- 実施タスク: [../../tasks/poc-native-surface.md](../../tasks/poc-native-surface.md)（`TASK-034JfHavtGmdqmRGyDAbch`）
- 上流: [plan](../../plans/issue-28-ui-poc.md)（T4）・[design](../../designs/issue-28-ui-poc.md)（AC3 実験設計: 2 モード・検査マトリクス・unsafe 方針の正本）・[spec](../../specs/issue-28-ui-poc.md)（AC3）・[test-spec](../../test-specs/issue-28-ui-poc.md)（記録要件・文書評価ラベル・status/exit code 規約）
- 前提: [poc-ui.md](poc-ui.md)（T1 ハーネス。IPC・`build_as_child` 引継ぎ事項・フォーカス機構）・[poc-render-backends.md](poc-render-backends.md)・[poc-message-boundary.md](poc-message-boundary.md)（実験ライフサイクル・poc-metric.v1 規約）
- 前提制約: `unsafe` は SAFETY コメント付き局所許可のみ・wgpu/windows-rs を `exp/surface.rs` に封じる・ルート `Cargo.toml`/`Cargo.lock`/`crates/`/`mise.toml` 無変更・コミットしない

## Implementation

### Changes

- **`src/exp/surface.rs` — AC3 実験本体（新規・~1050 行）**: 汎用 `Host`/`HostEvent` I/F が HWND レベル合成を表現しないため、このモジュールだけが例外として tao/wry/wgpu/windows-rs を直接保持する（module doc に理由を明記 — adapter 分割設計への入力として記録）。専用イベントループ（`EventLoopBuilder::with_user_event` + queue/proxy は host.rs と同じパターン）+ `WaitUntil` 16ms tick で wgpu フレーム駆動しつつ段階（stage 0-5）で検査を進行。検査マトリクス: `surface_init`・`webview_child`・`hit_test_webview`・`hit_test_native`・`focus_transition`・`z_order_overlap`・`resize_follow`・`frame_continuity`・`region_clip`。各項目は `pass`/`fail`/`obs` + note を `metrics.matrix` に記録。`S` 状態構造が IPC 観測（`hello`/`focus`/`resize_observed`）・Resized 履歴・frames 計数・HWND を集約。既定 timeout 30s（`--duration` 上書き可）。
- **2 モードの構成**: `adjacent-parent` = wgpu Surface を tao 親 HWND へ + WebView を `build_as_child` + `with_bounds` で左領域（50%）に配置（unsafe 不要の主経路だが、raw-handle 化の `create_surface_unsafe` は必要）。`adjacent-child` = WebView を `build` + `set_bounds`（実績経路）で左領域 + `ChildHwnd`（windows-rs `CreateWindowExW` `WS_CHILD|WS_VISIBLE|WS_CLIPSIBLINGS`・RAII Drop で `DestroyWindow`）を右領域へ作成し wgpu Surface を張る。リサイズ時は `relayout` が `set_bounds`（WebView 左）+ `SetWindowPos`（sibling）+ `surface.configure` を一括適用。
- **wgpu 30 API 対応**: `get_current_texture` → `CurrentSurfaceTexture`（Success/Suboptimal/Timeout/Occluded/Outdated/Lost/Validation）へ分岐、present は `queue.present(texture)`（SurfaceTexture::present は廃止）、`SurfaceConfiguration.color_space: Auto`、`RenderPipelineDescriptor.multiview_mask` 追加など wgpu 30 の新 API 形に対応。`Surface` は `create_surface_unsafe(SurfaceTargetUnsafe::RawHandle{raw_window_handle: Win32(..)})` で `'static` 化（親 HWND/sibling HWND の両方に同経路で適用）。描画は位相回転クリア + 三角パイプライン（present 実績と可視確認用）。
- **unsafe 封じ込めの機械化**: `poc/ui/Cargo.toml` に `[lints.rust] unsafe_code = "warn"` を追加。`Gpu::new`（`create_surface_unsafe`）・`ChildHwnd::{create,move_to,bring_to_top}`/`Drop`・`hwnd_at`/`hwnd_matches`/`class_of`/`dump_children`/`covering_children`/`clip_hwnd`・`child_wndproc` の各 unsafe 呼出を `#[expect(unsafe_code, reason=...)]` + SAFETY コメントの局所許可に限定し、`clippy -D warnings` で逸脱を機械検出可能にした。
- **依存追加（`poc/ui` のみ）**: `windows = "0.62"`（features: Win32_Foundation・Win32_Graphics_Gdi・Win32_System_LibraryLoader・Win32_UI_WindowsAndMessaging — sibling HWND・ヒットテスト・z-order・クリップ用。wry が内部で使う windows 系と同系列）・`raw-window-handle = "0.6"`（wgpu 30/tao 0.37 の rwh 版と整合）。
- **`src/exp/mod.rs`**: `surface` をレジストリ登録・`known_names()` 追加（module コメントで例外位置づけを明記）。`src/args.rs` usage に `surface` 追加。
- **`poc/ui/RESULTS.md`**: AC3 行（両モード passed）+ 所見（HWND ツリー限定・ヒットテスト判定法・z-order・SetWindowRgn・リサイズ追従・build_as_child 解消・PR #1762 制限事実・adapter 境界発見・unsafe 封じ込め・macOS/Linux 文書評価・MSRV ポインタ）を追記。
- **`docs/impl/exp/`**: `ac3-surface-adjacent-parent.jsonl`・`ac3-surface-adjacent-child.jsonl`（実機 `--out` 追記ログ）。

### Decisions During Implementation

- **専用イベントループ採用（host::run を拡張しない）**: wgpu Surface は `&Window` を要求するため `host.rs` 内部に触らせるか `raw-window-handle` を公開する設計が必要になるが、それは汎用 I/F に AC3 固有の HWND 概念を漏らす。代わりに `surface.rs` 内で完結する専用ループを採り「汎用 I/F で表現不能」という事実そのものを成果として記録する（design の adapter リハーサル評価軸に整合）。
- **ヒットテストの一次判定は自ツリー被覆照会**: 初回実測で `WindowFromPoint` が左右とも自ツリー外の `Chrome_RenderWidgetHostHWND`（外来常駐オーバーレイ混入）を返し誤判定した。実デスクトップでは外来 top-level が混入しうるため `EnumChildWindows` + `GetWindowRect` 包含による被覆判定を一次証拠とし、WindowFromPoint 生値は観察併記とした（合成実入力は PoC 範囲外であることも RESULTS.md に明記）。
- **`adjacent-parent` は `build_as_child` 経路**: design 正本どおり。resize 時の明示 `set_bounds` で JS viewport が左領域に正しく追従することを実機で確認 → T1 引継ぎ「build_as_child では viewport が追従しない」は set_bounds 未再適用が原因だったと特定・記録。
- **`adjacent-child` は `build` + `set_bounds` 経路**: T1 実績のあるホスト経路を使う。ただし wry 親 subclass の WM_SIZE→自動 SetBounds により resize 中に WebView が一瞬全面化（JS viewport 1020 観測→510 に戻る）する遷移を記録 — 実用上の注意として RESULTS.md へ。
- **z-order 試験は明示 raise を含める**: 初期実測で sibling 後置でも WebView コンテナが最上位に留まることを観測（wry 側が上位を維持）。`SetWindowPos(HWND_TOP)` 相当の raise で sibling が最上位化することを確認し「既定は WebView 上・raise で反転可能」を記録。
- **wgpu surface は両モード `create_surface_unsafe` に統一**: safe な `create_surface(&window)` は借用 `Surface<'a>` を返しイベントループへの所有移転が困難なため、raw-handle の `'static` surface に統一（unsafe は `Gpu::new` 内に 1 箇所集約）。
- **検査マトリクスの fail は実験失敗にしない**: 個別項目の `fail`/`obs` は採否材料の記録であり、実験の成立判定は「全項目が記録された + フレーム継続」とした（test-spec の記録要件 vs SLA の区別に準拠）。

## Related Experiments

### Adopted

- [../exp/ac3-surface-adjacent-parent.jsonl](../exp/ac3-surface-adjacent-parent.jsonl) — adjacent-parent 実機実行（matrix 全 9 項目・53 frames・passed）
- [../exp/ac3-surface-adjacent-child.jsonl](../exp/ac3-surface-adjacent-child.jsonl) — adjacent-child 実機実行（sibling HWND・z-order raise・53 frames・passed）

### Rejected

None.（初回試行で `WindowFromPoint` 一次判定が外来ウィンドウ混入で誤判定したため自ツリー被覆照会へ修正 — 判定方法の置換であり実験自体の棄却なし）

## Validation

実施環境: Windows 11 x64 対話デスクトップ・Git Bash・mise `rust = "1.99.0"`・WebView2 155.0.4283.62・wgpu adapter `NVIDIA GeForce RTX 4070 SUPER (Vulkan)`。

| Check | コマンド | 結果 |
| --- | --- | --- |
| PoC ビルド | `mise exec -- cargo build --manifest-path poc/ui/Cargo.toml` | clean・0 warnings |
| fmt | `cargo fmt --manifest-path poc/ui/Cargo.toml -- --check` | 差分なし |
| clippy | `cargo clippy --manifest-path poc/ui/Cargo.toml --all-targets -- -D warnings` | clean・0 warnings（`unsafe_code=warn` + `#[expect]` 封じ込め有効） |
| AC3 parent | `ui-poc --experiment surface --mode adjacent-parent --duration 30` | `status:"passed"`・`exit_reason:"matrix_complete"`・matrix 9 項目・frames_presented 53・frame_errors [] |
| AC3 child | `ui-poc --experiment surface --mode adjacent-child --duration 30` | 同上 + sibling `ui_poc_gpu_child` 生成・`z_order_overlap` pass（HWND_TOP raise 確認） |
| unsafe 限定 | `grep -rn "unsafe" poc/ui/src/` | `exp/surface.rs` の `#[expect]` 局所のみ（他モジュール 0 件） |
| 隔離 | `git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml` | 無出力（変更は `poc/`・`docs/` のみ） |

検査マトリクス実測値（代表、親モード）: `hit_test_webview` pass（左中央 = WRY_WEBVIEW→Chrome_WidgetWin_0/1→Chrome_RenderWidgetHostHWND+Intermediate D3D Window の被覆）・`hit_test_native` pass（右中央 = 被覆子なし=親自身）・`focus_transition` pass（`[false,true]` ペア）・`resize_follow` pass（960x640→1020x700・JS viewport→510px）・`frame_continuity` pass（53）・`region_clip` obs（コンテナリージョン設定は成功するが子 HWND 被覆は残存）・`z_order_overlap` obs（親モードでは sibling なし）。

## Risks

- **ヒットテストは構造判定の近似**: 被覆 HWND 列挙は「その点でどの自ツリー HWND が受けるか」の静的構造判定であり、実クリックイベント配送の検証ではない（範囲外と明記済）。実アプリ重なりで視覚と入力が乖離しうる点は別途 UI 層の検討事項
- **`region_clip` は不十分さの記録止まり**: 子 HWND 毎のリージョン適用や `ICoreWebView2CompositionController` 経路は未試行。DOM overlay 真回避の実用評価は今後（T6 ADR で制限事実として扱う）
- **`build` 経路の一時全面化**: `adjacent-child` で resize 中に WebView が一瞬全クライアント領域化する遷移を観測。製品コードではリサイズ中の視覚ちらつき対策が要る可能性
- **wgpu surface `create_surface_unsafe` のライフタイム契約は実装責任**: `Gpu` drop 順序は現状のスコープ管理に依存（hwnd 破棄前に surface を drop）。将来製品化ではこの順序を RAII で保証する設計が要る
- **wgpu 30 は比較的新しい API 面**: `CurrentSurfaceTexture`/`queue.present`/`color_space`/`multiview_mask` など wgpu 25 系から変更あり。将来の wgpu 更新で再検証が必要
- **macOS/Linux は文書評価のみ**: 実機検証は Windows のみ。別 OS の構造的前提（NSView/CAMetalLayer、Wayland 制約）の実証は後続

## Follow-ups

- T6（`poc-adr-finalize`）: AC3 の実測所見（隣接同居成立・z-order 既定挙動・SetWindowRgn 不十分・PR #1762 制限事実・unsafe 封じ込め実績）を採否条項へ反映。未解決事項（macOS/Linux 実機・DOM 内真合成・wry PR #1762 追跡）は ADR のフォローアップとして残す
- 将来 `ruviform-gpu` adapter 設計時: 「ネイティブ合成は汎用 host I/F の例外」発見を入力に、HWND/surface ライフサイクルを adapter 内に閉じる境界設計を検討
- wry PR #1762 が merge/upstream 化された場合: `ICoreWebView2CompositionController` 経路の DOM 内真合成を再評価（本 PoC では隣接構成を採否対象のまま維持）
