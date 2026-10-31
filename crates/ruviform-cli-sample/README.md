# ruviform-cli-sample

Issue #155 の利用例クレート: GUI 非依存の最小 CLI アプリケーションを、
**共有 Ruviform headless サービス面**の外側アダプタとして実装したもの。
`probe`（コンテナ・ストリーム事実の列挙）と `run`（オフライン
peak/RMS メータ解析）を持ち、GUI アダプタが同じ `probe` /
`spawn_analyze` を呼んで同じ `AnalysisReport` を描画できることを
示す。Ruviform 専用の CLI フレームワークではない — Issue #155 の
方針どおり、コマンド面は利用側サンプルとしての例に留まる。

## 責務（アプリ側 — Issue #155 責務境界の利用側）

- 共有サービス部（`src/lib.rs`）: メディアソースのバックエンド
  ルーティング（`RIFF....AVI ` → `ruviform-io-avi`、それ以外 →
  `ruviform-io-symphonia`）、`probe` / `spawn_analyze`、
  `ProbeReport` / `AnalysisReport` / `PipelineError` / `AnalyzeOptions`、
  メータカーネル `MeterStage`/`Meter`（`Processor`/`RtProcessor` 実装 —
  GUI エンジンが `prepare` するのと同一契約）、JSON レンダラ
  （`probe_json`/`report_json`/`error_json`/`event_json`）。
- CLI アダプタ部（`src/main.rs`）: clap 引数パース・`probe`/`run`
  コマンド名・stdout/stderr ポリシ・exit code 写像・Ctrl-C→
  `job.cancel()`・検証用 `--cancel-after` フック — すべてアプリ所有。
- 所有しないもの: ジョブレジストリ・`status`/`cancel <id>`・永続
  Manifest・`resume`・retry・診断スキーマ標準化 — 利用アプリの
  外部設計。

## 出力契約（sample 固有の選択 — 所有者は利用アプリ）

- **stdout**: 呼出しごとに JSON 1 文書（`probe`/`run` レポート、または
  `{"status":"failed"|"cancelled","error":{kind,message}}`）。
- **stderr**: `JobEvent` 1 件につき JSONL 1 行（`started`/`progress`/
  `finished`）+ clap の usage 診断。
- **exit code**: `0` 完了・`1` 操作失敗・`2` 引数/usage エラー（clap）・
  `130` キャンセル（SIGINT 慣例。`--cancel-after` でも同じ）。

## 層・許可依存

- 層: 利用側層（engine 同順位・engine ではない — `ruviform-edit` と同じ
  位置。実行・観測・I/O ポリシのアダプタ）。
- Ruviform 依存: `ruviform-exec`（`OfflineJob`/`OfflineDriver`/
  `CancellationToken`/`Progress`）・`ruviform-rt`（カーネル契約）・
  `ruviform-media-io` + `ruviform-io-avi`/`ruviform-io-symphonia`
  （demux/decode 境界と両バックエンド）・`ruviform-media-core`・
  `ruviform-audio-core`。
- アプリ側依存: `clap`（引数パース）・`serde_json`（JSON 文書）・
  `ctrlc`（SIGINT→cancel デモ）— 理由は `Cargo.toml` コメント。
  基礎クレートへの新規外部依存はない。
- GUI/デバイス/プラグイン/エンジンへの依存なし —
  `cargo tree -p ruviform-cli-sample` で検証済み（headless tree）。

## RT 契約

`Meter`（`RtProcessor` 半分）のみ RT 契約内（無アロケーション・チャンク
長で有界）。`MeterStage::prepare`・`probe`・`spawn_analyze`・JSON 系は
すべて `non-RT only`。デコード/`OfflineDriver` 駆動はジョブのワーカー
スレッド上で完結し、RT パスとは共有しない。

## ライフサイクル・キャンセル・決定論

- `run` は `OfflineJob` 1 本で駆動: デコード相は不定進捗
  （`total=0`・フレームごと bump）、ドライバ相は `completed/total`
  の確定進捗 — いずれも `JobEvent::Progress` で stderr に流れる。
- キャンセル経路は 3 つで同一: `Ctrl-C`（ctrlc ハンドラが共有
  `CancellationToken` を立てる）・`--cancel-after N`・`job.cancel()`。
  デコードは次のパケット境界、ドライバは次のチャンク境界で協調的に
  unwind し、`PipelineError::Cancelled` → exit 130。
- 決定論: `workers=1`（逐次）と `--workers N`（並列）は同一
  `AnalysisReport` を返す（`tests/pipeline.rs` で検証）。
- 中断時 cleanup: 本体所有リソースはワーカー終了時に全解放
  （`Finished`＝スレッド終了が合図）。`OfflineJob::Drop` は cancel を
  要求するが join しない — ブロッキング中の本体を捨てる側を止めない。

## 関連

- 設計: docs/designs/issue-155-headless-cli-foundation.md
- オフライン駆動設計: docs/designs/offline-execution-driver.md /
  docs/adr/offline-determinism-scope.md（決定論保証範囲の継承元）
- 観測契約（`Fact<T>`・native scale）: Issue #151、バックエンド: Issue #154
- Issue #155（本クレート新設）・AudioLens 移行 audio-lens#95
