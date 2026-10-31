---
id: "DISC-034P8EGzRky80ILbGeYdKp"
type: "discovery"
status: "resolved"
title: "Issue #156 GUI ホスト基盤: PoC 棚卸しと本番化ギャップ"
created: "2026-10-30"
updated: "2026-10-31"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/28"
    - "https://github.com/scarecrow173/ruviform/issues/57"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/ui-tech-selection-poc.md"
    - "poc/ui/RESULTS.md"
    - "docs/designs/issue-28-ui-poc.md"
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
    - "docs/adr/plugin-gui-embedding.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/issue-156-gui-host-foundation.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/95"
  defers: []
  deferred-by: []
---

# Issue #156 GUI ホスト基盤: PoC 棚卸しと本番化ギャップ

## Summary

`poc/ui`（Issue #28 / #57）の実機検証成果を棚卸しし、本番用再利用 GUI
ホスト基盤（`ruviform-ui` + `ruviform-ui-webview`）に必要な責務と、
PoC に残る未検証・未整備領域を確定した。採択済み選定（wry+tao /
Solid / 3 経路 IPC / 埋め込みアセット）は再審査せず、PoC 由来の
adapter 構造を本番クレートの設計入力とする。実機検証済みは
Windows 11 x64 のみであり、macOS / Linux は全量「文書評価」で
あることが最大のギャップ。

## Exploration Goal

Issue #156 のタスク 1「既存 `poc/ui`・UI ADR・各種 PoC の成果/
未検証範囲を棚卸しし、本番ライブラリ化に不足するものを記録する」
を実施する。問いは 2 つ:

1. PoC で確立済み・再利用可能な事実は何か（そのまま設計入力にできるか）。
2. 本番ライブラリ化にあたり PoC が持たない責務・検証・配布要件は何か。

## Key Issues and Assumptions

- PoC は `poc/ui` 独立ワークスペースの実験ハーネスであり、公開 API・
  契約保証・利用手順を持たない。本番化は「実験コードの移植」ではなく
  責務境界の再定義として扱う。
- WebView2 の callback は tao `WaitUntil` を起床させない（PoC 実測。
  `EventLoopProxy` 起床が必須）。本番ホストにも同機構が必要。
- `evaluate_script` push は高頻度で暗黙に合体する（max ~105-109
  msgs/task、受信間隔 ~0ms/~15.6ms の二峰）。「1 push = 1 UI 更新」の
  粒度は成立しないため、状態同期は pull + 系列付き latest-wins が前提。
- WebView2 はカスタムスキームを `http://poc.app` 系に書き換える
  （secure context でない = WebGPU 不可の根因）。本番の https
  仮想ホスト経路は Windows でも未実機検証。
- WebView2 UDF はシングルトンロックを持ち、残留ブラウザ子プロセスが
  次回初期化をブロックする。プロセス固有 data_dir が必須。
- `build_as_child` 経路は WebView2 155 で子コンテナ resize が JS
  viewport に反映されない実測。ホストは `build`（非子ビュー）経路を
  既定とし、wry 親 subclass の SetBounds 自動追従 + 冪等な明示
  `set_bounds` を併用する。
- リサイズ反映取りこぼしには ±1px ジッタ再送が有効（PoC で
  retries:4 → 0 に改善）。

## Alternatives and Comparison

