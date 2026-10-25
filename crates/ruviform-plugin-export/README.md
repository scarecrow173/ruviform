# ruviform-plugin-export

プラグイン側・L1 子プロセスランナー（Issue #76 Task 2）。

## 責務

- `run(Vec<Box<dyn PluginFactory>>) -> ExitCode` — プラグインバイナリの `main` から呼ぶ唯一の公開入口。argv を解釈し、stdio コントロールプレーン上で wire プロトコル（`ruviform-plugin-core::wire`）を駆動し、`--ruviform-shm-key` で指定された共有メモリデータプレーンをアタッチして単一プラグインインスタンスをホストする。
- `src/bin/ruviform-plugin-l1-runner.rs` — リファレンス runner。決定的な `GainPlugin`（`dev.ruviform.mock.gain`）と panic fixture `PanicPlugin`（`dev.ruviform.mock.panic`）を内蔵し、E2E（`tests/l1_runner_e2e.rs`）の spawn 対象となる。

## argv 契約

```
--ruviform-plugin-id <id>   factories のうち選択対象のプラグイン ID
--ruviform-shm-key <key>    親が作成済みの共有メモリセグメント名（attach のみ・作成しない）
```

起動直後に `Hello { version: WIRE_VERSION, capabilities: 0 }` を最初のフレームとして emit する。バージョンネゴシエーションは親側の責務。

## 終了コード

| code | 意味 |
| --- | --- |
| 0 | `Shutdown` またはクリーンな stdin EOF（lifecycle を best-effort 正規化して終了） |
| 1 | トランスポート/プロトコル/共有メモリの失敗（malformed frame、制限超過、attach/ヘッダ検証失敗、子側のみの inbound kind 等） |
| 2 | argv usage エラー（`Hello` 送出前） |

## 設計境界

- **非 RT**: この runner 全体（stdio ループ、shm ステージング）は非リアルタイムスレッドで動作する。RT 契約はホスト側のデータプレーンタスクの責務。
- **shared_memory の封じ込め**: `shared_memory` クレートは内部 `shm` モジュールに閉じ込め、公開 API に露出しない。子はセグメントを attach/検証するのみで作成・unlink しない（所有は親）。
- **unsafe**: `shared_memory 0.12.4` の `as_slice`/`as_mut_slice` が unsafe fn のため、`shm.rs` 内に SAFETY コメント付き `#[expect(unsafe_code)]` の局所例外を置く（設計の「runner-owned unsafe ゼロ」想定からの逸脱 — plan「依存の確定」に記録）。

## 層構造

plugin 拡張層。依存: `ruviform-plugin-core` のみ（+ 内部 `shared_memory`）。詳細は `docs/designs/plugin-l1-out-of-process.md` と `docs/tasks/plugin-export-crate-and-l1-runner.md`。
