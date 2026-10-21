---
id: "PLAN-034JfDEU8mhCDj4ldYFZUG"
type: "plan"
status: "completed"
title: "Issue #28 UI 技術検証 PoC 実施計画"
created: "2026-10-20"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/28"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/poc-adr-finalize.md"
      - type: "doc"
        path: "docs/impl/exp/ac7-adr-finalize.jsonl"
    modified:
      - type: "file"
        path: "poc/ui/RESULTS.md"
      - type: "doc"
        path: "docs/adr/ui-tech-selection-poc.md"
      - type: "doc"
        path: "docs/specs/issue-28-ui-poc.md"
      - type: "doc"
        path: "docs/tasks/poc-adr-finalize.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-28-ui-poc.md"
  implemented-by:
    - "../tasks/poc-adr-finalize.md"
    - "../tasks/poc-frontend-and-dist.md"
    - "../tasks/poc-message-boundary.md"
    - "../tasks/poc-native-surface.md"
    - "../tasks/poc-render-backends.md"
    - "../tasks/poc-ui-harness-host.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../specs/issue-28-ui-poc.md"
    - "../designs/issue-28-ui-poc.md"
    - "../adr/ui-tech-selection-poc.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-28-ui-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/issue-28-ui-poc.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #28 UI 技術検証 PoC 実施計画 Implementation Plan

## Goal

spec `SPEC-034Jc9BJlI7ULEBWoIlMP2`（Issue #28）の受け入れ基準 AC1–AC8 を充足する: `poc/ui/` 独立ワークスペースに使い捨て検証ハーネス（`ui-poc` 単一バイナリ + `--experiment` 駆動）を構築し、Windows 11 実機（WebView2 155 系・pin toolchain `rust = "1.99.0"`）で AC1–AC6 の実験を実施して `poc-metric.v1` 契約の構造化エビデンスを `poc/ui/RESULTS.md`・`docs/impl/exp/*.jsonl` に記録し、採否 ADR（`ADR-034Jc9Cguud7nZ2Tt88Ptu`）の全条項を confirm / revise して `accepted` へ遷移させる。ルートワークスペースへの変更はゼロ（AC8）。

## Scope

単一の実施ストリームとして扱う。T1 ハーネスが全実験の共通基盤であり、T2–T5 の各実験と T6 の ADR 確定は同一クリティカルパス・同一リリース意図（Issue #28 の成果物 = 採否 ADR）上にあるため、分割しない。

範囲外（spec Out of Scope・design Deferred Design Concerns と一致）: `ruviform-ui` / `ruviform-ui-webview` / `ruviform-gpu` クレートの実装、プラグイン GUI 統合（`wxp` / `baseview` は参照評価止まり）、製品 UI コンポーネント、macOS / Linux 実機検証（文書評価のみ）、wry PR #1762 の上流 merge 活動、性能 SLA の設定、`rust-version = "1.85"` 引き上げ判断（MSRV 見直し専用タスクへ事実を連携するのみ）、フロントエンドビルドの Cargo / mise タスク統合（PoC では手動配線）。

## Upstream Documents

