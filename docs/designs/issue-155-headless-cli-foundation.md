---
id: "DESIGN-WVN475nNdieOE0fyOxkhf"
type: "design"
status: "approved"
title: "GUI 非依存 CLI/headless 実行基盤設計（Issue #155）"
created: "2026-10-31"
updated: "2026-10-31"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/155"
  changes:
    added:
      - "crates/ruviform-exec/src/job.rs"
      - "crates/ruviform-cli-sample/"
    modified:
      - "crates/ruviform-exec/src/offline.rs"
      - "crates/ruviform-exec/src/lib.rs"
      - "Cargo.toml"
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
    - "https://github.com/scarecrow173/audio-lens/issues/95"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/offline-execution-driver.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/adr/offline-determinism-scope.md"
  defers: []
  deferred-by: []
---

# GUI 非依存 CLI/headless 実行基盤設計（Issue #155）

## Context

Issue #155（AudioLens 移行 #95 由来）。動機は「同じ解析・メディア処理を GUI アプリケーションと CLI/headless アプリケーションの両方から再利用する」こと。本設計は Ruviform が提供する headless 実行・制御契約の棚卸し結果と、不足した汎用部分だけを補う拡張、および最小 CLI 利用例の構成を確定する。前提として #151 の観測契約（`Fact<T>`・`observe_timing`・`NativeAudioFrame`）と #154 のバックエンド（`ruviform-io-avi`・`ruviform-io-symphonia`）が worktree に存在する。

## 棚卸し結果（設計判断の根拠）

| 必要機能 | 既存提供 | 不足 |
| --- | --- | --- |
| 非 RT オフライン駆動（逐次/並列・チャンク境界キャンセル・決定論） | `OfflineDriver::run` / `run_parallel`（Issue #24） | なし |
| 協調キャンセル | `CancellationToken`（共有 `Arc<AtomicBool>`） | なし |
| 進捗（ポーリング） | `Progress` の原子カウンタ（`completed`/`total`/`fraction`） | カウンタ操作が driver 内部専用・push 通知なし |
| エラー/結果 | `OfflineError` + media-io 正規化 `ErrorKind` | なし（利用側が合成する） |
| メディア読込 | `Demuxer`/`Decoder` trait + symphonia/AVI バックエンド | なし |
| 非同期駆動（spawn・状態・イベント・join・detach） | なし | **OfflineJob 系 — 本設計で追加** |
| 進捗 push 通知 | なし | **`Progress` リスナー — 本設計で追加** |

不足は「実行中操作を外から観測・制御する薄いハンドル」のみであり、同期ドライバ・トークン・media I/O の再実装は不要と確定。新設 CLI フレームワークは方針により作らない。

## Scope

### In Scope

- `ruviform-exec` への汎用拡張（`job` モジュール + `Progress` リスナー）:
  - `OfflineJob<T,E>` — 1 ワーカースレッド上で操作本体を走らせる spawn/observe/cancel/join ハンドル
  - `JobScope` — 本体に渡す共有 `CancellationToken` + `Progress` クローン
  - `JobState`（Running/Cancelling/Finished）・`JobOutcome`（Completed/Cancelled/Failed）・`JobEvent`（Started/Progress/Finished）・`JobError<E>`（Operation/Panicked）
  - `Progress::{set_listener, clear_listener, reset, bump}` — ポーリングと併用する push 経路
- `ruviform-cli-sample`（利用側層・新設クレート）:
  - 共有 headless サービス部（lib）: `probe`・`spawn_analyze`・`AnalysisReport`/`ProbeReport`・メータカーネル（`MeterStage`/`Meter`）・バックエンドルーティング・JSON レンダラ
  - CLI アダプタ部（bin）: clap 引数パース・`probe`/`run` コマンド・stdout JSON/stderr JSONL・exit code 写像・Ctrl-C→cancel・`--cancel-after` 検証フック
- headless 分離の検証（`cargo tree -p ruviform-cli-sample` で GUI/デバイス/プラグイン非依存を確認）

### Out of Scope（利用アプリ側責務として明示的に除外）

