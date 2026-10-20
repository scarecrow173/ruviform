---
id: "TSPEC-034JfAS5pKzR24eQWPln8d"
type: "test-spec"
status: "approved"
title: "Issue #28 UI PoC 検証仕様"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../specs/issue-28-ui-poc.md"
    - "../designs/issue-28-ui-poc.md"
    - "../adr/ui-tech-selection-poc.md"
    - "../plans/issue-28-ui-poc.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "../specs/issue-28-ui-poc.md"
    - "../adr/ui-tech-selection-poc.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #28 UI PoC 検証仕様

## Purpose

[spec](../specs/issue-28-ui-poc.md)（`SPEC-034Jc9BJlI7ULEBWoIlMP2`）の受け入れ基準 AC1–AC8 に対し、「どの実験が・どの構造化エビデンスを・どの判定規則で PASS / FAIL / SKIP とするか」を定義する正本。[design](../designs/issue-28-ui-poc.md) が定めた計測契約（`poc-metric.v1` JSON・自己終了・`--check`・exit code・`status:"skipped"` セマンティクス）と証跡の置き場（`poc/ui/RESULTS.md`・`poc/ui/out/*.jsonl`・`docs/impl/exp/*.jsonl`）を、実装の改変やタスク分割の変更を越えて守るべき契約として固定する。

本 PoC は使い捨てハーネスであり、計測値を「性能 SLA」としてではなく「記録要件」として扱う（達成すべき閾値ではなく、環境・再現手順付きで数値が記録されることが合格条件）。廃止してよいのは、PoC が完了して採否 ADR が `accepted` に遷移し `poc/ui/` が撤去されるとき、または spec / design の計測契約自体が改訂されたとき。

## Feature

`ui-poc` 単一バイナリ + `--experiment` フラグで駆動される 6 系統の実験（host / render / surface / ipc / frontend / dist）が、AC1–AC8 を判定可能な構造化エビデンスを産出し、そのエビデンスが採否 ADR（`ADR-034Jc9Cguud7nZ2Tt88Ptu`）の全採否条項の confirm / revise に使えること。

## Rules

### 計測・証跡の共通契約（全 AC に適用）

- **`poc-metric.v1` JSON を最終 stdout 行に出力する**。全実験は終了時に `{schema:"poc-metric.v1", experiment, ts, env:{os, arch, webview2_version, wgpu_adapter, rustc}, params, metrics, status, reason?}` の 1 行 JSON を stdout の最終行に出し、`--out`（既定 `poc/ui/out/<experiment>-<timestamp>.jsonl`）へも追記する。`env` は `check.rs` が 1 回だけ検出した値を持つ。最終行がこのスキーマに適合しない実験は証跡として不合格。
- **`status` と exit code のセマンティクスを分離する**。`status:"passed"` → exit 0、`status:"skipped"`（前提不足: WebView2 不在・`UI_POC_HEADLESS=1`・表示なし・frontend 未ビルド等）→ **exit 0 + 非空の `reason`**、`status:"failed"` → exit 1、CLI/内部エラー → exit 2。「実行されたか」を exit code ではなく構造化 `status` で判定できることが契約。`skipped` に `reason` がない、または `passed`/`failed` 以外で exit 0 以外を返す実装は不合格。
- **`--check` は GUI を作らず exit 0 で capability 検出 JSON を出す**。WebView2 検出・wgpu アダプタ・OS/toolchain を JSON で出力し、ヘッドレス環境でも常に動く唯一の入口であること（CI が呼ぶのはこの入口のみ）。
- **全実験が自己終了する**。「N フレーム完了」または「timeout」で JSON を出して exit する。オペレータの GUI 操作を無限に待つ実験は契約違反。
- **数値は「測定記録要件」であって SLA ではない**。各メトリックは「環境情報（`env`）+ 再現コマンド付きの数値」として記録されることが合格条件。メトリック欠落・非数値は「閾値未達」ではなく証跡不合格。達成値の良否の判定は採否 ADR の責務であり本仕様は数値閾値を設けない。
- **実機未検証項目は「文書評価」と明示する**。macOS / Linux の実挙動・wry PR #1762 依存の構成など、実機で測れない項目を実測値として記録してはならない。`RESULTS.md` の当該行には「文書評価」ラベルを必須とする。
- **証跡は 3 層で残す**。集約数値 → `poc/ui/RESULTS.md`（コミット対象・ADR の引用先）、生サンプル → `poc/ui/out/*.jsonl`（gitignore・再生成可能）、過程の観察 → `docs/impl/exp/*.jsonl`（impl-doc 規約）。`RESULTS.md` には「AC → 実験 → エビデンス行」対応表と再現コマンドを含める。
- **隔離を維持する（AC8）**。`poc/ui/` は `[workspace]` を持つ独立 cargo ワークスペースであり、ルート `Cargo.toml`（明示 `members` リスト）・ルート `Cargo.lock`・`crates/`・`workspace.lints`・`mise.toml` を変更しない。PoC 追加の前後でルート `mise run check` / `lint` / `test` が同一に成功し、`git status` でルート側に差分がないことが合格条件。

