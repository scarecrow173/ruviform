# ruviform-plugin-export

プラグイン側・L1 子プロセスランナー（Issue #76 Task 2）。

## 責務

- `run(Vec<Box<dyn PluginFactory>>) -> ExitCode` — プラグインバイナリの `main` から呼ぶ唯一の公開入口。stdio コントロールプレーン上で wire プロトコル（`ruviform-plugin-core::wire`、wire v2）を駆動し、`instances` レジストリで最大 `WIRE_MAX_INSTANCES`（64）個のプラグインインスタンスを 1 子プロセス内にホストする。インスタンスの登録は `CreateInstance` payload（plugin id + shm セグメント名）で行い、インスタンス毎の共有メモリデータプレーンをアタッチする。親側では `ruviform_plugin_host::PluginHost` コンポジットがこのバイナリを `OutOfProcessSpec.program` として登録し、`OutOfProcessSpec::isolation` に従い route の pool 経由で spawn・再利用する（既定 `Bundled` — `Dedicated` は `Instantiate` 毎に専用子を spawn）。
- `src/bin/ruviform-plugin-l1-runner.rs` — リファレンス runner。決定的な `GainPlugin`（`dev.ruviform.mock.gain`）と panic fixture `PanicPlugin`（`dev.ruviform.mock.panic`）を内蔵し、E2E（`tests/l1_runner_e2e.rs`）の spawn 対象となる。

## argv 契約

```
<runner>
```

`run` ベースの runner は**引数を取らない**。wire v2 で `--ruviform-plugin-id`/`--ruviform-shm-key` の bootstrap フラグは廃止 — プラグイン id と shm セグメント名はインスタンス毎の `CreateInstance` payload が運ぶ。`--ruviform-*` を含む任意の argv 入力は usage エラー（stderr + exit 2・`Hello` 非送出）。host は `spec.args` を runner の argv へそのまま前置するため、`run` ベースの runner では `OutOfProcessSpec::args` は空を維持する。

起動直後に `Hello { version: WIRE_VERSION, capabilities: 0 }` を最初のフレームとして emit する。バージョンネゴシエーションは親側の責務。

**factory 選択は `plugin_id` のみ**: `run` は `CreateInstance` payload の `plugin_id` で内蔵 factory を選択する（host は handshake の `Scanned` 応答を descriptor 全体で field-match するが、wire の `CreateInstance` が運ぶのは `plugin_id` のみ）。同一 id を持つ factory を 1 binary に 2 つ内蔵するのは authoring error — `position` が先頭一致を返すため 2 個目は到達不能。

## 終了コード

| code | 意味 |
| --- | --- |
| 0 | `Shutdown` またはクリーンな stdin EOF（lifecycle を best-effort 正規化して終了） |
| 1 | トランスポート/プロトコルの失敗（malformed frame、制限超過、decode 失敗、子側のみの inbound kind 等）— shm attach/header 検証失敗は終了ではなく `CreateInstance` への `Err(HostError::Ipc)` 応答 |
| 2 | argv usage エラー（`Hello` 送出前） |

## 設計境界

- **非 RT**: この runner 全体（stdio ループ、shm ステージング）は非リアルタイムスレッドで動作する。RT 契約はホスト側のデータプレーンタスクの責務。
- **shared_memory の封じ込め**: `shared_memory` クレートは内部 `shm` モジュールに閉じ込め、公開 API に露出しない。子はセグメントを attach/検証するのみで作成・unlink しない（所有は親）。
- **unsafe**: `shared_memory 0.12.4` の `as_slice`/`as_mut_slice` が unsafe fn のため、`shm.rs` 内に SAFETY コメント付き `#[expect(unsafe_code)]` の局所例外を置く（設計の「runner-owned unsafe ゼロ」想定からの逸脱 — plan「依存の確定」に記録）。

## 層構造

plugin 拡張層。依存: `ruviform-plugin-core` のみ（+ 内部 `shared_memory`）。詳細は `docs/designs/plugin-l1-out-of-process.md` と `docs/tasks/plugin-export-crate-and-l1-runner.md`。