- コマンド名/オプション・Job Definition・永続 Manifest・診断スキーマ・業務固有 exit code・stdout/stderr ポリシ・プロセス終了・引数パース — すべて利用側。sample はこれらを**例として**実装するが Ruviform API には昇格しない
- ジョブレジストリ（`status`/`cancel <id>` の id 付き複数ジョブ管理）・永続化・`resume`・async executor・スレッドプール共有
- AudioLens `al-cli` の `run`/`resume`/`inspect` の移植・置換 — 共通部品利用のデモに限定
- RT パス・デバイス・プラグインホスト・GUI/WebView への依存追加

## Components and Boundaries

```text
利用アプリ層（CLI シェル | GUI アダプタ | 将来の他コンシューマ）
      │  command / args / stdout / exit code  ← アプリ所有
      ▼
ruviform-cli-sample (lib) — 今回の例。GUI はこれと同形の関数を呼ぶ:
      probe(MediaSource) -> Result<ProbeReport, PipelineError>
      spawn_analyze(MediaSource, AnalyzeOptions) -> OfflineJob<AnalysisReport, PipelineError>
      │
      ▼  ← Ruviform 所有（本設計の拡張面は太字）
ruviform-exec ─ OfflineJob/JobScope/JobEvent/JobError 【新設】
              └ OfflineDriver::run/run_parallel（既存）・CancellationToken・Progress(+listener)
ruviform-media-io + ruviform-io-symphonia/ruviform-io-avi（既存）
ruviform-rt — Processor/RtProcessor カーネル契約（既存、sample の Meter が実装）
```

- `OfflineJob` は責務を「操作を別スレッドで駆動し状態・イベント・キャンセル・結果を提供する」に限定する薄い接着層。ジョブ管理・永続化・executor を持たない（`job.rs` モジュール doc に非責務を明記）。
- `Progress` リスナーはカウンタを更新するスレッド上で同期的に呼ばれる。`OfflineJob` はこれを使って scope 側 `Progress` だけにイベント送出リスナーを載せ、ハンドル側クローンにリスナーを持たせない — ワーカー終了で sender が drop され、受信側にチャネル close（確実な終端合図）が届く。
- イベントチャネルは有界（容量 64）で背圧時は `try_send` 損失許容（`Progress` は絶対スナップショットなので間引いても最新位置を失わない）。ポーリング面（`state()`/`progress()`）は常に正確で、イベント面は push 最適化 — 二系統を独立して維持する。

## Data and Control Flow

1. `OfflineJob::spawn(op)`: 有界イベントチャネル・共有トークン/状態/進捗を生成 → `Started` を send → ワーカースレッド起動 → `catch_unwind` で本体を呼び `Result<T,JobError<E>>` を保持。
2. 本体は `JobScope` 経由で `is_cancelled()` を I/O ループ境界で確認し、`scope.progress().reset/bump` でフェーズ進捗を報告、`OfflineDriver` にはトークン+進捗をそのまま引き渡す。
3. `cancel()`: トークンを立て `Running→Cancelling`（冪等・Finished 済みは無視）。停止は協調的 — 本体が次の観測点で unwind する。
4. 終了: ワーカーが `Finished` を store し `Finished{outcome}` を try_send → チャネル close。`wait()` は join して結果を返す（`try_wait` 非ブロッキング、`detach` は join せず破棄）。
5. リソース cleanup: 本体が drop するリソースはスレッド終了＝`Finished` までに全解放。`Drop` は実行中ジョブに cancel を要求するが join しない（ブロッキング I/O を抱える本体を捨てた呼出し側を止めない）。
6. sample `run`: `probe`→ストリーム選択→デコードループ（フレームごと progress bump・`total=0` の不定進捗）→ `Vec<AudioChunk>` を `OfflineDriver` へ（チャンク単位の `completed/total`）→ `AnalysisReport` を JSON で stdout。イベントは stderr に JSONL で逐次送出。

## Non-Functional Constraints