- [../specs/issue-28-ui-poc.md](../specs/issue-28-ui-poc.md) — `SPEC-034Jc9BJlI7ULEBWoIlMP2`（approved）。本 plan が実装する spec。AC1–AC8・Requirements の正本
- [GitHub Issue #28](https://github.com/scarecrow173/ruviform/issues/28) — 一次ソース（`relations.source`）

## Design Inputs

- [../designs/issue-28-ui-poc.md](../designs/issue-28-ui-poc.md) — `DESIGN-034JcpvK9cZEfkeEbBLBoz`（approved）。ハーネス構成・実験設計・計測契約（`poc-metric.v1`・`--check`・exit code・自己終了）・T1–T6 分解の正本。**本 plan はこの分解をそのまま採用する**
- [../adr/ui-tech-selection-poc.md](../adr/ui-tech-selection-poc.md) — `ADR-034Jc9Cguud7nZ2Tt88Ptu`（accepted — T6 で全条項 confirm/revise 済み）。confirm / revise 対象の採否条項の正本（T6 の作業対象）
- [../discovery/issue-28-ui-poc.md](../discovery/issue-28-ui-poc.md) — `DISC-034JbXZSba3SvOavRFfs66`。依存バージョン・プラットフォーム実情・未解決事項のファクト源
- [../test-specs/issue-28-ui-poc.md](../test-specs/issue-28-ui-poc.md) — 判定規則（PASS / FAIL / SKIP・記録要件・文書評価ラベル・隔離検証）の正本。各実験の合格条件は本書に従う

## File Map

`poc/ui/`（独立 cargo ワークスペース。ルート `Cargo.toml`・`Cargo.lock`・`crates/`・`mise.toml` には一切触れない）:

| Path | Responsibility | Notes |
| --- | --- | --- |
| `poc/ui/Cargo.toml` | `[workspace]` + `[package]` `ui-poc` 単一 bin | ルート `members` は明示リストで glob なしのため吸収されない（design 検証済み）。`exclude` 不要 |
| `poc/ui/Cargo.lock` | PoC 固有の lockfile | コミット対象 |
| `poc/ui/.gitignore` | `node_modules/`・`*/dist/`・`out/` | ルート `.gitignore` は既に `target/` を任意深度でカバー |
| `poc/ui/README.md` | ビルド・実行・再現手順 | T1 で作成 |
| `poc/ui/RESULTS.md` | 集約結果の正本（AC → 実験 → エビデンス行対応表 + 再現コマンド + 文書評価ラベル） | コミット対象・ADR の引用先。T1 でスケルトン作成 |
| `poc/ui/src/main.rs` | CLI dispatch・exit code（0/1/2） | |
| `poc/ui/src/args.rs` | std-only 引数パース（`--experiment`/`--frames`/`--duration`/`--backend`/`--mode`/`--variant`/`--out`/`--check`） | clap 等は使わない（KISS） |
| `poc/ui/src/check.rs` | capability 検出（WebView2 レジストリ・wgpu アダプタ・OS/toolchain）と `--check` JSON 出力 | GUI を作らない唯一の入口 |
| `poc/ui/src/host.rs` | tao `Window` + wry `WebView`・自己終了タイマ・done シグナル受信 | 外部型（wry/tao）はここに封じる |
| `poc/ui/src/assets.rs` | `rust-embed` + `with_custom_protocol`（`poc://app/` 埋め込み配信・`poc://data/` 計測バイナリ応答） | debug=FS 読み・release=埋め込み |
| `poc/ui/src/metrics.rs` | `poc-metric.v1` スキーマ・パーセンタイル集計・stdout/`out/` 書き出し | wry 型に触れない |
| `poc/ui/src/exp/mod.rs` | `Experiment` trait（setup/run/teardown）と実験レジストリ | wry 型に触れない（内部 host I/F のみ） |
| `poc/ui/src/exp/render.rs` | AC2 render 実験 | T2 |
| `poc/ui/src/exp/ipc.rs` | AC4 ipc 実験 | T3 |
| `poc/ui/src/exp/surface.rs` | AC3 surface 実験（wgpu はこのモジュールに封じる。windows-rs の SAFETY コメント付き局所 `unsafe`） | T4 |
| `poc/ui/src/exp/dist.rs` | AC6 dist 実験（WebView2 検出・バイナリサイズ・bootstrapper 案内） | T5 |
| `poc/ui/frontend/package.json` + `package-lock.json` | npm workspaces（bench-shared + vanilla + solid） | lock はコミット対象 |
| `poc/ui/frontend/bench-shared/` | ワークロード生成・rAF フレーム統計・IPC ブリッジ・fetch タイミング（FW 非依存） | T2 |
| `poc/ui/frontend/vanilla/` | Vite + TS 参照実装（FW ゼロ下限） | T2 |
| `poc/ui/frontend/solid/` | Vite + vite-plugin-solid（第一候補） | T5 |
| `poc/ui/out/` | 生 JSONL サンプル | gitignore・再生成可能 |
| `docs/impl/exp/*.jsonl` | 過程の観察（impl-doc 規約の実験ログ） | 各タスクの実施時に記録 |

将来クレート分割のリハーサル: `host.rs` / `assets.rs` / `check.rs` が外部型（wry・tao・winreg・windows-rs）を持つ adapter 側、`exp/*`・`metrics.rs` は内部 host I/F にのみ依存する port 側。外部型を境界内に留められたかは `RESULTS.md` に評価軸として記録する。

## Implementation Sequence

### T1: PoC ハーネスホスト（task: poc-ui-harness-host）

- [x] `poc/ui/` 独立ワークスペース骨格（`Cargo.toml` `[workspace]`+`[package]`・`.gitignore`・`Cargo.lock`）を作成し、ルート `Cargo.toml` / `Cargo.lock` / `crates/` が無変更であることを確認する
- [x] `args.rs`（std-only パース）+ `main.rs`（dispatch・exit code 0/1/2）+ `check.rs`（WebView2 レジストリ・wgpu アダプタ・OS/toolchain 検出、`--check` で JSON 出力 exit 0）
- [x] `host.rs`（tao Window + wry WebView・自己終了タイマ）+ `assets.rs`（`with_custom_protocol` で `poc://app/` 埋め込み配信）+ IPC echo（`window.ipc.postMessage` → `with_ipc_handler` → `evaluate_script` 返送）
- [x] `metrics.rs` に `poc-metric.v1` スキーマ・パーセンタイル・stdout 最終行 + `out/*.jsonl` 追記を実装
- [x] `RESULTS.md` スケルトン（AC → 実験 → エビデンス行対応表・再現コマンド欄・文書評価ラベル欄）と `README.md`（前提: `npm ci && npm run build`、再現手順）を作成
- [x] `--experiment host` を実機で実行し AC1 項目（`protocol_load`/`ipc_echo`/`resize_follow`/`focus_transition`/`webview2_version`）を記録

### T2: 描画バックエンド計測（task: poc-render-backends。T1 完了後・T3/T4 と並行可）

- [x] `frontend/` に npm workspaces 骨格 + `bench-shared/`（波形ピーク列 約 2,000 点/フレーム + メータ群のワークロード生成・rAF 統計・IPC ブリッジ）+ `vanilla/`（Vite+TS 参照 UI）を構築
- [x] `exp/render.rs` に `--backend canvas2d|webgl2|webgpu` dispatch と `frame_time_ms avg/p50/p95/p99`・`dropped`・`achieved_fps` の集約を実装（既定 600 フレームで自己終了）
- [x] WebGPU は `navigator.gpu` + adapter 要求の機能検出を実装し、失敗時 WebGL2 フォールバック + `metrics.fallback_to` 記録を確認
- [x] 3 バックエンド × 既定フレームで実機実行し、バックエンド別数値行を `RESULTS.md` に記録

### T3: メッセージ境界計測（task: poc-message-boundary。T1 完了後・T2/T4 と並行可）

- [x] `exp/ipc.rs` に JSON RTT 計測（JS `{seq,t0}` postMessage → Rust `evaluate_script` 返送 → JS `t1−t0`。ペイロード 64B/1KiB/16KiB/64KiB）を実装し `ipc_rtt_ms{avg,p50,p99}`・`ipc_ingest_msg_s` を記録
- [x] `poc://data/blob?bytes=N` のバイナリ pull（`Response<Cow<'static,[u8]>>`）を実装し `protocol_pull_MBps`（64KiB/1MiB/16MiB）とリクエスト毎レイテンシを記録
- [x] `evaluate_script` 定速 push を実装し `push_msg_s`・受信間隔・ドロップ・coalesce 挙動を観察記録
- [x] 3 経路併用の実現性所見を `RESULTS.md` に記録

### T4: ネイティブサーフェス合成（task: poc-native-surface。T1 完了後・T2/T3 と並行可）

- [x] `exp/surface.rs` に `adjacent-parent`（tao 親 HWND を wgpu `Surface` 化・WebView を `build_as_child` で左領域・右に wgpu 描画）を実装
- [x] `adjacent-child`（sibling 子 HWND: windows-rs `WS_CHILD` + raw-window-handle。windows-rs 呼出箇所のみ SAFETY コメント付き局所 `unsafe`）を実装
- [x] 検査マトリクス（ヒットテスト・フォーカス遷移・z-order・リサイズ追従・wgpu フレーム継続）を両モードで pass/fail + 観察記録。`SetWindowRgn` クリップは試行観察
- [x] 「真の DOM 内合成は wry 未対応（PR #1762 = CompositionController/DirectComposition）」を `RESULTS.md` に記録し、macOS/Linux は文書評価行を追加。fork スパイクは隣接構成完了後の任意探索（「探索的付録」として分離）

### T5: フロントエンド比較と配布（task: poc-frontend-and-dist。T1+T2 完了後）

- [x] `frontend/solid/`（Vite + vite-plugin-solid）に T2 のベンチ UI を移植し、`--variant vanilla|solid` で比較実行
- [x] dist 合計バイト（raw/gzip -9）・Vite ビルド時間・glue-code LOC・DOM mutation 数・ワークフロー観察を `RESULTS.md` に 2 候補比較行として記録（AC5）
- [x] `cargo build --release` で単一バイナリを作り、`exp/dist.rs` で WebView2 レジストリ検出（winreg・`pv` 値）・バイナリサイズ・未検出時 bootstrapper 案内を実装・実機検証
- [x] portable zip を hands-on で構成し、macOS（`.app`+署名/公証）・Linux（`libwebkit2gtk-4.1`+GTK3・deb 依存・glibc ベースライン・AppImage）を依存パッケージ/API バージョン表で定量化した文書評価行を記録（AC6）

### T6: 採否 ADR 確定（task: poc-adr-finalize。T2–T5 完了後）

- [x] `RESULTS.md` の AC → 実験 → エビデンス対応表を完成させ、各採否条項（WebView 実装/高頻度描画/合成方式/メッセージ境界/フロントエンド FW/配布/採否構造）をエビデンス行へのリンク付きで confirm または revise する
- [x] 実機未検証項目に「文書評価」ラベルがあることを監査し、ADR status を `accepted` に遷移（レビューは review-protocol で確認）
- [x] spec の AC チェックボックス完了と status 遷移（`approved` → `implemented`）・plan/task status 更新・`changes` 記録を同期し、impl 記録（`docs/impl/ir/`）が各 task に紐付いていることを監査
- [x] wgpu MSRV 1.87 と宣言 `rust-version = "1.85"` の衝突事実を MSRV 見直し専用タスクへ連携記録

## Dependencies and Risks

### Dependency Graph

| 依存元 | 依存先 | 内容 |
| --- | --- | --- |
| T2 | T1 | host・metrics・frontend 配線が前提 |
| T3 | T1 | host・metrics・protocol 配線が前提 |
| T4 | T1 | host・check（wgpu 検出）が前提 |
| T5 | T1・T2 | bench UI（T2 産物）の Solid 移植と release/dist 実験が前提 |
| T6 | T2・T3・T4・T5 | 全実験エビデンスの集約が前提 |

- T1 が全タスクの前提。T1 完了後、T2/T3/T4 は並行可能（ただし wry WebView = 1 イベントループのため実験はプロセス分離で実行・同一バイナリでも `--experiment` は起動時選択のみ）
- T1 完了時点で `poc-metric.v1` スキーマと `RESULTS.md` 対応表が ADR 引用先として機能するかレビューする（design の unchecked 項目）

### Risk Register

| # | リスク | 影響 | 確率 | 対応 |
| --- | --- | --- | --- | --- |
| 1 | wry/tao/wgpu 依存木の初回ビルドが重い（数分級） | T1 以降の反復が鈍る | 高 | `poc/ui` 専用 `target/` キャッシュ。wgpu を optional feature にする逃げ道を README に記録（初期は KISS で常時依存） |
| 2 | GUI/GPU 実験がエージェント・ヘッドレス環境で実行不能・flaky | 自動化経路で実験が走らない | 高 | `--check` + `status:"skipped"` 規約で分離。実機実行はオペレータ実施とし再現手順を `RESULTS.md` に記録。CI が呼ぶのは `--check` のみ |
| 3 | wry PR #1762 未 merge（DOM 内合成が公式に作れない） | AC3 の理想構成が実現しない | 確定 | 隣接構成を主経路とし DOM 内合成は「未 merge 上流エビデンス」として文書記録。fork スパイクは任意の探索的付録に隔離 |
| 4 | wgpu MSRV 1.87 が宣言 `rust-version = "1.85"` と衝突 | ワークスペース内では wgpu を依存にできない | 確定 | 独立 WS + pin toolchain 1.99.0 で先行検証。衝突の事実を `RESULTS.md` に記録し MSRV 見直しタスクへ連携（引き上げ自体は決めない） |
| 5 | Linux `libwebkit2gtk-4.1` 実行時依存は文書評価止まり | 配布採否の一部が実測なし | 確定 | 依存パッケージ/API バージョン表で定量化し「文書評価」ラベルを必須化（test-spec で強制） |
| 6 | 計測の客観性（エージェント採取値の信頼性） | ADR が引用する数値の再現性 | 中 | `env` を `poc-metric.v1` に同梱・生サンプルは `out/` で再生成可能・`RESULTS.md` に再現コマンド必須 |
| 7 | 単一バイナリ内の実験間干渉 | 前実験の状態が次実験に混入 | 低 | 実験はプロセス分離で実行（`--experiment` は起動時選択。プロセス内連続実行は持たない） |
| 8 | `frontend/dist` 未ビルドで実験が skip 化 | フロント依存実験が走らない | 中 | `package-lock.json` コミット・`dist/` gitignore・`npm ci && npm run build` を README の標準前提とし未ビルド時は `reason: frontend not built` で skip |

## Task Breakdown

| Task | 内容 | 対応 AC | 依存 |
| --- | --- | --- | --- |
| [poc-ui-harness-host.md](../tasks/poc-ui-harness-host.md) | 独立 WS・`ui-poc` bin・`--check`・wry+tao ホスト・埋め込み protocol・IPC echo・自己終了・`poc-metric.v1`・`RESULTS.md` 骨格 | AC1, AC8 | なし |
| [poc-render-backends.md](../tasks/poc-render-backends.md) | bench-shared + vanilla 参照 UI・canvas2d/webgl2/webgpu フレーム統計計測・記録 | AC2 | T1 |
| [poc-message-boundary.md](../tasks/poc-message-boundary.md) | JSON IPC RTT・バイナリ pull スループット・evaluate_script push の 3 経路計測 | AC4 | T1 |
| [poc-native-surface.md](../tasks/poc-native-surface.md) | wgpu 隣接構成（parent/child 2 モード）・イベント/z-order/リサイズ検査・PR #1762 文書評価 | AC3 | T1 |
| [poc-frontend-and-dist.md](../tasks/poc-frontend-and-dist.md) | Solid 移植 + FW 比較表・release 単一バイナリ + WebView2 検出 + portable zip・macOS/Linux 文書評価 | AC5, AC6 | T1, T2 |
| [poc-adr-finalize.md](../tasks/poc-adr-finalize.md) | エビデンス → ADR confirm/revise → `accepted`・RESULTS 完成・spec status 遷移・impl 記録監査 | AC7 | T2–T5 |

## Verification

| ID | 対象（AC → 実験） | 検証コマンド / 確認 | 証跡（test-spec 規則 → ADR 条項） |
| --- | --- | --- | --- |
| V-1 | AC1 → `host` | `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment host` が `status:"passed"`・`protocol_load`/`ipc_echo`/`resize_follow`/`focus_transition` 記録・exit 0 | RESULTS.md AC1 行 → ADR「WebView 実装」条項 |
| V-2 | AC2 → `render` | `--experiment render --backend canvas2d\|webgl2\|webgpu --frames 600` で各バックエンドの `frame_time_ms` 統計 + `dropped` + `achieved_fps` が `poc-metric.v1` 最終行に出力 | RESULTS.md バックエンド別行 → ADR「高頻度描画」条項 |
| V-3 | AC3 → `surface` | `--experiment surface --mode adjacent-parent\|adjacent-child` の検査マトリクス全項目が pass/fail + 観察で記録。PR #1762 依存と macOS/Linux 文書評価行が存在 | RESULTS.md AC3 行 → ADR「合成方式」条項 |
| V-4 | AC4 → `ipc` | `--experiment ipc` で `ipc_rtt_ms{avg,p50,p99}`・`ipc_ingest_msg_s`・`protocol_pull_MBps`・`push_msg_s` + coalesce 観察が記録 | RESULTS.md AC4 行 → ADR「メッセージ境界」条項 |
| V-5 | AC5 → `frontend` | `--experiment frontend --variant vanilla\|solid` で dist バイト・ビルド時間・LOC・DOM mutation 数が 2 候補で記録 | RESULTS.md 比較行 → ADR「フロントエンド FW」条項 |
| V-6 | AC6 → `dist` | `cargo build --release --manifest-path poc/ui/Cargo.toml` の単一バイナリが埋め込みのみで起動し `--check` 全項目を報告。WebView2 検出・バイナリサイズ・bootstrapper 案内を記録 | RESULTS.md AC6 行（Windows hands-on + macOS/Linux 文書評価）→ ADR「配布」条項 |
| V-7 | AC7 → ADR 確定 | 採否 ADR の全条項がリンク付きで confirm/revise され status=`accepted`・「文書評価」ラベル監査済み | ADR 本体 + `docs/impl/exp/` ログ |
| V-8 | AC8 → 隔離 | `git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml` が空・`mise run check`/`lint`/`test` が PoC 前後で同一成功・`cargo metadata` で `members` に poc 混入なし | RESULTS.md AC8 行 → ADR「採否構造」条項 |
| V-9 | ハーネス契約 | `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --check` がヘッドレスで JSON 出力 exit 0。`UI_POC_HEADLESS=1` または WebView2 不在で各実験が `status:"skipped"` + 非空 `reason` + exit 0 | test-spec「status/exit code セマンティクス」規則 |

判定規則（PASS / FAIL / SKIP・記録要件・文書評価ラベル・`poc-metric.v1` 契約）は [test spec](../test-specs/issue-28-ui-poc.md) に委譲し、本表では実験 → 証跡 → ADR 条項の対応と入口コマンドのみを管理する。

## Rollback Strategy

| 対象 | 戻し方 | 影響 | 備考 |
| --- | --- | --- | --- |
| `poc/ui/` 全体 | ディレクトリ削除（`git rm -r poc/ui` または未追跡なら削除） | ルートワークスペースはそもそも無変更のため影響なし | PoC は additive。`out/`・`target/`・`dist/`・`node_modules/` は gitignore 済みで残骸を残さない |
| 本 plan・test-spec・task・backfill 済み relations | `git revert`（design/spec の `derived-by`/`implemented-by`/`verified-by` 追記を含む） | ドキュメントグラフが PoC 計画前の状態に戻る | コミット単位で巻き戻し可能な範囲に留める |
| 採否 ADR の `accepted` 遷移 | ADR の status と条項を `proposed` へ戻す編集 | 採否が未確定に戻るのみ | T6 完了後に発覚した反証は revert ではなく ADR 改訂として扱う |
| `RESULTS.md` / `docs/impl/exp/` の記録 | 削除しない（実施済み計測の記録は保持） | なし | 計測値の撤回は削除ではなく「訂正行の追記」で行う |

不可逆な変更（ルート資産の改変・外部公開 API・lockfile の強制更新）は本 plan の操作に含まれない。

## Open Questions / Gaps

- `adjacent-child` の sibling HWND を wry API（`build_as_child` の第 2 子ビューや `WebViewExtWindows` reparent）で構成できるか、windows-rs 直叩きが必要か — T4 実装時に最小構成で選択（design Open Questions）
- wry/WebView2 が高頻度 `evaluate_script` を暗黙に coalesce/drop するか — T3 の観察項目として実測
- `WebContext` のデータディレクトリを PoC 既定の一時ディレクトリに向けるか（再現性推奨）— T1 実装時に決定
- `SetWindowRgn` クリップが DOM overlay 回避策として実機で実用的か — T4 の試行観察項目
- Linux 実機/VM を PoC 後半で用意するか — discovery の Open Question として判断を保留（本 plan は文書評価で処理）

## Self-Review

- [x] spec の AC1–AC8 が T1–T6・V-1〜V-9 に対応している（AC1/AC8→T1、AC2→T2、AC4→T3、AC3→T4、AC5/AC6→T5、AC7→T6、隔離→V-8）
- [x] design のハーネス構成・計測契約・T1–T6 分解を採用し、分解の変更はない（タスク名・依存・担当 AC が一致）
- [x] TBD / TODO / 「後で実装」のプレースホルダは残っていない（据え置き項目は Open Questions / Gaps と design Deferred Design Concerns に記録）
- [x] ファイルパス・relation・status・用語が spec/design/ADR/test-spec と整合している

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation（T1 完了後に T2/T3/T4 は並行実施可能。GUI/GPU 実験の実機実行はオペレータによる実行が前提のため、サブエージェントにはコード実装と `--check`/skip 経路の検証を、実機計測はオペレータに振り分ける）
- Independent work streams: T2・T3・T4（T1 完了後に相互独立）。T5 は T2 依存、T6 は全実験完了後の直列フェーズ
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 明示的な仮定: Node 26 / npm 11 が利用可能（discovery の実機調査）。WebView2 evergreen 155 系が実機に存在。pin toolchain `rust = "1.99.0"`（mise）で `poc/ui` をビルドする。`poc/ui` は `[workspace]` を持つ独立ワークスペースで、ルート `members` が明示リストのため `exclude` 不要（design で検証済み）
- 意図的に残した gap: sibling HWND の構成 API・`evaluate_script` coalesce・`WebContext` データディレクトリ・`SetWindowRgn` 実用性は設計段階で未解決のまま実装/観察に委譲（Open Questions / Gaps）
- 未解決の依存: `rust-version = "1.85"` と wgpu MSRV 1.87 の衝突は既存の MSRV 見直し専用タスクへ入力として引き渡す（本 plan は解決しない）
- 上流参照: spec `SPEC-034Jc9BJlI7ULEBWoIlMP2`、design `DESIGN-034JcpvK9cZEfkeEbBLBoz`、採否 ADR `ADR-034Jc9Cguud7nZ2Tt88Ptu`、discovery `DISC-034JbXZSba3SvOavRFfs66`、Issue #28
