# poc/ui RESULTS — Issue #28 UI 技術検証 PoC 計測結果

採否 ADR（`ADR-034Jc9Cguud7nZ2Tt88Ptu`）が cite する集約エビデンスの正本。
生サンプルは `out/*.jsonl`（gitignore・再生成可能）、過程の観察は
`docs/impl/exp/*.jsonl`（impl-doc 規約）に記録する。

実機で未検証の項目は「**文書評価**」ラベルを必須とし、実測値として記載しない
（test-spec「実機未検証項目は文書評価と明示する」）。

## AC → 実験 → エビデンス行 対応表

| AC | 検証項目 | 実験 | 証跡行 | 状態 |
| --- | --- | --- | --- | --- |
| AC1 | WebView ホスト（wry+tao 起動・埋め込みロード・IPC・リサイズ・フォーカス） | `--experiment host` | [AC1 host](#ac1----experiment-host) | **実測** |
| AC2 | 描画バックエンド（canvas2d/webgl2/webgpu） | `--experiment render` | [AC2 render](#ac2----experiment-render) | **実測** |
| AC3 | ネイティブサーフェス同居 | `--experiment surface` | [AC3 surface](#ac3----experiment-surface) | **実測** |
| AC4 | メッセージ境界（IPC/プロトコル/pull） | `--experiment ipc` | [AC4 ipc](#ac4----experiment-ipc) | **実測** |
| AC5 | フロントエンド FW + バンドラ比較 | `--experiment frontend --variant vanilla\|solid` | [AC5 frontend](#ac5----experiment-frontend---variant-vanillasolid) | **実測** |
| AC6 | 配布（portable zip / 依存整理） | `--experiment dist` + release 単体・zip 検証 | [AC6 dist](#ac6----experiment-distheadless-release-単一バイナリ) | **実測**（Windows）+ 文書評価（macOS/Linux） |
| AC7 | 採否 ADR 確定 | `docs/tasks/poc-adr-finalize.md`（T6・本工程） | [AC7 adr-finalize](#ac7--採否-adr-確定) | **実施**（ドキュメント工程） |
| AC8 | PoC 隔離（ルート無変更・独立 WS） | 構造検査 | [AC8 隔離](#ac8--poc-隔離) | **実測** |

## 環境（env）

`poc-metric.v1` の `env` フィールドは `check.rs` が 1 回だけ検出する。
`--check` で同じ検出経路を確認できる（GUI 非生成・exit 0）。

| 項目 | 値（T1 実機） |
| --- | --- |
| os / arch | `windows` / `x86_64` |
| webview2_version | `155.0.4283.62`（レジストリ `pv` 検出） |
| wgpu_adapter | `NVIDIA GeForce RTX 4070 SUPER (Vulkan)` |
| rustc | `rustc 1.99.0 (b940084d7 2026-09-28)`（mise pin・build.rs 焼き込み） |
| Node / npm | `v26.10.0` / `11.19.1` |

## 再現コマンド

```sh
# フロントエンド最小ビルド（未実施だと実験は status:"skipped"）
cd poc/ui/frontend && npm ci && npm run build && cd ../..

# capability 検出のみ（GUI 非生成・exit 0）
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --check

# AC1 host 実験（自己終了。最終 stdout 行 = poc-metric.v1、out/*.jsonl へ追記）
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment host

# 証跡出力先・タイムアウト指定
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment host --duration 15 --out out/host-manual.jsonl

# AC2 render 実験（backend 別・既定 600 フレームで自己終了）
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment render --backend canvas2d --frames 600
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment render --backend webgl2 --frames 600
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment render --backend webgpu --frames 600   # navigator.gpu 不在 → webgl2 fallback

# AC4 ipc 実験（JSON RTT・バイナリ pull・evaluate_script push・併用。自己終了）
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment ipc --duration 10

# AC3 surface 実験（wgpu + WebView 隣接同居・検査マトリクス。自己終了）
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment surface --mode adjacent-parent --duration 30
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment surface --mode adjacent-child --duration 30

# AC5 frontend 比較実験（variant 別。canvas2d 経路・600 フレームで自己終了）
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment frontend --variant vanilla
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment frontend --variant solid

# AC6 dist 実験（headless・GUI 非生成。WebView2 検出・バイナリサイズ・埋め込み状態を記録）
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment dist

# AC6 release 単一バイナリ（アセット埋め込みモード）
mise exec -- cargo build --release --manifest-path poc/ui/Cargo.toml
poc/ui/target/release/ui-poc.exe --check
poc/ui/target/release/ui-poc.exe --experiment dist

# AC6 portable zip 検証（PowerShell）
#   Compress-Archive -Path <temp>\ui-poc.exe -DestinationPath ui-poc-portable.zip
#   Expand-Archive ui-poc-portable.zip -DestinationPath <temp2> ; <temp2>\ui-poc.exe --check
```

## AC1 — `--experiment host`

| 判定 | 実験 | エビデンス（集約値） | 証跡 | 文書評価 |
| --- | --- | --- | --- | --- |
| **passed** | host | `protocol_load.ok=true`（`http://poc.app/index.html` 埋め込みロード + JS hello 確認）・`ipc_echo.ok=true`（RTT 8.1ms。EventLoopProxy 非起床修正前の実行では 255.7ms・初回 WebView2 ウォームアップ込みでは ~500-940ms）・`resize_follow.ok=true`（要求 `[947,631]` → RO 観測 `[947,631]`、±4px 許容、retries:0。修正前の実行では retries:4）・`focus_transition.ok=true`（`mechanism:"window"`、JS window blur→focus ペア `[false,true,false,true]`、API 記録 `set_focus:invoked`/`focus_parent:ok`/`focus:ok`）・`webview2_version=155.0.4283.62` | `out/host-1792563666.jsonl`（非起床修正後の再実行）・`out/host-1792554329.jsonl`（修正前）・`docs/impl/exp/ac1-host-experiment-wry-tao-webview.jsonl` | 実測（Windows 11 x64） |

### AC1 所見（採否材料）

- **埋め込みアセット + カスタムプロトコル**: `poc://app/` → `rust-embed` 配信で
  dev server 不要のロードが成立。Windows では要求 URL が `http://poc.app/...` に
  書き換えられて届く（ハンドラは両形式を受理する実装）。
- **IPC 往復**: JS `window.ipc.postMessage` JSON intent → `with_ipc_handler` →
  `evaluate_script` echo の往復が成立。RTT は WebView2 起動直後のウォームアップで
  ~500-940ms、以降 ~8-260ms と大きく改善する（初回コストとして記録）。
- **リサイズ追従**: `build(&window)` 経路では wry 親 subclass が WM_SIZE→
  `controller.SetBounds` を直結するが、SetBounds の反映は非同期で、同一値の
  再送が合体/破棄される観測があった（RO が初期 900x600 のみ観測する実行を確認）。
  ±1px 交互ジッタ付きの `set_bounds` 再適用リトライ（本実装・最大4回）で
  確実に追従する。**`build_as_child` 経路では子コンテナ HWND のリサイズが
  JS viewport に反映されない** 観測があり、隣接構成（AC3）の検証ではこの
  非対称を考慮する必要がある（T4 へ引継ぎ）。
- **フォーカス遷移**: バックグラウンド起動プロセスは Windows の foreground
  activation を得られず（`set_focus`→`SetForegroundWindow`+SendInput も失効）、
  tao `Focused` イベントは発生しない（`is_active&&is_focused` 両立が必要）。
  ただし `focus_parent()`→`SetFocus(parent HWND)` が wry 親 subclass の
  `WM_SETFOCUS`→`MoveFocus(PROGRAMMATIC)` リダイレクトを誘発し、DOM の
  window blur→focus イベントペアとして IPC 観測される（`mechanism:"window"`）。
  `document.hasFocus()` ハートビートも遷移の真値として一致する観測を得た。
  OS フォーカス前提の検査項目（AC3 のフォーカス遷移検査）は、この
  「対話セッションでない起動経路では OS フォーカスを取れない」制約を
  前提条件として扱う必要がある。
- **自己終了**: 全メトリック収集後に `exit_reason:"metrics_complete"` で終了。
  完了直前の IPC イベントがデッドラインバッチに乗ったため `timeout_hit:true`
  を併記する実行がある（観測は成立。実時間は ~20s ≒ `--duration` 既定値）。

### wgpu MSRV メモ（design リスク表対応）

`wgpu = "30"` は MSRV 1.87 を要求するが、ルート `workspace.package` の
`rust-version = "1.85"` 宣言（Issue #34 で確定・同 Issue は closed）と衝突しうる。本 PoC は
独立ワークスペース + pin 1.99.0 でビルドするため実害なし。ルート側へ
wgpu を導入する採否になった場合は宣言値の見直しタスク（[Issue #57](https://github.com/scarecrow173/ruviform/issues/57)）へ連携すること
（本 PoC では判断しない）。

## AC2 — `--experiment render`

計測環境: 上記 env 表どおり（Windows 11 x64・WebView2 155.0.4283.62・60Hz 表示）。
ワークロード: スクロール波形ピーク列 ~2,000 点/フレーム（1,000 列 × min/max）
+ 8ch レベルメータ群（DOM transform 更新）。`--frames 600`・warmup 12 フレーム。
`frame_time_ms` は rAF タイムスタンプ差分（vsync 待ち込みの実間隔）、
`work_time_ms` はフレーム内の生成+描画 JS 作業時間。

| 判定 | backend（要求 → 実使用） | frame_time_ms avg/p50/p95/p99 | dropped | achieved_fps | work_time_ms avg/p99 | 証跡 | 文書評価 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **passed** | canvas2d → canvas2d | 16.667 / 16.70 / 16.80 / 16.80 | 0 | 60.00 | 0.46 / 0.70 | `out/render-1792563717.jsonl`・`docs/impl/exp/ac2-render-canvas2d-webgl2-webgpu.jsonl` | 実測 |
| **passed** | webgl2 → webgl2 | 16.666 / 16.70 / 16.80 / 16.80 | 0 | 60.00 | 0.41 / 0.60 | `out/render-1792563728.jsonl`・同上 | 実測 |
| **passed**（fallback） | webgpu → **webgl2** | 16.666 / 16.70 / 16.80 / 16.80 | 0 | 60.00 | 0.41 / 0.60 | `out/render-1792563740.jsonl`・同上 | 実測 |

### AC2 所見（採否材料）

- **`navigator.gpu` は WebView2 155 非公開**: `capabilities.navigator_gpu=false`。
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--enable-unsafe-webgpu"`、
  `+ --enable-features=WebGPUService,WebGPU --enable-dawn-features=allow_unsafe_apis`
  の 2 試行でも変化なし（`--backend webgpu --frames 120/300` で確認）。本環境で
  WebGPU の実計測は不可能であり、機能検出 → WebGL2 フォールバックが設計どおり
  動作した（`metrics.fallback_to={to:"webgl2",reason:"navigator.gpu unavailable"}`・
  `status:"passed"`。フォールバックを失敗としない spec 要件どおり）。
  WebView2 内 WebGPU を採用要件にする場合は、flags 以外の有効化経路
  （SDK 側 opt-in API の有無等）を別途調査する必要がある。
- **計測値の解釈**: 3 バックエンドとも `frame_time_ms` は表示 vsync（16.7ms・
  60Hz）にロックされ `achieved_fps≈60`・`dropped=0`。バックエンド差は
  `frame_time_ms` には現れず、フレーム内作業時間 `work_time_ms`
  （canvas2d avg 0.46ms・webgl2/webgpu-fallback avg 0.41ms、p99 ≤0.7ms）側の
  余裕が主証拠。本ワークロード（~2,000 点ピーク + 8 メータ）では 60fps
  更新に対し 30 倍以上のヘッドルームがある。
- **IPC 配送の非起床問題（T2 で発見・修正）**: WebView2 の IPC / ページロード
  callback は tao `WaitUntil` スリープを起床させず、ウィンドウイベントが無い
  区間の IPC メッセージは deadline まで滞留した（初回 render 実行は bench
  完了後も全 60s を消費し `timeout_hit` 終了）。`host.rs` で `EventLoopProxy`
  の user event による明示起床を実装し、`done` 受信 → 即終了
  （`exit_reason:"done_signal"`）となった。同根因で T1 host 実験に
  あった「`timeout_hit:true` 併記」も解消し、再実行で `ipc_echo` RTT が
  255-940ms → 8.1ms、`resize_follow` retries 4 → 0 に改善。
- **WebView2 UDF ロック競合**: 連続実行で前回ブラウザ子プロセスが UDF
  シングルトンロックを保持し、WebView2 初期化がブロックされる事象を観測
  （>100s ハング 1 件）。`host.rs` の既定 data_dir を
  `ui-poc-webview2-<pid>` のプロセス固有パスへ変更し隔離。
- **status 契約の実測**: `UI_POC_HEADLESS=1` → `skipped`（reason 非空・exit 0）、
  `--backend` 欠落/不正値 → `failed`（reason 付き・exit 1）、
  frontend 未ビルド → `skipped`（`vanilla/index.html` 単位で部分ビルドも検出）。

## AC3 — `--experiment surface`

| 判定 | 実験 | エビデンス（集約値） | 証跡 | 文書評価 |
| --- | --- | --- | --- | --- |
| **passed** | surface `adjacent-parent` | 検査マトリクス全 9 項目記録: `surface_init` pass（tao 親 HWND へ wgpu Surface+device+pipeline 生成）・`webview_child` pass（`build_as_child` で左領域配置・hello 観測）・`hit_test_webview` pass・`hit_test_native` pass・`focus_transition` pass（DOM blur→focus ペア）・`z_order_overlap` obs・`resize_follow` pass（960x640→1020x700・JS viewport→左 510px）・`frame_continuity` pass（53 フレーム present・エラー 0）・`region_clip` obs | `out/ac3-surface-adjacent-parent.jsonl`・`docs/impl/exp/ac3-surface-adjacent-parent.jsonl` | 実測（Windows 11 x64） |
| **passed** | surface `adjacent-child` | 同上 + sibling 子 HWND `ui_poc_gpu_child`（`WS_CHILD`・480x640 右領域）生成・`hit_test_native` pass（右領域の最上位 = sibling）・`z_order_overlap` pass（`HWND_TOP` raise で sibling が WebView 領域の最上位化を確認）・`frames_presented` 53 | `out/ac3-surface-adjacent-child.jsonl`・`docs/impl/exp/ac3-surface-adjacent-child.jsonl` | 実測（Windows 11 x64） |

### AC3 所見（採否材料）

- **隣接同居は両モードで成立**: `adjacent-parent`（親 HWND が wgpu 描画面・
  WebView が左領域の子）と `adjacent-child`（右領域に sibling `WS_CHILD`
  HWND + wgpu Surface）の双方で、WebView 描画・IPC・wgpu present が
  同時に継続した（53 フレーム・0 エラー・resize/z-order/clip 各試験を挟んで継続）。
- **WebView2 子 HWND ツリーはコンテナ矩形に厳密に限定される**:
  `WRY_WEBVIEW`（wry コンテナ）→ `Chrome_WidgetWin_0` → `Chrome_WidgetWin_1`
  → `Chrome_RenderWidgetHostHWND` + `Intermediate D3D Window` の全 5 層が
  左領域矩形（実測 480x640 → resize 後 510x700）を持つことを
  `EnumChildWindows` で確認。右領域を覆う WebView2 系子 HWND は存在しない
  → ネイティブ描画は視覚的に隣接同居する。
- **ヒットテスト判定方法**: `WindowFromPoint` は実デスクトップで外来
  top-level ウィンドウ（観測: `Chrome_RenderWidgetHostHWND` `0x26031c`、
  自ツリー列挙に存在しない = 他プロセス/オーバーレイ混入）を返し得るため
  一次判定に使えない。本実験は `EnumChildWindows` + `GetWindowRect` 包含による
  「自ツリー内被覆 HWND」で判定し、WindowFromPoint 生値は観察値として併記
  （合成クリック実入力の検証は PoC 範囲外）。
- **z-order**: `adjacent-child` の sibling は生成後も WebView コンテナが
  最上位に留まる初期順序を観測（wry 側がコンテナを上位に維持する挙動）。
  `SetWindowPos(HWND_TOP)` の明示 raise で sibling が WebView 領域の最上位に
  なることを確認 → 「**windowed WebView2 の子 HWND は同一ウィンドウの
  ネイティブ面より上に座る**」既知の制約と整合し、z-order 制御で一時的な
  上書きは可能。
- **SetWindowRgn クリップ試行**: `WRY_WEBVIEW` コンテナへのリージョン設定
  自体は成功するが、切り欠き領域でも子 HWND ツリー（Chrome_WidgetWin_*）の
  被覆は残存した（子 HWND は親リージョンで構造的に切り抜かれない）。
  → DOM overlay 回避として `SetWindowRgn` 単体は不十分。実用には子 HWND 毎の
  リージョン設定か `ICoreWebView2CompositionController` + DirectComposition
  経路が要る。
- **リサイズ追従**: `set_bounds`（WebView 左領域）+ `SetWindowPos`（sibling）+
  `surface.configure` で両モードとも追従。`adjacent-child`（`build` 経路）
  では wry 親 subclass の WM_SIZE 自動 SetBounds により一瞬全面化
  （JS viewport 1020 を観測）してから実験側 `set_bounds` で 510 に戻る
  遷移を記録 — 同居構成では resize 中に WebView が一時的に全面化し得る
  実用上の注意。
- **`build_as_child` viewport 追従（T1 引継ぎの解消）**: `adjacent-parent` は
  `build_as_child` + `with_bounds` で構成し、resize 時の明示 `set_bounds` で
  JS viewport が左領域 510px に正しく追従した。T1 の「build_as_child では
  viewport が追従しない」観測は、あの経路ではリサイズ時に set_bounds を
  再適用していなかったことが原因と判明（明示再適用で追従する）。
- **フォーカス遷移**: `focus_parent()`→`focus()` で DOM blur→focus ペアを
  両モードで観測（AC1 と同じ `mechanism:"window"` 経路。対話セッション外
  では OS フォーカスを取れない制約は変わらず）。
- **PR #1762 制限事実**: Windows の windowed WebView2 では子 HWND が同矩形
  ネイティブ面の上に座るため、真の下層 DOM 内合成（ネイティブ描画の上に
  DOM を重ねる）には `ICoreWebView2CompositionController` + DirectComposition
  が必要で、wry は未対応（upstream 未 merge PR
  [#1762](https://github.com/tauri-apps/wry/pull/1762)）。本 PoC では
  fork スパイクは未実施（時間外オプション）。**採否は「隣接別サーフェス
  + WebView オーバーレイ」方針のまま評価可能** — DOM 内真合成は将来拡張。
- **adapter 境界の発見**: 本実験は `host.rs` の汎用 `Host`/`HostEvent`
  I/F では HWND 合成を表現できず、`exp/surface.rs` が wgpu・windows-rs・
  wry 子ビューを直接保持する構成になった。「ネイティブ合成実験は汎用
  ホスト I/F の例外ケース」自体が adapter 分割設計（将来の `ruviform-gpu`
  adapter）への入力。
- **unsafe 封じ込め**: `poc/ui/Cargo.toml` に `unsafe_code = "warn"` を追加し、
  windows-rs 呼出と `create_surface_unsafe` の計 12 箇所を
  `#[expect(unsafe_code)]` + SAFETY コメントの局所許可に限定。
  `clippy -D warnings` で封じ込めを機械検証（expect 外の unsafe は warn→deny）。
- **macOS / Linux（文書評価）**: 実機未検証。**macOS** は layer-backed
  NSView + `CAMetalLayer` sibling の隣接合成が最有力（NSView は layer 追加が
  低コスト・z-order は subviews 順で DOM 上段配置も構造的に可能。要自前実装）。
  **Linux** は X11/Wayland とも真の下層合成は実質不可寄りで、隣接構成か
  Canvas 一本化に収束する見込み。どちらも実測ではなく文書評価として扱う。
- **wgpu MSRV**: `wgpu = "30"` の MSRV 1.87 がルート宣言 `rust-version =
  "1.85"`（Issue #34 で確定・closed。追跡は Issue #57）と衝突する事実は [AC1 の MSRV メモ](#wgpu-msrv-メモdesign-リスク表対応)
  に記録済み（pin 1.99.0 で実害なし・引き上げ判断は本 Issue ではしない）。

## AC4 — `--experiment ipc`

計測環境: 上記 env 表どおり（Windows 11 x64・WebView2 155.0.4283.62・60Hz 表示）。
計測ページ: `poc://app/ipc/index.html`（`frontend/ipc` workspace + bench-shared
`ipcbench.ts`）。フェーズ構成: JSON RTT（64B/1KiB/16KiB/64KiB × 各 32 往復）
→ 単方向 ingest（500 msg フラッド）→ バイナリ pull（64KiB ×16・1MiB ×8・
16MiB ×3）→ push（200 件バースト + 4ms tick 名目 ×160 件定速）→ coexist
（定速 push 96 件中に rtt ping ×8 + pull fetch ×6 を並行）。
反復回数・サイズ系列は Rust 側 `exp/ipc.rs` が `__poc.start(cfg)` で注入する単一源泉。

| 判定 | 経路 | エビデンス（集約値） | 証跡 | 文書評価 |
| --- | --- | --- | --- | --- |
| **passed** | JSON RTT | `ipc_rtt_ms` 64B: avg 0.57-1.06 / p50 0.4 / p99 ≤3.3・1KiB: avg 0.46 / p50 0.4 / p99 1.2・16KiB: avg 2.11 / p50 1.6 / p99 7.1・64KiB: avg 4.33-4.42 / p50 4.3 / p99 ≤5.8（ms。各 32 往復・missed=0） | `out/ipc-1792564680.jsonl`・`out/ipc-1792564737.jsonl`・`docs/impl/exp/ac4-ipc-json-rtt-pull-evaluate-script-push.jsonl` | 実測 |
| **passed** | 単方向 ingest | `ipc_ingest_msg_s` ≒ 40,396-44,113 msg/s（500 msg・Rust 側到達窓 11.3-12.4ms・received=500/500・end_seen=true） | 同上 | 実測 |
| **passed** | バイナリ pull | `protocol_pull_MBps` 64KiB: 2.86-2.95・1MiB: 94.3-115.1・16MiB: 88.4-146.1（要求毎 `protocol_pull_latency_ms`: 64KiB p50 1.6-2.2ms・1MiB p50 ~11ms・16MiB p50 ~189ms） | 同上 | 実測 |
| **passed** | push | `push_msg_s` 192.6-218.2 msg/s（送信 burst 200 + paced 160 + coexist 96 = 456 件、受信 360+96・**dropped=0**）。`coalesced` 231-276/360・`max_batch` 105-109・interval p50 0.1ms / p95 15.8ms | 同上 | 実測 |
| **passed** | 3 経路併用 | 定速 push 受信中に rtt ping avg 0.71-0.94ms（単独 64B の 0.57-1.06 と同水準）・pull 64KiB avg 3.3ms / 1MiB avg 10ms・coexist push 96/96 受信・dropped=0 | 同上 | 実測 |

### AC4 所見（採否材料）

- **JSON intent 往復は実用的な遅延域**: ≤1KiB ペイロードで p50 0.4ms・p99 ≤3.3ms
  の RTT。16KiB/64KiB ではペイロード比例で 2-4.4ms へ増大（postMessage の
  JSON 文字列コスト＋往復配送）。UI 操作 intent・計測制御の用途には十分。
  T1 の ipc_echo RTT 8-940ms は WebView2 ウォームアップ＋非起床問題の
  影響であり、修正後の定常値は本表のとおり。
- **単方向 ingest はバックログ排水型で ~40k msg/s**: 500 件フラッドは
  Rust 側ディスパッチ窓 ~11-12ms で消化（1 msg ≒ 22µs、serde パース込み）。
  配送はドロップなし（500/500 到達）。計測された値は「キュー滞留分の排水
  レート」であり、連続高レート配信時の定常値ではない点に注意。
- **バイナリ pull は ≥1MiB で ~90-146 MB/s**: `fetch("poc://data/blob?bytes=N")`
  → `arrayBuffer()` 完走までで 1MiB ~11ms・16MiB ~190ms。高頻度描画データ
  相当の pull 転送として実用的な帯域。一方 64KiB 級の小要求は
  **初回リクエストに ~330ms のコールドパス**（遅延確保バッファ初期化 +
  カスタムスキーム経路ウォームアップ）が乗り、p95 が 330ms となる
  （2 回目以降は p50 1.6-2.2ms）。小さな pull を高頻度で使う設計は
  ウォームアップリクエストを含めるか、初回コストを許容する必要がある。
- **evaluate_script push はドロップしないが暗黙に coalesce される**:
  456 件送信で dropped=0。ただし受信側では明確にバッチ化される —
  バースト 200 件は最大 ~105-109 件/タスクのまとめ着弾、4ms tick 名目の
  定速送信も受信間隔 p50 0.1ms・p95 15.8ms のクラスタ状（interval 分布が
  ~0ms と ~15.6ms に二峰化。15.6ms は Windows タイマ量子と一致）。
  すなわち「1 evaluate_script = 1 JS タスク実行」という粒度は
  **高レートでは成立せず**、受信側はまとめて drain される。
- **Rust 側定速送信は OS タイマ分解能に支配される**: `tick`（tao
  `WaitUntil`）名目 4ms に対し実配信 ~11.1-11.2ms/tick（`tick_avg_ms`）。
  これにより paced push の実送信レートは ~90 msg/s 頭打ちとなり、
  `push_msg_s` 全体値（~193-218 msg/s）はバースト区間込みの合成値。
  ms 級の精密 pacing が必要なら、1 tick 複数送信か別の駆動源が要る。
- **3 経路併用は実現可能**: 定速 push 受信中の rtt ping・pull fetch は
  遅延劣化ほぼ無し・ドロップ 0 で完走。**設計含意**: 高頻度状態同期は
  「evaluate_script の逐次 push をそのまま UI 更新として消費する」構造を
  避け、(a) 大容量・高頻度データは pull（`poc://data` fetch）型で
  フロント側が間隔・併合を制御する、(b) push 通知は seq 付き差分
  （latest-wins で coalesce 耐性）にする、(c) UI 操作 intent は低レートの
  JSON intent を使う — という棲み分けが計測と整合する。
  coalesce 観察は design Open Question への実測回答として記録する。

## AC5 — `--experiment frontend --variant vanilla|solid`

計測環境: 上記 env 表どおり（Windows 11 x64・WebView2 155.0.4283.62・60Hz 表示）。
対象: 同一ベンチ UI（AC2 ワークロード + bench-shared 計装）を **vanilla TS**
（参照・FW ランタイムゼロ下限）と **Solid**（`vite-plugin-solid` 2.11.14 +
`solid-js` 1.9.17・fine-grained signals）の 2 候補で構築。`poc://app/<variant>/index.html`
で render 実験と同じ状態機械ベンチ（canvas2d・600 フレーム・warmup 12）を実行し、
併せて FS/埋め込み側メトリック（dist バイト・ビルド時間・glue LOC）を収集する。
DOM mutation 計装は bench-shared `MutationMeter`（MutationObserver:
attributes/childList/characterData/added/removed）で両 variant 同一経路。
`meters_driver` で実使用のメータ更新経路（vanilla=`createMeterHost` /
solid=`injected`＝Solid signal 駆動）を証跡化した。

| 判定 | variant | dist bytes raw / gzip-9 | build_ms | glue LOC（総/非空） | DOM mutations/meter 更新 | 証跡 | 文書評価 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **passed** | vanilla | 11,371 / 5,152 | 1,417 | 91 / 83 | 1.0（attr=style のみ 4,904・childList churn 53・characterData 0） | `out/frontend-vanilla-4.jsonl`・`docs/impl/exp/ac5-frontend-compare.jsonl` | 実測 |
| **passed** | solid | 20,194 / 8,682 | 2,035 | 188 / 173 | 1.0（attr=style のみ 4,904・childList churn 53・characterData 0） | `out/frontend-solid-4.jsonl`・同上 | 実測 |

gzip-9 はファイル毎に `flate2`/miniz_oxide best で圧縮した合計値
（`gzip9_impl` メトリックに明記）。GNU `gzip -9` 実測のクロスチェック値は
vanilla 5,169B・solid 8,702B で、実装差 −17B / −20B（0.3% 未満）に留まる。
`meta_total_bytes_match=true`（埋め込みアセット列挙と FS 側 build-meta.json の
total_bytes が一致）を両 variant で確認。

### AC5 所見（採否材料）

- **dist サイズ**: solid-js ランタイム分で raw +8,823B（+78%）・gzip-9 で
  +3,530B（+69%）。ホスト全体（release exe 8.1MB）に対し gzip 差 +3.5KB は
  無視できる水準だが、vanilla が「FW ランタイムゼロ下限」として機能した。
- **ビルド時間**: vanilla 1,417ms → solid 2,035ms（+44%。初回計測では
  1,659ms/2,186ms で +32% — 測定分散あり。`build_ms` は `tsc --noEmit +
  vite build` のワークスペース単位時間）。`babel-preset-solid` の
  JSX→リアクティブ変換コスト分、Solid が常に遅い。開発ループへの実害はない。
- **glue LOC**: vanilla 91 行（非空 83）→ solid 188 行（非空 173、+2.1 倍）。
  Solid 側は JSX コンポーネント（`Meters`）+ `render()` 配線 + signals で
  構造化される一方、絶対量は小さい。bench UI 規模では宣言的構造の利得より
  ボイラープレートが上回る。
- **更新粒度（本比較の核心的観察）**: `dom_mutations` が両 variant で
  **完全に同型** — attributes=4,904（全て `style`・8ch×613 frame で
  mutations/meter 更新=1.0 丁度）、child_list=53（added/removed 各 53、
  `hud.textContent` 置換 ~41 回 + プラットフォーム要因 ~12 の variant 共通経路）、
  character_data=0。**Solid の fine-grained 更新は手書き vanilla と同一の
  部分適用粒度を実測で確認** — VDOM 的な childList churn はゼロで、
  signal→`style.transform` の直接適用に収束した。
- **実行コスト**: `work_time_ms` avg は vanilla 0.53ms / solid 0.52ms、
  achieved_fps 60.0・dropped=0 で実測差なし。ベンチ規模では FW オーバーヘッドは
  計測分解能以下。
- **開発ワークフロー観察**: 両 variant とも `frontend/<name>/` workspace +
  `build.mjs` の VARIANTS 登録で同一経路（`npm run build` →
  `dist/<variant>/` + `build-meta.json` → rust-embed）に載る。Solid は
  vite.config に `vite-plugin-solid` を足すだけで Vite 7.3.7 の peer 適合を
  確認済み。**Svelte 5 は design どおり YAGNI 延期** — 2 候補で AC5 判定に
  十分であり、追加は VARIANTS 列 + variant src のみで同一計装経路に載るため
  容易（必要時に追加可能なことを記録）。
- **status 契約**: `--variant` 欠落/不正値は `failed`、variant の dist 欠落は
  `skipped`（setup は args を見ないため run 側で判定）。

## AC6 — `--experiment dist`（headless）+ release 単一バイナリ

`--experiment dist` は `check_gui_prereqs` を呼ばない **headless 実験**
（GUI 非生成・前提不足 skip なし）として実装し、WebView2 レジストリ検出
（`check.rs` の winreg HKLM/HKCU `pv` 値経路）・`current_exe()` サイズ・
埋め込みアセット状態・bootstrapper 案内を記録する。

| 判定 | 検証 | エビデンス（集約値） | 証跡 | 文書評価 |
| --- | --- | --- | --- | --- |
| **passed** | dist（debug） | `webview2_version=155.0.4283.62`・`binary_bytes=18,052,096`・`embedded_ok=true`・`embed_mode=fs-debug`・`frontend_built=true`・`app_assets=11`・`index_html_bytes=3,316`・`bootstrapper_required=false`（検出済みのため案内は記録のみ） | `out/dist-manual-1.jsonl`・`docs/impl/exp/ac6-dist-single-binary.jsonl` | 実測 |
| **passed** | release 単体 | `ui-poc.exe` **8,100,864B**。exe 単体を frontend/dist 不在の temp dir へコピー → `--check` 全項目（os/arch/webview2/wgpu/rustc）報告・`--experiment dist` passed・`embed_mode=embedded-release`・`--experiment frontend --variant vanilla` も passed（poc://app/vanilla が埋め込みのみで完走） | `out/dist-release-final.jsonl`・同上 | 実測 |
| **passed** | portable zip | `Compress-Archive` で exe 単体 zip 化（**3,039,116B**・deflate 62% 削減）→ 別 temp dir へ `Expand-Archive` → `--check` 全項目報告・`--experiment dist` passed/`embedded_ok=true`（zip 内 exe 8,100,864B で同一物） | 同上 | 実測 |
| 文書評価 | macOS | `.app` バンドル + codesign/公証（notarytool）・`productbuild`（.pkg）または DMG 配布。wry は WKWebView を OS 同梱で使うため追加ランタイム不要だが、Gatekeeper 通過には Developer ID 署名 + 公証が必須。本 PoC では macOS 実機未検証 | — | **文書評価** |
| 文書評価 | Linux | `libwebkit2gtk-4.1` + GTK3 実行時依存（deb では `Depends: libwebkit2gtk-4.1-0, libgtk-3-0` 等を宣言）。glibc ベースラインは「最も古い対象ディストロでビルド」が定石。AppImage は依存同梱で回避可能だが WebKitGTK 同梱で大幅肥大化。本 PoC では Linux 実機未検証 | — | **文書評価** |

bootstrapper 案内（WebView2 未検出時に stderr へも出力）:
`WebView2 Evergreen Runtime not detected. Install via the official bootstrapper
(https://go.microsoft.com/fwlink/p/?LinkId=2124703) or
winget install Microsoft.EdgeWebView2Runtime` — Evergreen 配布の公式経路。

### AC6 所見（採否材料）

- **単一バイナリ配布は成立**: release ビルドで rust-embed が `frontend/dist`
  全アセット（index + vanilla/solid/ipc 各バンドル + build-meta.json）を
  exe に焼き込み、`frontend/dist` を持たない temp dir 単体でも `--check`・
  全実験が完走した。配布物は exe 1 ファイルのみで、実行時前提は
  **WebView2 Evergreen Runtime のみ**（本機 155.0.4283.62）。
- **portable zip は最小構成で実用**: exe 1 個の zip が 3.0MB に圧縮され、
  解凍先で追加セットアップ無しに動作。spec の「NSIS/WiX/Inno 等または
  portable zip のいずれか」hands-on 要件を portable zip で充足し、
  NSIS/WiX は参照評価（インストーラ必要時の後続判断）とする。
- **dist 実験は headless 契約**: GUI を作らず前提 gating もしないため、
  非表示環境・CI でも検出値（WebView2 有無・バイナリサイズ・埋め込み状態）を
  `poc-metric.v1` として記録できる。`bootstrapper_required` で未検出環境を
  機械判別可能。
- **debug/release で埋め込みモードが異なる**: `embed_mode` メトリックで
  `fs-debug`（rust-embed が FS 参照）/ `embedded-release`（バイナリ内埋め込み）を
  明示 — 「release で埋め込まれている」をメトリック自体が証明する。

### Linux 実行時依存リスク定量化（文書評価）

wry は Linux で WebKitGTK を使い、API バージョンでリンクする libsoup/GTK が
決まる。パッケージ名と API バージョンの対応を整理した（2026-10 時点・文書評価）:

| API バージョン | GTK | libsoup | 状態 | ディストロ例 |
| --- | --- | --- | --- | --- |
| `webkit2gtk-4.0` | GTK3 | soup2 | **廃止方向**: WebKitGTK 2.52（2026-03・2.51.1 で soup2 コード削除）以降は提供されない。Debian unstable の webkit2gtk 2.52.0-1 は 4.1 のみ出荷、bookworm は 2.50.x に残留 | 旧 Debian/Ubuntu（～bookworm）の `libwebkit2gtk-4.0-37` |
| `webkit2gtk-4.1` | GTK3 | soup3 | **現行・唯一の現実経路**: wry/Tauri v2 系の依存先。Debian/Ubuntu `libwebkit2gtk-4.1-0`（dev は `-dev`）・Fedora `webkit2gtk4.1`・Arch `webkit2gtk-4.1` | Ubuntu 22.04+・Debian 12+・Fedora 38+ |
| `webkitgtk-6.0` | GTK4 | soup3 | GTK4 系。wry の tao/GTK3 経路とは非互換のため対象外 | Fedora 等 |

- **リスク**: `libwebkit2gtk-4.1` が無い古いディストロ（Ubuntu 20.04 以前・
  Debian 11 以前・CentOS/RHEL 系の glib2<2.70 環境）では soup3 経路自体が
  提供されず、deb 依存宣言で解決不能 → AppImage 同梱かターゲット下限の
  引き上げが必要。soup2/soup3 を同一プロセスに混在させるとクラッシュする
  制約（両バージョンの同時リンク不可）にも注意。
- **glibc ベースライン**: 配布バイナリの glibc 要求は「最も古い対象
  ディストロでビルド」（manylinux 的発想・Debian oldstable や Ubuntu LTS での
  ビルド）で下げるのが定石。AppImage は glibc 以外の依存を同梱できるが
  WebKitGTK 同梱で大幅に肥大化するため、deb/Flatpak 等の配布系との
  棲み分け判断が要る。
- **macOS**: `.app` バンドル化 + Developer ID 署名 + notarytool 公証が
  Gatekeeper 通過の前提。配布形態は `productbuild`（.pkg インストーラ）か
  DMG ドラッグ＆ドロップ。WKWebView は OS 同梱のためランタイム追加依存は
  無いが、署名/公証は CI ワークフロー組込みが必要（本 PoC 未検証）。

## AC7 — 採否 ADR 確定

T6（`docs/tasks/poc-adr-finalize.md`）のドキュメント工程。
[採否 ADR `ADR-034Jc9Cguud7nZ2Tt88Ptu`](../docs/adr/ui-tech-selection-poc.md) の
全 7 採否条項を、本書の AC1–AC6・AC8 エビデンス行と `docs/impl/exp/*.jsonl`
へのリンク付きで confirm / revise し、status を `proposed` → `accepted` へ
遷移させた。実機未検証項目（macOS / Linux 実挙動・wry PR #1762 依存の
DOM 内合成・WebGPU 有効化経路）は「文書評価」/ 後続調査として明示したまま
採否に反映している。

| 判定 | 採否条項 | verdict | 根拠エビデンス |
| --- | --- | --- | --- |
| **passed** | WebView 実装（`wry` 0.57 + `tao` 0.37 採用） | confirm | [AC1](#ac1----experiment-host)・`docs/impl/exp/ac1-host-experiment-wry-tao-webview.jsonl` |
| **passed** | 高頻度描画（特性別併用） | revise（WebView2 155 では `navigator.gpu` 非公開・flags でも有効化せず → in-WebView WebGPU は実計測不能で「Windows 向け opt-in 特性」から現時点では採用要件外へ修正） | [AC2](#ac2----experiment-render)・`docs/impl/exp/ac2-render-canvas2d-webgl2-webgpu.jsonl` |
| **passed** | ネイティブサーフェス合成（隣接別サーフェス + WebView） | revise（隣接同居は両モードで成立を確認する一方、既定 z-order は WebView コンテナ最上位・`SetWindowRgn` は子 HWND ツリーを切り抜けず DOM overlay 回避策として不十分と判明 → 回避策記述を修正。DOM 内真合成は PR #1762 依存の文書評価のまま） | [AC3](#ac3----experiment-surface)・`docs/impl/exp/ac3-surface-adjacent-{parent,child}.jsonl` |
| **passed** | メッセージ境界（3 経路併用） | confirm（併用の実現性を実測。`evaluate_script` の暗黙 coalesce・小 pull の初回コールドパス・Rust 定速送信のタイマ量子支配を設計含意として条項へ追記） | [AC4](#ac4----experiment-ipc)・`docs/impl/exp/ac4-ipc-json-rtt-pull-evaluate-script-push.jsonl` |
| **passed** | フロントエンド FW（Solid 採用） | confirm（vanilla 比較で同一の部分適用粒度・無視できるコスト差を実測。Svelte 5 / Preact は不採用・vanilla は最小下限として位置づけを明記） | [AC5](#ac5----experiment-frontend---variant-vanillasolid)・`docs/impl/exp/ac5-frontend-compare.jsonl` |
| **passed** | 配布（埋め込み単一バイナリ） | confirm（release 8,100,864B 単体・portable zip 3,039,116B の hands-on 充足。macOS / Linux は文書評価維持） | [AC6](#ac6----experiment-distheadless-release-単一バイナリ)・`docs/impl/exp/ac6-dist-single-binary.jsonl` |
| **passed** | 採否の構造（コンポーネント単位・固定フォールバック階層にしない） | confirm | 全条項 + [AC8](#ac8--poc-隔離) |

再現コマンド（本工程の検証入口。`.agents/skills/` は `mise run setup` の
`apm install` で復元される gitignore 対象依存のため、未セットアップ環境では
先に `mise run setup` を実行すること）:

```sh
node .agents/skills/doc-status/scripts/audit_docs.js --type adr --json
node .agents/skills/doc-status/scripts/audit_docs.js --type all --json
node .agents/skills/impl-doc/scripts/audit_impl_record.js --dir docs/impl/ir --json
node .agents/skills/impl-doc/scripts/audit_experiment_log.js --dir docs/impl/exp --json
git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml   # 空であること（AC8 再確認）
```

証跡: `docs/impl/exp/ac7-adr-finalize.jsonl`（confirm/revise の逐条記録）・
`docs/impl/ir/poc-adr-finalize.md`（実施記録）。

## AC8 — PoC 隔離

| 判定 | 検査 | エビデンス | 文書評価 |
| --- | --- | --- | --- |
| **passed** | ルート無変更 + 独立ワークスペース | `poc/ui/Cargo.toml` は `[workspace]`+`[package]` の独立 WS（ルート `members` は明示リストで `poc/` を含まないため `exclude` 不要・吸収されない）。`git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml` が無出力。`mise run check`/`lint`/`test` が本変更前後で同一に成功（検証ログは `docs/impl/ir/poc-ui.md`） | 実測 |

### 依存の境界

wry 0.57 / tao 0.37 / wgpu 30 / rust-embed 8 / serde_json / http / dpi /
pollster / raw-window-handle / flate2 / winreg・windows（Windows のみ）は
`poc/ui` ワークスペース内に限定し、
ルート `Cargo.toml`/`Cargo.lock`/`crates/` へは一切追加しない
（`poc/ui/Cargo.lock` はこのワークスペース内で生成・コミット対象）。
`unsafe`・windows-rs 直接呼出は T1 では不使用だったが、最終状態では
`exp/surface.rs`（AC3）の `#[expect(unsafe_code)]` + SAFETY コメント付き
局所許可（計 12 箇所）に限定され、`poc/ui/Cargo.toml` の
`unsafe_code = "warn"` と `clippy -D warnings` で封じ込めを機械検証する。

## Follow-up — Issue #57 高負荷再計測（2026-11、Windows 11 x64・WebView2 155.0.4283.62・60Hz）

ワークロードを `--peaks`/`--meters`/`--preview` でスケールして再計測
（実装: `bench-shared` の `WorkloadGenerator` 引数化 + 720p `PreviewLayer`）。
**従来の 60fps/dropped=0 は基準 workload (~2,000点+8メータ) 限定の値であり SLA ではない**
ことが今回の計測で確定。証跡: `out/load-*.jsonl`・`docs/impl/exp/issue-57-high-load-webgpu-multi-webview.jsonl`。

| 判定 | 負荷 | backend | frame_time_ms avg/p99 | dropped | achieved_fps | work_time_ms avg/p99 |
| --- | --- | --- | --- | --- | --- | --- |
| **passed** | timeline 10k列/64メータ | canvas2d | 32.6 / 50.0 | 0 | **30.7** | 3.9 / 5.3 |
| **passed** | timeline 10k列/64メータ | webgl2 | 16.7 / 16.9 | 0 | 60.0 | 5.7 / 21.1 |
| **passed** | timeline 32k列/128メータ | canvas2d | 114.2 / 133.4 | 0 | **8.75** | 16.9 / 18.2 |
| **passed** | timeline 32k列/128メータ | webgl2 | 17.3 / 33.3 | 2 | 57.9 | 13.5 / 38.7 |
| **passed** | 基準 + 720p プレビュー | canvas2d | 16.7 / 16.8 | 0 | 60.0 | 1.1 / 3.1 |
| **passed** | 基準 + 720p プレビュー | webgl2 | 16.7 / 16.9 | 0 | 60.0 | 1.3 / 3.1 |
| **passed** | 32k列/128メータ + プレビュー | canvas2d | 114.9 / 133.4 | 0 | **8.70** | 12.5 / 15.7 |
| **passed** | 32k列/128メータ + プレビュー | webgl2 | 16.7 / 16.8 | 0 | 60.0 | 15.9 / 16.8 |

所見:

- **canvas2d は大規模タイムラインで破綻**: 10k 列で vsync 半分（30fps）、32k 列で 8.7fps。
  JS work_time 自体は小さく、パス描画のラスター側が支配（frame_time 114ms ≫ work 17ms）。
- **webgl2 は 32k 列 + プレビューでも 60fps 維持**、ただし work_time avg 15.9ms・p99 16.8ms
  で 16.7ms 予算の限界 — これ以上の列数や併用負荷ではドロップする見込み。
- 720p プレビュー単体の追加コストは ~0.6-0.8ms/frame。

### WebView2 内 WebGPU 有効化経路（#57）

- **根因確定**: `navigator_gpu=false` は WebView2 の機能不足ではなく、wry が Windows で
  `poc://app` → `http://poc.app` に書き換える**非セキュアコンテキスト**が原因
  （`navigator.gpu` は SecureContext 限定 API）。
- **検証済み**: `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--unsafely-treat-insecure-origin-as-secure=http://poc.app`
  で `navigator_gpu=true`・`webgpu_adapter=(nvidia/lovelace)`・`backend_used=webgpu`・60fps
  （`out/webgpu-secure-origin.jsonl`）。
- **本番経路**: browser flags は MS が「開発用途・本番出荷不可」と明記。本番では
  `SetVirtualHostNameToFolderMapping` による **https 仮想ホスト**等の真正セキュアオリジンで
  UI を配信すれば opt-in なしで WebGPU が使える見込み（macOS/Linux 実機確認は残課題）。

### wry PR #1762（ICoreWebView2CompositionController）

- 2026-11 時点 **open・mergeable_state=blocked**（最終更新 2026-07-22）。引き続き追跡。

### 再現

```sh
# タイムライン負荷
mise exec -- ui-poc --experiment render --backend webgl2 --frames 300 --peaks 32000 --meters 128
# プレビュー負荷
mise exec -- ui-poc --experiment render --backend canvas2d --frames 300 --preview
# WebGPU（セキュアオリジン扱い）
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--unsafely-treat-insecure-origin-as-secure=http://poc.app" \
  mise exec -- ui-poc --experiment render --backend webgpu --frames 120
```