- **RT/非 RT**: 追加 API はすべて `non-RT only`（スレッド spawn/join・チャネル・アロケーション）。`Meter`（`RtProcessor` 側）だけが RT 契約内（無アロケーション・チャンク長で有界）— `Processor`/`RtProcessor` 契約の退行なし。
- **スレッド・メモリ**: ジョブごとに `std::thread` ×1 + 有界チャネル + 共有 atomics。executor/プールなし。イベント背圧は容量 64 で打ち止め・メモリは定数上界。
- **決定論**: `OfflineJob` は本体の決定論を変更しない（ライフサイクル付与のみ）。ドライバ経路の出力決定論は Issue #24 ADR の保証範囲を継承（sample テストで workers=1 と並列の同一結果を検証）。
- **再入性**: ジョブは使い捨て（spawn ごと新規）。`spawn` 同士は独立。`Progress` リスナーは同期呼出し・ブロッキング禁止を doc で契約化。
- **テスト可能性**: 全ハンドルがインプロセス — テストはシグナル・サブプロセスなしで同面を駆動できる。`--cancel-after` は E2E でも決定的に cancel 経路を通せる検証フック（アプリ側で所有）。
- **panic**: 本体の panic は `catch_unwind` で `JobError::Panicked` に写像 — ハンドルを毒状態にしない。
- **依存追加の記録**: `clap`（引数パース・アプリ側責務）・`serde_json`（JSON レンダ）・`ctrlc`（Ctrl-C→cancel デモ）は sample クレートのみの直接依存で、基礎クレートへ新規外部依存なし（理由は `crates/ruviform-cli-sample/Cargo.toml` コメント）。
- `unsafe` 不使用・公開 enum は全て `#[non_exhaustive]`。

## Risks and Trade-offs

- `JobOutcome::Cancelled` は「Err かつトークン済」のヒューリスティック分類 — 失敗とキャンセルが競合した場合 Cancelled に分類される旨 doc に明記（厳密な区別が要るアプリは `JobError` を見る）。
- イベント損失許容は「最新位置を失わない絶対スナップショット + close を終端」とすることで正当化 — 順序厳密な履歴が要る用途には向かない旨 doc で明確化。
- `run` のデコードが `Vec<AudioChunk>` に全蓄積するのは巨大入力でメモリを食う — sample の範囲では割り切り、ストリーミング化は Deferred（Issue #24 と同じ境界判断）。
- `probe` をジョブ化していない（同期関数）— 観測が一瞬で終わる操作を非同期化する必然がなく、`run` がジョブ面の使い方を示す役割を担う。

## Deferred Design Concerns

- 複数ジョブの id 付きレジストリ・`status <id>`/`cancel <id>` 形式の長時間ジョブ制御 — アプリ所有（Issue 本文の責務境界どおり sample も実装しない）。
- ストリーミング decode→process パイプライン結合（`Vec` 全蓄積の解消）— エンジン層 Issue（#25/#27 系）の既存保留。
- async executor 向け `Future` ブリッジ — 実需が出た時点で `events()`/`state()`/`wait()` の薄い写像として追加可能。
- 終了コード・診断スキーマの標準語彙の Ruviform への昇格 — 各アプリ固有のため昇格しない方針を Issue で確定済み。

## Verification Notes

- `tests/pipeline.rs`（in-memory WAV/AVI fixture）: probe 成功/バックエンドルーティング、run 正常系・canonical/native 両スケール・並列=逐次の決定論、corrupt/未知ストリーム/IO エラー、トークン/途中キャンセル、進捗イベント系列。
- `tests/cli_e2e.rs`（実プロセス）: stdout に JSON 1 文書・stderr に JSONL イベント、exit 0/1/2/130、native、引数エラー。
- `job.rs` 単体テスト: spawn→wait 結果、Started→Progress→Finished→close、cancel→Cancelling→協調停止、panic→`JobError::Panicked`、Drop→cancel 要求・非 join、detach、状態/進捗ポーリング。
- headless 分離: `cargo tree -p ruviform-cli-sample` に GUI/WebView/デバイス/プラグイン crate が出ないことを確認済み（依存は clap/serde_json/ctrlc/symphonia + ruviform foundations のみ）。
- 検証列: `mise run fmt-check`/`check`/`lint`/`test`/`build` 全 pass。