本棚卸しでは採択済み ADR の再審査は行わない。比較対象は「PoC 資産を
本番化へどう持ち込むか」の構造案。

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| PoC `host.rs`/`assets.rs`/`check.rs` の責務分割（adapter 層に外部型を封じる）を `ruviform-ui`（ポート）/ `ruviform-ui-webview`（adapter）に引き継ぐ | PoC で実証済みの境界（exp/* が内部 I/F のみに依存）をそのまま設計へ展開。依存方向ルールとも整合 | PoC の実験状態機械・メトリック系は移植しないため API は再設計が必要 | yes |
| `poc/ui` をそのまま公開 API 化して workspace に昇格 | 実装が速い | PoC 契約（自己終了・計測前提・poc:// 名前空間）を本番 API に混入させる。層・依存規約と不整合 | no |
| `ruviform-ui` 単一クレートに wry 実装を feature で内包 | クレート数を減らせる | ベースライン「外部バックエンドは分離クレート」規約に違反。headless 検証経路が wry/WebKitGTK 依存に引きずられる | no |

## Tentative Conclusions and Hypotheses

すべて「confirmed（PoC 実機証跡あり）」または「document-eval
（文書評価・未実機）」と明記する。

### PoC で確立済み（本番化の設計入力）

- **ホスト構造**: tao Window + wry WebView（`build` + `set_bounds`
  追従）。protocol_load / ipc_echo / resize_follow / focus_transition
  すべて実機 pass（Windows 11 x64、WebView2 155.0.4283.62）〔confirmed〕
- **3 経路メッセージ境界**: intent = JSON（`window.ipc.postMessage` →
  `with_ipc_handler`、文字列のみ）/ 状態 push = `evaluate_script` /
  大きな描画データ = カスタムプロトコルのバイナリ pull。
  3 経路併用で測定上の干渉なし〔confirmed〕
- **バイナリ pull の実効**: 1MiB ~94-115MB/s、16MiB ~88-146MB/s、
  初回 ~330ms の cold path（pull-warmup で緩和）。64KiB 小 pull は
  ~2.9MB/s で JSON RTT より遅い → 小さな値は intent/result JSON で
  返し、バルクのみ pull 経路が妥当〔confirmed〕
- **JSON IPC の実効**: RTT 64B avg ~0.5-1ms / 64KiB ~4.4ms、
  単方向 ingest ~40k msg/s（バックログ drain 率であり定常値ではない）〔confirmed〕
- **アセット配布**: rust-embed 埋め込み + `with_custom_protocol` で
  dev server 不要の単一バイナリ（release 8.1MB、portable zip 3.0MB、
  実機で frontend/dist なし単体起動を確認）〔confirmed〕
- **環境検出**: GUI 非生成で WebView2 レジストリ `pv` / wgpu adapter /
  表示環境を検出し実験をゲーティングする構造は本番でも有効
  （`wry::webview_version()` で winreg 依存を減らせる）〔confirmed〕
- **描画責務の分離**: Canvas2D は 32k 列規模で破綻（8.75fps）、
  WebGL2 は 57.9-60fps、wgpu ネイティブサーフェスは隣接共存で検証済み。
  DOM/コントロール層と高頻度描画層の分離が必須〔confirmed〕

### 本番化に不足しているもの（Issue #156 で作る）

- **再利用可能な公開 API**: ホスト生成・イベント正規化・close/shutdown
  契約・エラー型・capability 検出を crate API として定義すること
  （PoC は実験コールバック直結で公開契約なし）。
- **型付き IPC スキーマ**: intent/result/progress/cancel/error/notify の
  バージョン付きエンベロープ（`v:1`）+ JS 側ブリッジ
  （`window.__rvf`）。PoC は実験固有の ad-hoc JSON で、schema・
  エラー規約・id 相関を持たない。
- **サービス呼び出しモデル**: GUI/CLI 双方から同じ非 UI 処理を
  呼べる、Intent → Service → Job（progress/cancel/terminal）の
  汎用経路。`ruviform-exec` の `CancellationToken`/`Progress`/
  `OfflineDriver` 契約と整合させる。
- **bounded data transfer のポート契約**: 大規模描画データを
  名前付き `DataSource` + 範囲 pull + 上限キャップで取得する
  汎用ポート（PoC は固定 `poc://data/blob?bytes=N` の合成バッファのみ）。
- **セキュリティ境界**: 登録 op / 登録 data source / 埋め込みアセットに
  限定するアクセスモデル、外部ナビゲーション拒否、devtools の
  ビルド別制御、パストラバーサル拒否の単一実装。
- **利用手順・配布文書**: crate README・OS 別依存（WebView2
  Evergreen / webkit2gtk-4.1 / WKWebView）・feature・検証経路の文書化。
- **検証レーン**: `webview` feature を非既定とし `cargo check
  --workspace` の headless 経路を壊さない構成 + feature 有効レーン
  （mise タスク + CI ジョブ）。

### 未検証・文書評価のまま残るもの（#156 の完了条件を塞がない）

- macOS / Linux の全実機検証（WKWebView 配布・署名/公証、
  webkit2gtk-4.1 実行時依存、AppImage/deb/Flatpak 選定）。
- https 仮想ホスト経路の in-WebView WebGPU（Windows でも未実機。
  `--unsafely-treat-insecure-origin-as-secure` は開発専用）。
- wry PR #1762 / ICoreWebView2CompositionController による
  DOM 内側コンポジット（fork spike 未実施。Linux では実質不可）。
- Linux カスタムプロトコルの request body（`linux-body` feature +
  WebKit2GTK ≥2.40 が要件）。
- 複数ウィンドウ・WebView レンダラプロセスのクラッシュ検出/復帰。

## Open Questions

- `ruviform-ui-webview` の `webview` feature は非既定が妥当か
  — 設計で確定する（既定の headless 検証経路を壊さないことが
  前提制約。feature 名は外部固有名詞を含まない意図命名）。
- IPC スキーマの詳細（フィールド名・エラーコード集合・job 状態機械）
  — 設計文書で確定する。
- 大規模データのゼロコピー保証はどこまで持てるか — カスタム
  プロトコルは `Cow<'static,[u8]>` 応答で送信側複製は避けられるが、
  WebView 側到達時のコピーは実測依存。実測可能な範囲に限定する。

## Promotion Candidates

- [x] spec-doc needed: 不要 — Issue #156 本文と AudioLens 側
  責務マップが要件正本として十分に明確。別途 spec は作らない
- [ ] adr-doc needed: 不要 — 技術選定は accepted ADR 2 件で確定済み。
  本 Issue の設計判断（crate 分割・feature・IPC スキーマ）は
  design-doc に記録する
- [x] design-doc needed: `docs/designs/issue-156-gui-host-foundation.md`
  （責務・依存 DAG・IPC スキーマ・検証・配布の確定場所）

## Sources and Relations

- Issue #156（本棚卸しの要求元）・#28（UI PoC）・#57（高負荷
  追検証）・#155（CLI/headless。GUI と同じ非 UI 処理を共有する対側）
- 採択 ADR: `ui-architecture-webview-hybrid.md`（方針）・
  `ui-tech-selection-poc.md`（技術選定・実機証跡）
- 実測正本: `poc/ui/RESULTS.md`（AC→実験→証跡）、
  `docs/impl/exp/`・`docs/impl/ir/poc-*.md`
- 層・依存方向: `docs/designs/ruviform-architecture-design-baseline.md`
  （ruviform-ui / ruviform-ui-webview / ruviform-gpu の既確定行）
- AudioLens 側責務境界: 移行責務マップ（al97 worktree の
  `docs/designs/ruviform-migration-responsibility-map.md`。Ruviform 側に
  AL 固有型を持ち込まない契約）

## Evidence

### 2026-10-31: `poc/ui` + ADR 棚卸し（実機実績の抜粋）

- AC1 host: protocol_load.ok、ipc_echo RTT 8.1ms（EventLoopProxy
  修正前 255.7ms）、resize_follow.ok（要求 [947,631] と実測一致・
  retries:0）、focus_transition.ok（mechanism:"window"）。
- AC4 IPC: JSON RTT 64B avg 0.57-1.06ms p99≤3.3 / 64KiB avg ~4.4ms、
  ingest ~40.4-44.1k msg/s（drain 率）、pull 64KiB ~2.9MB/s /
  1MiB ~94-115MB/s / 16MiB ~88-146MB/s（初回 ~330ms cold）、
  eval push ~192-218 msg/s（coalesce 231-276/360、max_batch ~105-109、
  間隔二峰 ~0/~15.6ms）。3 経路併用で劣化なし。
- AC2/#57 描画: baseline（~2k 列+8ch meter）は全方式 60fps だが
  workload 固有であり SLA ではない。高負荷: canvas2d 10k 列/64m →
  30.7fps、32k/128 → 8.75fps。webgl2 10k/64 → 60fps、32k/128 →
  57.9fps、+720p preview → work p99 16.8ms（16.7ms 枠の際）。
- AC3 ネイティブサーフェス: adjacent-parent/child 両モードで 9 項目
  検査 pass（z-order の観測: WebView コンテナ既定最上位、
  SetWindowRgn では子 HWND 木を構造的に clip 不可、
  WindowFromPoint は hit test 不適）。
- AC5 frontend: vanilla dist 11,371B vs solid 20,194B（gzip-9
  5,152B/8,682B）、更新粒度は同等（mutations/update=1.0）。Solid 採択
  （vanilla は小 UI の下限として残置）。
- AC6 配布: release 単一 exe 8,100,864B が temp dir で単体起動、
  portable zip 3,039,116B。実行時前提は WebView2 Evergreen のみ。
- OS 別依存（ADR・PoC 記録の要約）:
  - Windows: WebView2 Evergreen 必須（レジストリ pv 検出、欠落時は
    fwlink 2124703 / winget Microsoft.EdgeWebView2Runtime）。
    UDF シングルトンロック → プロセス固有 data_dir 必須。
    カスタムスキームは http:// に書き換えられる。
  - macOS: WKWebView は OS 同梱。Gatekeeper 対応に Developer ID
    署名 + notarytool 公証が必要。配布形態 .app bundle/DMG は未検証。
  - Linux: libwebkit2gtk-4.1（GTK3/soup3）が唯一の実行時選択肢
    （4.0/soup2 は WebKitGTK 2.52 で削除、webkitgtk-6.0/GTK4 は
    wry の GTK3 経路と不整合、soup2+soup3 同居はクラッシュ）。
    Ubuntu ≤20.04/Debian ≤11 等の旧 glibc 環境は deb 解決不可 →
    AppImage 等が必要。request body は `linux-body` feature +
    WebKit2GTK ≥2.40。
- ハーネス構造: poc/ui は独立 workspace。外部型（wry/tao/winreg/
  http/dpi）は host.rs/assets.rs/check.rs に封じ込め、exp/* と
  metrics.rs は内部 I/F のみに依存 — ruviform-ui / ruviform-ui-webview
  分割の予行として設計済み。unsafe は `unsafe_code=warn` +
  `#[expect]` 局所許可 12 箇所（exp/surface.rs）に封じ込め。