### AC1 — WebView ホスト（`--experiment host`）

- `tao` ウィンドウ + `wry` WebView が起動し、`with_custom_protocol` 経由で `poc://app/...` の**埋め込みアセット**から HTML/JS がロードされる（実行時に dev server 不要）。JS の `window.ipc.postMessage` JSON intent が `with_ipc_handler` の Rust ハンドラで受信・`evaluate_script` でエコー返送される。リサイズ追従（`set_bounds`）とフォーカス遷移が動作する。
- 記録メトリック: `protocol_load`・`ipc_echo`（roundtrip 成立 + ms）・`resize_follow`・`focus_transition`（各 boolean/観察値）・`webview2_version`。全項目成立で `passed`。

### AC2 — WebView 内描画バックエンド（`--experiment render --backend canvas2d|webgl2|webgpu`）

- 同一ワークロード（スクロール波形・ピーク列 約 2,000 点/フレーム + レベルメータ群、目安 60fps）を bench-shared が各バックエンドに供給する。フレーム統計 `frame_time_ms` の `avg/p50/p95/p99`・`dropped`（2 vsync 間隔超過）・`achieved_fps` を **既定 600 フレーム以上**（短縮する場合は `params.frames` に実フレーム数を記録）でバックエンド別に数値記録する。
- WebGPU は `navigator.gpu` + adapter 要求の機能検出を行い、失敗時は WebGL2 へフォールバックして `status:"passed"` かつ `metrics.fallback_to` に理由を記録する。フォールバック自体を失敗としない（spec の「フォールバック込みで評価」要件）。
- `RESULTS.md` にバックエンド別の数値行があることが合格条件。

### AC3 — ネイティブ GPU サーフェス合成（`--experiment surface --mode adjacent-parent|adjacent-child`）

