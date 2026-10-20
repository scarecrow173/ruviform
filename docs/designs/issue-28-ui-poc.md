---
id: "DESIGN-034JcpvK9cZEfkeEbBLBoz"
type: "design"
status: "approved"
title: "Issue #28 UI 技術検証 PoC の設計（使い捨て検証ハーネスと計測・評価方式）"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/28"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-28-ui-poc.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/issue-28-ui-poc.md"
    - "../adr/ui-architecture-webview-hybrid.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-28-ui-poc.md"
    - "../adr/ui-tech-selection-poc.md"
  derived-by:
    - "../plans/issue-28-ui-poc.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/28"
    - "https://docs.rs/wry/latest/wry/"
    - "https://crates.io/crates/tao"
    - "https://crates.io/crates/wgpu"
    - "https://github.com/tauri-apps/wry/pull/1762"
    - "https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution"
  defers: []
  deferred-by: []
---

# Issue #28 UI 技術検証 PoC の設計（使い捨て検証ハーネスと計測・評価方式）

## Context

[Issue #28 PoC spec](../specs/issue-28-ui-poc.md)（approved）は「使い捨て PoC 検証ハーネス（Rust + Web フロントエンド）」と「[採否 ADR](../adr/ui-tech-selection-poc.md) の各条項を裏付けるエビデンス」の 2 成果物を要求し、AC1〜AC8 を受け入れ基準として定義した。[採否 ADR](../adr/ui-tech-selection-poc.md)（`proposed`）は全採否条項を `要 PoC 確証` と宣言しており、本設計は「どう検証し、どう計測し、どこへ記録するか」を確定して計画段階の入力とする。

設計上確定が必要だった論点と、本設計の結論:

- **配置・依存隔離**: `poc/ui/` を独立 cargo ワークスペースとする。ルート `Cargo.toml` の `members` は `crates/...` の明示リスト（グロブなし）であり `poc/ui` は吸収されない。`poc/ui/Cargo.toml` に `[workspace]` テーブルを置くことで poc 自身がワークスペースルートになり、親ワークスペースへの遡及も発生しない。ルート側への `exclude` 追加は不要（明示 `members` で吸収不能・ルート `Cargo.toml` 非改変の方が spec の「既存資産の非改変」に適合）。wry/tao/wgpu/フロントエンド依存・独自 `Cargo.lock`・`target/`・lint 方針がルートワークスペースと完全に分離される。
- **ハーネス形態**: 単一バイナリ `ui-poc` + `--experiment` フラグ方式。実験別 bin 分割はホスト起動コードの重複または共有 lib クレートを要し、使い捨てには過剰。
- **自己終了**: 全実験は「N フレーム完了」または「タイムアウト」で計測結果を JSON として出力して終了する。GUI ウィンドウは一瞬だけ出る構成であり、CLI/自動化コンテキストで完結する。
- **エビデンス記録**: 生サンプルは `poc/ui/out/`（gitignore・再生成可能）、集約済み数値は `poc/ui/RESULTS.md`（コミット対象・採否 ADR の cite 先）、実施過程の観察は `docs/impl/exp/*.jsonl`（impl-doc 規約）。

## Scope

### In Scope

- `poc/ui/` 独立ワークスペースの構成（`[workspace]` + bin `ui-poc` + 独自 `Cargo.lock`/`target/`/`.gitignore`）
- `ui-poc` の CLI 契約（`--experiment`・`--frames`・`--check`・`--out`・環境ゲーティング終了コード）
- AC1〜AC6 に対応する 6 系統の実験設計と計測契約（記録する数値・単位・永続化先）
- フロントエンドハーネス（`poc/ui/frontend/`、Vite、Solid + vanilla TS の 2 候補）とアセット埋め込み配線
- AC3 の隣接サーフェス実験の具体構成（wgpu 描画領域 + WebView 子ビュー、イベント/z-order/リサイズ検査）
- エビデンスの永続化方式（`RESULTS.md` + `docs/impl/exp/*.jsonl` + `out/` 生データ）と AC7 への接続
- 実装タスク分解案（plan 段階の入力）

### Out of Scope

- `ruviform-ui` / `ruviform-ui-webview` / `ruviform-gpu` の実装（将来クレート。本設計は分割を意識した構造で書くが、クレート自体は作らない）
- ルート `Cargo.toml`（`members`/`exclude`/`workspace.lints`）・`crates/` 配下・ルート `Cargo.lock` の変更
- ルート `mise.toml` へのタスク追加（PoC は `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml` で起動する）
- macOS / Linux の実機検証（文書評価のみ）
- wry PR #1762 適用 fork の常設検証経路（採否材料の文書評価が既定。fork spike は隣接構成が早期完了した場合のオプション扱い）
- 性能 SLA の設定、`rust-version = "1.85"` 引き上げ判断（事実記録と専用タスク連携のみ）
- プラグイン GUI 統合（`wxp`/`baseview` 経路は参照評価に留める）

## Components and Boundaries

### ディレクトリ構成

```text
poc/ui/
  Cargo.toml            # [workspace] + [package] ui-poc（bin）。親 WS に吸収されない独立ルート
  Cargo.lock            # PoC 専用 lockfile（コミット対象。再現性確保）
  .gitignore            # node_modules/・*/dist/・out/（target/ はルート .gitignore が網羅）
  README.md             # 目的・前提・起動手順（コミット対象）
  RESULTS.md            # 計測結果テーブル + 環境 + 再現手順 + AC対応表（コミット対象。採否 ADR の cite 先）
  src/
    main.rs             # CLI エントリ・実験ディスパッチ・終了コード規約
    args.rs             # CLI パース（std のみ。clap 等の引数クレートは入れない）
    check.rs            # 能力検出: WebView2 (レジストリ)・wgpu adapter・OS/toolchain
    host.rs             # tao Window + wry WebView 起動・自己終了・イベント配線（全実験共用）
    assets.rs           # rust-embed + with_custom_protocol 応答（poc:// スキーム）
    metrics.rs          # 計測 JSON スキーマ・percentile 集計・stdout/out 書き出し
    exp/
      mod.rs            # 実験レジストリ（Experiment trait: setup/run/teardown）
      render.rs         # AC2: Canvas2D/WebGL2/WebGPU フレーム計測
      ipc.rs            # AC4: 3 経路メッセージ境界計測
      surface.rs        # AC3: wgpu ネイティブサーフェス同居・合成評価
      dist.rs           # AC6: WebView2 検出・バイナリ情報・配布チェック
  frontend/
    package.json        # npm workspaces（bench-shared + vanilla + solid）
    package-lock.json   # コミット対象（フロント側の再現性）
    bench-shared/       # 共有ロジック: ワークロード生成・rAF 統計・ipc ブリッジ・fetch 計測
    vanilla/            # Vite + TypeScript 参照実装（FW ゼロの下限）
    solid/              # Vite + vite-plugin-solid 実装（第一候補）
  out/                  # 生サンプル JSONL（gitignore。再生成可能）
```

### コンポーネント責務

| コンポーネント | 責務 | 対応 AC |
| --- | --- | --- |
| `main.rs`/`args.rs` | CLI パース・実験選択・終了コード | 全般 |
| `check.rs` | GUI 不要の能力検出（WebView2 バージョン・wgpu adapter 列挙・OS/arch・rustc） | AC1-AC6 の前提・ゲーティング |
| `host.rs` | tao イベントループ・wry WebView 生成・自己終了タイマ・実験完了シグナル処理 | AC1 |
| `assets.rs` | 埋め込みアセットのカスタムプロトコル配信（debug は FS 読み・release は埋め込み） | AC1/AC6 |
| `metrics.rs` | `poc-metric.v1` JSON スキーマ・集計・永続化 | AC2-AC6 |
| `exp/*` | 実験本体。ホスト上でフロントエンド workload を駆動し、結果を `metrics.rs` へ渡す | AC2-AC6 |
| `frontend/bench-shared` | フレーム計測・ワークロード・プロトコル/fetch/IPC ブリッジを FW 非依存で提供 | AC2/AC4/AC5 |
| `frontend/vanilla`/`solid` | 同一ベンチマーク UI の FW 別実装 | AC2/AC5 |

### 将来クレート分割の予行

PoC はクレートを作らないが、モジュール境界を baseline の `ruviform-ui`（ポート定義）/ `ruviform-ui-webview`（adapter）分割に対応させて書く:

- `host.rs`/`assets.rs`/`check.rs` が wry・tao・winreg・windows-rs 等の**外部型を握る adapter 側相当**。`exp/*` と `metrics.rs` は wry 型を直接扱わず、ホストが提供する最小の PoC 内部 I/F（ウィンドウライフサイクル・プロトコル応答・イベント購読・計測書き出し）に依存する。
- wgpu は `exp/surface.rs` の内部に閉じる（`ruviform-gpu` adapter 想定の予行）。
- 「外部型を境界の背後に置いたまま PoC が書けたか」は採否 ADR の評価軸として `RESULTS.md` に所見を記録する（spec Requirements「外部型の隔離」）。

## Data and Control Flow

### 起動〜計測〜終了（全実験共通）

```text
mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- \
    --experiment <name> [--frames N | --duration SEC] [--backend ...] [--out out/x.jsonl]

  → args パース → check.rs で前提能力検出
      ├─ 不足 → {"status":"skipped","reason":...} を stdout に出し exit 0
      └─ 充足 → host.rs が tao Window + wry WebView を生成
                → 実験モジュールが workload を開始
                → JS 側が N フレーム/条件完了で window.ipc.postMessage({"type":"done","stats":{...}})
                → ipc_handler が done を受理 → metrics.rs が環境情報と合成
                → JSON 要約を stdout 最終行 + out/*.jsonl に追記 → exit(0)
```

### Rust ⇄ フロントエンド経路（AC4 の計測対象そのもの）

| 経路 | 方向 | wry 機構 | PoC での用途 |
| --- | --- | --- | --- |
| JSON intent | JS → Rust | `window.ipc.postMessage` → `with_ipc_handler` | UI 操作 intent・計測完了通知・往復遅延の片道 |
| 差分 push | Rust → JS | `evaluate_script` | 状態差分・往復遅延の復路・push スループット |
| バイナリ pull | JS → Rust | `fetch("poc://data/...")` → `with_custom_protocol` の `Response<Cow<[u8]>>` | 高頻度描画データ相当の pull 型転送 |

アセット配信は同じ `with_custom_protocol` の別名前空間（`poc://app/...` = 埋め込み静的アセット、`poc://data/...` = 計測用バイナリ応答）で行う。`rust-embed` は release ビルドで埋め込み・debug で FS 読みにして、フロントエンド反復時に Rust リビルドを不要にする。

## 実験設計（AC 別）と計測契約

### 共通計測契約

- 全実験は終了時に 1 行の JSON を **stdout 最終行**に出力し、同内容を `--out`（既定 `poc/ui/out/<experiment>-<timestamp>.jsonl`）へ追記する。
- スキーマ `poc-metric.v1`:
  `{schema, experiment, ts, env:{os, arch, webview2_version, wgpu_adapter, rustc}, params:{frames, backend, ...}, metrics:{...}, status:"passed"|"skipped"|"failed", reason?}`
- `env` は `check.rs` が 1 回だけ検出する。結果の集約表は `poc/ui/RESULTS.md` に手で転記する（採否 ADR は `RESULTS.md` の行と `docs/impl/exp/*.jsonl` を cite する）。
- 観察・失敗・判断の経過は impl-doc 規約の Experiment Log（`docs/impl/exp/*.jsonl`、`append_experiment_event` で追記）に記録し、タスクごとの Implementation Record から参照する。

### AC1 — WebView ホスト（`--experiment host`）

- 実施: tao Window + wry `build_as_child`（または `build` + bounds）で WebView を生成。`with_custom_protocol` が `poc://app/index.html` を `rust-embed` 埋め込みから返す（dev server 不使用を証明）。JS から `window.ipc.postMessage` の JSON intent → `with_ipc_handler` で受理し `evaluate_script` で echo 返却。リサイズイベント追従（`set_bounds`）・フォーカス遷移をイベントログに記録。
- 記録: `protocol_load`（埋め込みロード可否）、`ipc_echo`（往復可否・ms）、`resize_follow`、`focus_transition`（各 boolean/観察）、`webview2_version`。
- 判定: 全項目 pass で AC1 充足。

### AC2 — WebView 内描画（`--experiment render --backend canvas2d|webgl2|webgpu [--frames 600]`）

- ワークロード: スクロール波形（ピーク列・フレーム毎 2,000 点規模）+ レベルメータ群を 60fps 目標で更新。`bench-shared` が同一データ生成・同一画面を 3 バックエンドで描画する（vanilla 実装を参照ハーネスとする）。
- 計測: JS 側 rAF タイムスタンプ列から `frame_time_ms` の avg/p50/p95/p99・dropped（> 2 vsync 間隔）・achieved_fps を算出し done メッセージで返送。N フレーム完了で自己終了。
- WebGPU: `navigator.gpu` 存在 + adapter request の機能検出を必須とし、失敗時は WebGL2 へフォールバックして `status:"passed"` + `metrics.fallback_to` に理由を記録する（「フォールバック込み評価」の spec 要件）。
- 判定: バックエンド別の数値行が `RESULTS.md` に記録されること（優劣の採否は ADR が行う）。

### AC3 — ネイティブサーフェス合成（`--experiment surface --mode adjacent-parent|adjacent-child`）

- 主構成 `adjacent-parent`: **親ウィンドウ（tao）自身の HWND を wgpu の Surface にし**、WebView は `build_as_child` で左側サブ領域に配置する。右側の親クライアント領域に wgpu 三角/波形描画が見える状態を作る。子 HWND は親描画の上に来るため、追加 HWND なしで「隣接同居」を構成でき、`unsafe` を伴う自前子 HWND 生成を主経路から外せる。
- 変形 `adjacent-child`: 兄弟子 HWND（`windows` crate で `WS_CHILD` 生成 + `raw-window-handle` 実装）へ wgpu を張り、兄弟 z-order・重なり（WebView 領域への一時的なオーバーラップ配置）を検査する。**`unsafe` は windows-rs 呼び出し箇所の SAFETY コメント付き局所許可**に留める（PoC でも暫定規約どおり）。DOM overlay がネイティブ面に被るケースでは `SetWindowRgn` クリップの回避策を試し、挙動を記録する。
- 検査項目（各 pass/fail + 観察を記録）: ヒットテスト（各領域のクリックが正しい層に届く）・フォーカス遷移・z-order（兄弟 HWND 間・WebView 対ネイティブ）・リサイズ追従（`set_bounds` + `SetWindowPos`/`surface.configure`）・wgpu フレーム描画の継続性。
- DOM 内合成の扱い: windowed WebView2 では子 HWND が同矩形のネイティブコンテンツより常に上、かつ真の下層合成は `ICoreWebView2CompositionController` + DirectComposition が要り wry 未対応（未 merge PR [#1762](https://github.com/tauri-apps/wry/pull/1762)）である事実を、discovery の loomux/`SetWindowRgn` 実例とともに `RESULTS.md` に記録する。**fork spike は既定スコープ外**（隣接構成が予定より早期に完了し `[patch.crates-io]` 適用が自明に小さい場合のみオプション実施。実施する場合も結果は「探索的付記」として本編と分離する）。macOS（layer-backed `NSView`/`CAMetalLayer` sibling 合成）・Linux（真の下層合成は実質不可寄り、隣接/Canvas 収束）は文書評価として整理する。
- 判定: 検査項目マトリクスと制限事実が `RESULTS.md` に記録されること。

### AC4 — メッセージ境界（`--experiment ipc [--duration 10]`）

- JSON intent 往復: JS が `{seq,t0}` を postMessage → Rust が受理して `evaluate_script` で seq を返送 → JS が `t1-t0` を計測。ペイロード 64B/1KiB/16KiB/64KiB × 所定回数で `rtt_ms` の avg/p50/p99 と単方向 ingest レート（msg/s）を記録。
- バイナリ pull: JS が `fetch("poc://data/blob?bytes=N")` を繰り返し、`ArrayBuffer` 受領までの時間からスループット（MB/s）と要求毎遅延を 64KiB/1MiB/16MiB で記録。サーバ側は合成バッファ応答（`Cow`）。
- push 経路: Rust 側が `evaluate_script` で固定レート送信し、JS 側の受信間隔・取りこぼし・coalesce 挙動を記録。
- 記録: `ipc_rtt_ms{avg,p50,p99}`、`ipc_ingest_msg_s`、`protocol_pull_MBps`、`push_msg_s`、coalesce 観察。3 経路併用の実現性所見を `RESULTS.md` に記す。

### AC5 — フロントエンド比較（`--experiment frontend --variant vanilla|solid`）

- 候補 2 本は **vanilla TS（参照・下限）と Solid**（第一候補）で固定する。理由: AC5 は「少なくとも 2 候補」であり、runtime ゼロの vanilla と fine-grained signals の Solid は設計空間の両端として最も情報量が大きい。Svelte 5 は discovery/ADR の次点候補だが、2 本で結論が出た段階で追加しない（YAGNI。必要なら後続追加は構成上容易）。
- 同一ベンチマーク UI（AC2 の workload + intent/diff 表示部品）を両者で構築し、`bench-shared` を共用して描画負荷を揃える。
- 記録: `dist` 合計バイト数（raw / gzip -9、再現コマンド併記）、Vite ビルド時間、グルーコード LOC、更新粒度（1 メータ更新当たりの DOM 変更箇所数・Solid の fine-grained 更新が部分更新に留まるかの観察）、開発ワークフロー所見（ビルド → 埋め込み配線の手数）。
- 判定: 2 候補の比較行が `RESULTS.md` に記録されること。

### AC6 — 配布（`--experiment dist` + release ビルド手順）

- 単一バイナリ: `cargo build --release --manifest-path poc/ui/Cargo.toml` の産物が、埋め込みアセットのみで起動・全 `--check` 項目を報告することを実機で検証。`dist` 実験は WebView2 レジストリ検出（`HKLM/HKCU\...\Clients\{F3017226-...}` の `pv`、winreg 経由）・バイナリサイズ・検出失敗時の bootstrapper 手順案内出力を行う。
- パッケージング hands-on: **portable zip**（`ui-poc.exe` 単体で完結するため最小の実機検証として採用。NSIS/WiX は参照評価に留め spec が要求する「いずれか」を充足）。macOS（`.app`+署名/公証）・Linux（`libwebkit2gtk-4.1`+GTK3 実行時依存、deb 依存・glibc ベースライン・AppImage 方式）は文書評価として `RESULTS.md` に整理し、Linux 実行時依存リスクは依存パッケージ一覧・API 版（4.1/soup3 が唯一の現実経路）を定量表で記録する。
- 判定: Windows 実機の hands-on 行 + macOS/Linux 文書評価行が記録されること。

### AC7 — 採否 ADR への接続

- `RESULTS.md` 冒頭に「AC → 実験 → エビデンス行」の対応表を置き、採否 ADR の各条項が cite するリンク先を一意にする。文書評価の行は「文書評価」ラベルを必須とする（実機未検証の誤記防止）。
- エビデンスが揃った段階で採否 ADR の条項を confirm/revise し `accepted` へ遷移させるのは後続タスクの作業であり、本設計は記録フォーマットと対応表の提供までを責務とする。

### AC8 — 使い捨て検証の隔離

- `poc/ui/` は `[workspace]` 持ちの独立ワークスペースであり、ルート `members`（明示リスト）・`workspace.lints`・`rust-version = "1.85"`・ルート `Cargo.lock` のいずれにも触れない。wgpu MSRV 1.87 は `poc/ui` 側では問題にならない（pin toolchain 1.99.0 でビルドし、宣言値衝突の事実は `RESULTS.md` と MSRV 見直しタスクへの連携メモとして記録する）。
- 検証として: ルートで `mise run check`/`lint`/`test` が PoC 追加前後で同一に通ること、`git status` でルート `Cargo.toml`/`Cargo.lock`/`crates/` が無変更であることを確認する。

## 環境ゲーティング・自動化での振る舞い

- **`--check`**: ウィンドウを生成せず能力検出のみを JSON 出力して exit 0（headless でも常に動く診断入口）。
- **実験実行の前提不足**（WebView2 不在・`UI_POC_HEADLESS=1`・表示環境なし等）: `{status:"skipped", reason}` を出して **exit 0**。失敗と skip を exit code でなく構造化 `status` で分けることで、スクリプト/エージェントは「実施したか・スキップ理由」を一意に解釈できる。`status:"failed"` のみ exit 1、CLI/内部エラーは exit 2。
- **pass/skip の判定規約**: 検証側は `status` が `passed` か `skipped`+`reason` 非空であることを確認する。CI 等の非 GUI 環境で本 PoC を走らせる場合は `--check` のみを呼ぶ（実験は実機検証が前提であり、CI では計測ゲートを設けない）。
- **フロントエンド前提**: `node`/`npm` が無い環境では既コミットの `dist/` を使わずビルド不能 → 該当実験は `skipped`（理由: frontend not built）。`dist/` は gitignore なので原則 `npm ci && npm run build` を前置する。

## 非機能・規約上の制約

- **ツールチェーン**: mise pin `rust = "1.99.0"`（`mise exec -- cargo ...` 起動を正本手順とする）。Node 26/npm 11 は discovery 確認済み環境を前提とし `package-lock.json` をコミットする。
- **`unsafe`**: PoC でも暫定規約を踏襲し、windows-rs 等の呼び出し箇所のみ SAFETY コメント付き局所許可とする。
- **RT 契約整合**: UI スレッド ↔（将来の）エンジン間は非ブロッキング経路のみという方針を PoC でも破らない。本 PoC に RT スレッドは存在しないが、「UI 側は全て非 RT コンテキストとして書く」ことを構造上の前提とする。
- **依存追加**: wry/tao/wgpu/rust-embed/winreg/windows/serde_json・Vite/solid-js 等は PoC ワークスペース・frontend 側に限定し、ライセンス（MIT/Apache-2.0 系。discovery で確認済みのものを採用）と追加理由を `RESULTS.md` または PR に記録する。`poc/ui/Cargo.lock`・`package-lock.json` はコミットする。
- **gitignore**: ルート `.gitignore` は非改変（`target/` パターンが既に `poc/ui/target/` を網羅）。`poc/ui/.gitignore` に `node_modules/`・`*/dist/`・`out/` を置き、PoC 内で ignore 規則を完結させる。`dist/` はコミットせずビルドで再生成する。

## Risks and Trade-offs

| リスク | 影響 | 緩和 |
| --- | --- | --- |
| wry/tao/wgpu の初回ビルドが重い（WebView2・gtk 系・wgpu 依存木） | PoC 反復コスト増 | `poc/ui` 専用 `target/` でキャッシュ分離。wgpu を optional feature 化する逃げ道を README に明記（初期は KISS で常時依存） |
| GUI/GPU を要する実験がエージェント/headless 環境で動かない | 自動実行時の誤失敗 | `--check` と `status:"skipped"` 規約で明示 skip。実機検証はオペレータ実施を前提に RESULTS.md へ再現手順を残す |
| wry PR #1762 の帰趨が不透明（DOM 内合成の Windows 実現可否） | AC3 の結論深度 | 隣接構成を主経路とし、DOM 内合成は「未 merge 上流エビデンス」の文書記録を既定成果とする。fork はオプション spike に隔離 |
| wgpu MSRV 1.87 vs 宣言 `rust-version = "1.85"` | 将来クレートの宣言値整合 | PoC は独立 WS で pin 1.99.0 ビルド。衝突事実を `RESULTS.md` に記録し MSRV 見直しタスクへ連携（本 PoC では引き上げ判断しない） |
| フロントエンド成果物の取り扱い（dist をコミットするか） | 再現性 vs 差分ノイズ | `package-lock.json` コミット + `dist/` gitignore + README に build 手順。欠落時は実験が `skipped` を返す設計で誤検証を防ぐ |
| 計測の客観性（エージェント実施の数値を信用してよいか） | AC7 のエビデンス品質 | `poc-metric.v1` に env 同梱・生サンプルを `out/` に残し再生成可能にする。`RESULTS.md` は再現コマンド併記必須 |
| 単一バイナリ化で実験間の相互干渉（wry の WebView は 1 イベントループ） | 設計複雑化 | 実験は実行毎にプロセスを分ける（`--experiment` は起動時選択。同一プロセス内の逐次実行は持たせない） |

## タスク分解案（plan 段階への入力）

各タスクは「実装 → エビデンス → `RESULTS.md`/exp 記録更新」で閉じる。依存: T1 が全実験の前提。T2・T3・T4 は T1 後に並行可能。T5 は T2 のベンチ UI を再利用するため T2 依存。T6 は全実験完了後。

| # | タスク（案） | 内容 | 依存 | 対応 AC |
| --- | --- | --- | --- | --- |
| T1 | poc-ui-harness-host | `poc/ui` 独立 WS・`ui-poc` bin・CLI/`--check`・host（wry+tao+埋め込み protocol+IPC echo）・自己終了・metrics スキーマ・`.gitignore`/`README`/`RESULTS.md` 骨格 | — | AC1, AC8 |
| T2 | poc-render-backends | `bench-shared` + vanilla 参照 UI + canvas2d/webgl2/webgpu 実験 + フレーム統計記録 | T1 | AC2 |
| T3 | poc-message-boundary | JSON 往復・バイナリ pull・`evaluate_script` push の 3 経路計測 | T1（T2 と並行可） | AC4 |
| T4 | poc-native-surface | wgpu 隣接サーフェス（parent/child 両モード）・イベント/z-order/リサイズ検査・DOM 内合成の文書記録（+ 任意 fork spike） | T1 | AC3 |
| T5 | poc-frontend-and-dist | Solid 版ベンチ UI + 2 候補比較（AC5）。release 単一バイナリ・WebView2 検出・portable zip・macOS/Linux 文書評価（AC6） | T1, T2 | AC5, AC6 |
| T6 | poc-adr-finalize | エビデンス集約 → 採否 ADR の confirm/revise → `accepted` 遷移・文書評価ラベル確認 | T2〜T5 | AC7 |

## Deferred Design Concerns

| 論点 | 本設計での扱い | 移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `ruviform-ui`/`ruviform-ui-webview`/`ruviform-gpu` の実装設計 | モジュール境界で予行するのみ | 採否 ADR 後の実装 Issue | 検証と設計の混同で PoC が恒久化してしまう |
| wry PR #1762 fork の常設評価・保守 | 文書記録が既定、任意 spike のみ | 採否 ADR・必要時フォーク保守 Issue | Windows DOM 内合成が未 merge のまま固定化 |
| フロントエンドビルドの Cargo/mise 統合 | 手動 `npm run build` + `cargo run` の最小配線 | `ruviform-ui-webview` 実装 Issue | ビルド経路未整備のまま製品実装へ進む |
| macOS/Linux の実機検証 | 文書評価のみ | 採否 ADR・後続 Issue | 合成限界・配布リスクが実測なしで残る |
| `rust-version = "1.85"` 引き上げ | 事実記録のみ | 既存 MSRV 見直しタスク | wgpu 採用時に宣言値と検証ゲートが衝突 |
| 性能 SLA の確定 | 計測記録のみ | UI 実装 Issue | 計測値と要件の乖離が未管理 |

## Open Questions

- `adjacent-child` モードで兄弟 HWND を最小コードで構成する際、`windows` crate 直接利用以外に wry 側 API（`build_as_child` の別子ビューや `WebViewExtWindows::reparent`）で代替可能か — 実装時に最小構成を選ぶ。
- `evaluate_script` の高頻度送信で wry/WebView2 側が暗黙に coalesce/ドロップする挙動があるか（AC4 の観察項目として扱い、設計上は仕様としない）。
- WebView2 のデータディレクトリ（`WebContext`）を PoC 既定の一時 dir に置くかユーザー既定にするか — 実装時に決定（再現性のため既定は一時 dir 推奨）。
- 実機で `SetWindowRgn` クリップが DOM overlay 回避として実用になるか（AC3 の観察項目）。

## Verification Notes

- [x] ルート `Cargo.toml` の `members` が明示リスト（`crates/...` 6 件・グロブなし）であることを確認 — `poc/ui` が吸収されず `exclude` 不要と判断した
- [x] ルート `.gitignore` の `target/` パターンが任意階層の `target/` を網羅することを確認
- [ ] plan 段階でタスク分解案 T1〜T6 を task-doc に展開し、各タスクの検証入口（コマンド・`status` 判定）を確定する
- [ ] `poc-metric.v1` スキーマと `RESULTS.md` の対応表が採否 ADR の cite 先として機能するか、T1 完了時にレビューする
