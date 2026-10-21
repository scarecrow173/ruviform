# poc/ui RESULTS — Issue #28 UI 技術検証 PoC 計測結果

採否 ADR（`ADR-034Jc9Cguud7nZ2Tt88Ptu`）が cite する集約エビデンスの正本。
生サンプルは `out/*.jsonl`（gitignore・再生成可能）、過程の観察は
`docs/impl/exp/*.jsonl`（impl-doc 規約）に記録する。

実機で未検証の項目は「**文書評価**」ラベルを必須とし、実測値として記載しない
（test-spec「実機未検証項目は文書評価と明示する」）。

## AC → 実験 → エビデンス行 対応表

| AC | 検証項目 | 実験 | 証跡行 | 状態 |
| --- | --- | --- | --- | --- |
| AC1 | WebView ホスト（wry+tao 起動・埋め込みロード・IPC・リサイズ・フォーカス） | `--experiment host` | [AC1 host](#ac1---experiment-host) | **実測** |
| AC2 | 描画バックエンド（canvas2d/webgl2/webgpu） | `--experiment render` | （T2 で記録予定） | 未実施 |
| AC3 | ネイティブサーフェス同居 | `--experiment surface` | （T4 で記録予定） | 未実施 |
| AC4 | メッセージ境界（IPC/プロトコル/pull） | `--experiment ipc` | （T3 で記録予定） | 未実施 |
| AC5 | フロントエンド FW + バンドラ比較 | `--experiment frontend` | （T5 で記録予定） | 未実施 |
| AC6 | 配布（portable zip / 依存整理） | `--experiment dist` | （T5 で記録予定） | 未実施 |
| AC7 | 採否 ADR 確定 | `poc-adr-finalize` | （全 AC 完了後） | 未実施 |
| AC8 | PoC 隔離（ルート無変更・独立 WS） | 構造検査 | [AC8 隔離](#ac8---ポック隔離) | **実測** |

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
```

## AC1 — `--experiment host`

| 判定 | 実験 | エビデンス（集約値） | 証跡 | 文書評価 |
| --- | --- | --- | --- | --- |
| **passed** | host | `protocol_load.ok=true`（`http://poc.app/index.html` 埋め込みロード + JS hello 確認）・`ipc_echo.ok=true`（RTT 255.7ms。初回 WebView2 ウォームアップ込みの実行では ~500-940ms）・`resize_follow.ok=true`（要求 `[947,631]` → RO 観測 `[947,631]`、±4px 許容、retries:4）・`focus_transition.ok=true`（`mechanism:"window"`、JS window blur→focus ペア `[false,true,false,true]`、API 記録 `set_focus:invoked`/`focus_parent:ok`/`focus:ok`）・`webview2_version=155.0.4283.62` | `out/host-1792554329.jsonl`・`docs/impl/exp/ac1-host-experiment-wry-tao-webview.jsonl` | 実測（Windows 11 x64） |

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
`rust-version = "1.85"` 宣言（Issue #34 で確定予定）と衝突しうる。本 PoC は
独立ワークスペース + pin 1.99.0 でビルドするため実害なし。ルート側へ
wgpu を導入する採否になった場合は宣言値の見直しタスクへ連携すること
（本 PoC では判断しない）。

## AC8 — PoC 隔離

| 判定 | 検査 | エビデンス | 文書評価 |
| --- | --- | --- | --- |
| **passed** | ルート無変更 + 独立ワークスペース | `poc/ui/Cargo.toml` は `[workspace]`+`[package]` の独立 WS（ルート `members` は明示リストで `poc/` を含まないため `exclude` 不要・吸収されない）。`git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml` が無出力。`mise run check`/`lint`/`test` が本変更前後で同一に成功（検証ログは `docs/impl/ir/poc-ui.md`） | 実測 |

### 依存の境界

wry 0.57 / tao 0.37 / wgpu 30 / rust-embed 8 / serde_json / http / dpi /
pollster / winreg（Windows のみ）は `poc/ui` ワークスペース内に限定し、
ルート `Cargo.toml`/`Cargo.lock`/`crates/` へは一切追加しない
（`poc/ui/Cargo.lock` はこのワークスペース内で生成・コミット対象）。
`unsafe`・windows-rs 直接呼出は T1 では不使用。