- `adjacent-parent`（tao 親 HWND を wgpu `Surface` 化し WebView を子として左領域に配置・右に wgpu 描画）と `adjacent-child`（sibling 子 HWND + windows-rs・SAFETY コメント付き局所 `unsafe`）の 2 構成で、検査マトリクス各項目（ヒットテスト・フォーカス遷移・z-order・リサイズ追従・wgpu フレーム継続）を pass/fail + 観察で記録する。DOM overlay 回避策としての `SetWindowRgn` クリップは試行観察項目。
- 「ネイティブ面を WebView の下層に敷く真の DOM 内合成は wry 未対応（PR [#1762](https://github.com/tauri-apps/wry/pull/1762) = `ICoreWebView2CompositionController` + DirectComposition）」という事実を `RESULTS.md` に記録する。fork スパイク（`[patch.crates-io]`）は隣接構成完了後の選択的探索のみとし、結果は「探索的付録」として本編と分離する。
- macOS（layer-backed NSView/CAMetalLayer 合成の見込み）・Linux（真の下層合成は実質不可寄り → 隣接/Canvas 収束）は文書評価行として記録する。

### AC4 — メッセージ境界（`--experiment ipc`）

- **JSON RTT**: JS が `{seq,t0}` を postMessage → Rust が `evaluate_script` で seq を返送 → JS が `t1−t0` を計測。ペイロード 64B / 1KiB / 16KiB / 64KiB × 既定反復で `ipc_rtt_ms{avg,p50,p99}` と単方向 `ipc_ingest_msg_s` を記録。
- **バイナリ pull**: JS が `fetch("poc://data/blob?bytes=N")` を反復し、`ArrayBuffer` 取得までの時間から `protocol_pull_MBps`（64KiB / 1MiB / 16MiB）とリクエスト毎レイテンシを記録。Rust 側は合成 `Cow` バッファを返す。
- **push**: Rust が `evaluate_script` で定速送信し、JS 側の受信間隔・ドロップ・coalesce 挙動を `push_msg_s` + 観察として記録。wry/WebView2 が高頻度 `evaluate_script` を暗黙に coalesce/drop するかは観察項目。
- 3 経路併用の実現性所見を `RESULTS.md` に記録することが合格条件。

### AC5 — フロントエンド比較（`--experiment frontend --variant vanilla|solid`）

- 候補は vanilla TS（参照・下限）と Solid（第一候補）の 2 本（spec の「少なくとも 2 候補」を充足。Svelte 5 は YAGNI で延期・追加容易と記録）。同一ベンチ UI（AC2 ワークロード + intent/差分表示）を bench-shared 共有で構築する。
- 記録項目: `dist` 合計バイト（raw / gzip -9・再現コマンド付き）・Vite ビルド時間・glue-code LOC・更新粒度（メータ更新当たりの DOM mutation 数・Solid の fine-grained 更新が部分適用に留まるか）・開発ワークフロー観察（ビルド→埋め込み配線の手順）。`RESULTS.md` に 2 候補の比較行があることが合格条件。

### AC6 — 配布（`--experiment dist` + release 手順）

- `cargo build --release --manifest-path poc/ui/Cargo.toml` 産物が**埋め込みアセットのみ**で起動し、実機で `--check` 全項目を報告する。WebView2 レジストリ検出（HKLM/HKCU `...\Clients\{F3017226-...}` の `pv` 値、winreg 経由）・バイナリサイズ・検出失敗時の bootstrapper 案内出力を記録する。
- パッケージング hands-on は portable zip（`ui-poc.exe` が自己完結。NSIS/WiX は参照評価で spec の「いずれか」要件を充足）。macOS（`.app` + 署名/公証）・Linux（`libwebkit2gtk-4.1` + GTK3 実行時依存・deb 依存・glibc ベースライン・AppImage）は文書評価行とし、Linux 実行時依存リスクを「依存パッケージ / API バージョン表」で定量化する。

### AC7 — 採否 ADR 確定（タスク `poc-adr-finalize`）

- [採否 ADR](../adr/ui-tech-selection-poc.md) の全採否条項（WebView 実装 / 高頻度描画 / 合成方式 / メッセージ境界 / フロントエンド FW / 配布 / 採否構造）が、`RESULTS.md` の行または `docs/impl/exp/*.jsonl` へのリンク付きで confirm または revise され、status が `accepted` に遷移する。実機未検証項目に「文書評価」ラベルがあること。採否はコンポーネント単位であり固定フォールバック階層にしない。

## Examples

- Given: WebView2 が存在する Windows 11 実機。When: `ui-poc --experiment host` を実行する。Then: 最終 stdout 行が `poc-metric.v1` で `status:"passed"`、`metrics.protocol_load`/`ipc_echo`/`resize_follow`/`focus_transition` が記録され exit 0。
- Given: `UI_POC_HEADLESS=1` または WebView2 不在の環境。When: `ui-poc --experiment render --backend webgl2` を実行する。Then: `status:"skipped"`・非空 `reason`・exit 0 で終了し、失敗扱いにならない。
- Given: `--experiment render --backend webgpu` で `navigator.gpu` または adapter 要求が失敗する環境。When: 実験を実行する。Then: WebGL2 へフォールバックして `status:"passed"`、`metrics.fallback_to` に理由が記録される。
- Given: `--check` を表示のないエージェント環境で実行する。When: 実行する。Then: ウィンドウを作らず capability 検出 JSON を出して exit 0。
- Given: `--experiment surface --mode adjacent-child`。When: sibling 子 HWND 上の wgpu 描画を実施する。Then: 検査マトリクス（ヒットテスト/フォーカス/z-order/リサイズ/フレーム継続）の各項目が pass/fail + 観察で `RESULTS.md` に記録される。
- Given: macOS / Linux の合成実現性のような実機未検証項目。When: `RESULTS.md` に記録する。Then: 当該行に「文書評価」ラベルがあり、実測値として記述されていない。
- Given: PoC 実施後のルートワークスペース。When: `git status` と `mise run check` / `lint` / `test` を実行する。Then: ルート `Cargo.toml` / `Cargo.lock` / `crates/` に差分がなく、検証が PoC 前と同一に成功する。
- Given: 採否 ADR のある条項がエビデンスと矛盾する（例: 隣接構成で z-order が成立しない）。When: `poc-adr-finalize` で条項を確定する。Then: 黙って confirm せず revise として条項を修正し、根拠行へリンクする。

## Guarantees

- AC1–AC8 の各判定が「`poc-metric.v1` の `status` + `RESULTS.md` の証跡行」の組で一意に再現可能であり、`skipped`（前提不足）と `failed`（実験失敗）が混同されない。
- 全メトリックが `env`（os/arch/webview2_version/wgpu_adapter/rustc）と再現コマンド付きの数値として残り、後続の採否 ADR 条項がリンク先を持つ。
- 実機未検証項目が「文書評価」として区別され、実測と誤記されないことがレビューで確認できる。
- `poc/ui/` がワークスペース境界を越えないこと（既存クレートの公開 API・lockfile・lint 設定が不変）が機械的に確認できる。

## Non-goals

- 性能 SLA・保証値の設定（計測値は記録のみ。閾値合否は設けず、採否判断は ADR の責務）。
- macOS / Linux の実機検証（文書評価のみ。Linux 実機/VM 導入可否は後続判断）。
- wry PR #1762 の上流 merge 活動・常設 fork の保守経路の評価（探索的スパイクは AC3 の付録として任意）。
- `ruviform-ui` / `ruviform-ui-webview` / `ruviform-gpu` の実装検証（将来クレート。本仕様は分割を「リハーサルする構造で書けたか」の記録に留める）。
- 製品 UI コンポーネント品質（波形・メータは計測用代表ワークロード）。
- `cargo test` への統合（ハーネスは CLI 実験であり、結果は `RESULTS.md` / impl log に残す）。
- `rust-version = "1.85"` 引き上げ判断（wgpu MSRV 1.87 衝突の記録と専用タスクへの連携のみ）。
- プラグイン GUI 統合（`wxp` / `baseview` 経路は参照評価止まり）。

## Risk

- `status` / exit code の規則が崩れると、前提不足（未実施）を実験失敗と誤判定し、逆に真の失敗を skip 扱いして採否エビデンスが空洞化する。
- `env`・再現コマンドなしに数値だけが記録されると、ADR が引用するエビデンスの再現性が失われ、採否の追跡可能性が崩れる。
- 「文書評価」と実測の区別が失われると、macOS / Linux の未検証リスク（特に `libwebkit2gtk-4.1` 依存と合成限界）を検証済みと誤認した採否が確定する。
- 証跡が gitignore 対象の `out/` だけに残ると、コミット対象の `RESULTS.md` が欠落し、ADR の引用先が消える。
- 隔離契約が破れる（ルート `members`・lockfile への混入）と、「使い捨て PoC」が恒常依存を持ち込み AC8 と baseline の依存方向契約を侵害する。
- フォールバック挙動（WebGPU → WebGL2）の記録が欠けると、「Windows で WebGPU が使えない実機」を検出不能のまま採否に反映できない。
