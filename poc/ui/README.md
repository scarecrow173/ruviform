# poc/ui — Issue #28 UI 技術検証 PoC ハーネス

`ui-poc` は Issue #28（UI 技術検証: wry・描画バックエンド・フロントエンド FW・配布）の
**捨て場付き検証ハーネス**である。本番ワークスペース（ルート `Cargo.toml` / `crates/`）から
独立した Cargo ワークスペースであり、製品コードへの依存混入は行わない。

- AC1 `--experiment host` … tao `Window` + wry `WebView` の基本検証（T1）
- AC2 `--experiment render` … WebView 内描画バックエンド計測（T2。canvas2d/webgl2/webgpu）
- AC3–AC7 の実験（surface / ipc / frontend / dist）は後続タスク T3–T5 の範囲
- 計測契約は `poc-metric.v1`（`status`/`reason`/exit code 規約）。集約結果は
  [`RESULTS.md`](./RESULTS.md)（コミット対象）へ、生サンプルは `out/*.jsonl`（gitignore）へ、
  過程の観察は `docs/impl/exp/*.jsonl` へ記録する（3 層の証跡）

## 前提（prerequisites）

- Windows 11 x64 対話デスクトップ（T1 の実機検証環境。他 OS は文書評価のみ）
- Rust toolchain: リポジトリ pin（mise `rust = "1.99.0"`）。cargo コマンドは
  **`mise exec --` 経由**で呼ぶこと
- WebView2 Runtime（Windows 11 では標準同梱。`--check` で `webview2_version` を検出）
- Node.js + npm（`frontend/` のビルドに使用。npm workspaces: `bench-shared`
  （FW 非依存のワークロード・rAF 統計・レンダラ・ipc ブリッジ）と
  `vanilla`（Vite + TypeScript 参照 UI）を `npm run build` で一括ビルドし、
  `build.mjs` が `dist/` を集約する。`solid` variant は T5 で追加）

## セットアップ

```sh
cd poc/ui/frontend
npm ci          # package-lock.json から復元（workspaces 全依存）
npm run build   # workspaces ビルド + dist/ 集約（.ui-poc-built マーカー配置）
cd ../..
```

`frontend/dist/` と `frontend/*/dist/` は gitignore 対象。未ビルド時は
`build.rs` がスタブ `index.html` を生成するため `cargo build` / `--check`
単体は常に動くが、実験は `status:"skipped"`（`reason: "frontend not built"`）
となる。`dist/index.html` は host 実験ページ、`dist/<variant>/index.html`
が各 FW バリアントのベンチ UI（render 実験は `poc://app/vanilla/index.html`
を使う）。

## ビルド・実行

```sh
# ビルド（リポジトリルートから）
mise exec -- cargo build --manifest-path poc/ui/Cargo.toml

# 環境 capability 検出のみ（GUI を作らない・常に exit 0）
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --check

# AC1 host 実験（ウィンドウが一瞬表示され、done/timeout で自己終了）
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment host

# AC2 render 実験（--backend は必須。既定 600 フレーム記録で自己終了）
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment render --backend canvas2d --frames 600
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment render --backend webgl2 --frames 600
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment render --backend webgpu --frames 600  # navigator.gpu 不在なら webgl2 fallback

# パラメータ指定例（タイムアウト秒・証跡出力先・フレーム数）
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
  --experiment host --duration 15 --out out/host-manual.jsonl --frames 300
```

## 計測契約（poc-metric.v1）

- 最終 stdout 行は常に `poc-metric.v1` JSON 1 行（診断はすべて stderr）
- 同じレコードを `out/<experiment>-<epoch>.jsonl`（既定）または `--out` 指定先へ追記
- `status` と exit code の対応:
  - `passed` → exit 0（全記録項目成立）
  - `skipped` → exit 0 + 非空 `reason`（前提不足: headless・WebView2 未検出・frontend 未ビルド）
  - `failed` → exit 1（実験失敗: タイムアウト・観測不能・検証不一致）
  - CLI / 内部エラー → exit 2（usage・未知実験名・証跡書き出し失敗等）

```text
{"schema":"poc-metric.v1","experiment":"host","ts":…,"env":{"os":…,"arch":…,
 "webview2_version":…,"wgpu_adapter":…,"rustc":…},"params":{…},
 "metrics":{…},"status":"passed"}
```

## 構成

```text
poc/ui/
  Cargo.toml          # 独立 [workspace]（ルート WS へは非参加・非参照）
  build.rs            # frontend/dist スタブ生成 + UI_POC_RUSTC 焼き込み
  src/
    main.rs           # dispatch + exit code 規約
    args.rs           # std-only CLI パース
    check.rs          # capability 検出（WebView2 reg / wgpu adapter / env）
    host.rs           # tao Window + wry WebView adapter（外部型はここに封じる）
    assets.rs         # rust-embed + poc://app（app 配信 / data 予約）
    metrics.rs        # poc-metric.v1 生成・stdout/out/*.jsonl 出力・集計
    exp/mod.rs        # Experiment trait + 実験レジストリ + 共通前提ゲーティング
    exp/host.rs       # AC1 実験（状態機械は Rust 側が所有）
    exp/render.rs     # AC2 実験（backend 別フレーム統計・webgpu→webgl2 fallback）
  frontend/
    package.json      # npm workspaces ルート（bench-shared + vanilla）
    package-lock.json # コミット対象（npm ci の正本）
    build.mjs         # dist/ 集約（src/ + <variant>/dist → dist/<variant>/）
    src/index.html    # host 実験用 JS エージェント（観測イベントを IPC 報告）
    bench-shared/     # FW 非依存: workload/rAF stats/renderers(canvas2d|webgl2|webgpu)/ipc/runner
    vanilla/          # Vite + TypeScript 参照ベンチ UI（FW ランタイムゼロ下限）
  RESULTS.md          # AC→実験→エビデンス行対応表（コミット対象）
  out/                # 生 JSONL（gitignore）
```

### 外部型の隔離境界

`wry`/`tao`/`winreg`/`http`/`dpi` 等の外部型は `host.rs`・`assets.rs`・`check.rs`
に限定する。`exp/*` と `metrics.rs` は内部 I/F（`Host`/`HostEvent`/`Environment`）と
`serde_json` のみに依存し、将来クレート分割を予行する（design 評価軸）。

## wgpu optional feature 化の逃げ道

`wgpu = "30"` は現状 `--check` の adapter 検出と T4 のネイティブサーフェス実験に使う
**必須依存**だが、GPU 非搭載/CI 環境でビルド負荷が問題になった場合は
`wgpu = { version = "30", optional = true }` + feature `gpu` で分離し、
`check.rs` の `detect_wgpu_adapter` と T4 の `surface` 実験を `#[cfg(feature)]`
で囲う設計とする（design「Open Questions」の採用案。T1 では常時有効のまま）。

## 既知の挙動メモ（T1 実機観測）

- Windows のカスタムプロトコル要求は `poc://app/index.html` → `http://poc.app/index.html`
  に書き換えられてハンドラへ届く（`assets.rs` が両形式を受理）
- WebView2 `controller.SetBounds` の JS viewport 反映は非同期で、同一値の再送が
  伝播しないケースを実測。host 実験は ±1px 交互ジッタ付きリトライで回避する
- バックグラウンド起動では OS foreground activation が得られない（`Focused` イベントなし）
  が、`focus_parent()`→`SetFocus(parent)`→wry 親 subclass `MoveFocus` の連鎖で
  DOM window blur→focus 遷移は観測可能（`focus_transition.mechanism:"window"`）
- `build_as_child` 経路では子コンテナ HWND リサイズが JS viewport に反映されない
  観測（T4/AC3 の隣接構成検証へ引継ぎ）
- WebView2 の IPC / ページロード callback は tao `WaitUntil` を起床させない
  （OS ウィンドウイベントではないため）。`host.rs` は `EventLoopProxy` の
  user event で起床を明示する。これがないとウィンドウイベント途絶区間の
  IPC メッセージ（done シグナル等）が deadline まで滞留する（T2 実測）
- WebView2 UDF はシングルトンロック持ち。連続実行で前回ブラウザ子プロセスの
  残留ロックが初期化をブロックするため、既定 data_dir は
  `ui-poc-webview2-<pid>` のプロセス固有パス
- `navigator.gpu` は WebView2 155.0.4283.62 で非公開（`--backend webgpu` は
  機能検出失敗 → `fallback_to` 記録付きで WebGL2 実測となる。
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` の enable 系 flags 2 試行でも非公開のまま）
