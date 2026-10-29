# ruviform-plugin-export

プラグイン側・L1 子プロセスランナー（Issue #76 Task 2・video 経路は Issue #118）。

## 責務

- `run(Vec<Box<dyn PluginFactory>>) -> ExitCode` — **audio** プラグインバイナリの `main` から呼ぶ L1 runner entry（video の `run_video` と parallel な 2 公開入口の片方）。stdio コントロールプレーン上で wire プロトコル（`ruviform-plugin-core::wire`、現行 v3 — audio codec は tag 0-11/0-3 で不変）を駆動し、`instances` レジストリで最大 `WIRE_MAX_INSTANCES`（64）個のプラグインインスタンスを 1 子プロセス内にホストする。インスタンスの登録は `CreateInstance` payload（plugin id + shm セグメント名）で行い、インスタンス毎の共有メモリデータプレーンをアタッチする。親側では `ruviform_plugin_host::PluginHost` コンポジットがこのバイナリを `OutOfProcessSpec.program` として登録し、`OutOfProcessSpec::isolation` に従い route の pool 経由で spawn・再利用する（既定 `Bundled` — `Dedicated` は `Instantiate` 毎に専用子を spawn）。
- `run_video(Vec<Box<dyn VideoPluginFactory>>) -> ExitCode` — **video** runner entry（Issue #118・wire v3）。`run` と同じ framed pipe・`Hello`/`Scan`/`Shutdown` handshake・argv 拒否・exit 規約・panic 正規化を共有し、差分は動詞系統（`CreateVideoInstance` + video tag 12-22）と doorbell データ面の不在のみ — video は `VideoRender` が frame descriptor をコマンド内に運ぶ message-driven で、子は `BlockIn`/`BlockDone` を持たない（inbound `BlockIn` は direction-legal な foreign traffic として静かに drop・inbound `BlockDone`/`Hello`/`Reply` は direction-illegal で exit 1 の従来規約）。`CreateVideoInstance` payload の `VideoProcessConfig` は registry entry が stash し、`VideoInitialize` 時に `ErasedVideoPlugin::initialize` へ replay する。親側の登録面は `VideoOutOfProcessSpec.program`（Dedicated 固定 — pool なし）。
- `src/bin/ruviform-plugin-l1-runner.rs` — リファレンス runner。決定的な `GainPlugin`（`dev.ruviform.mock.gain`）と panic fixture `PanicPlugin`（`dev.ruviform.mock.panic`）を内蔵し、E2E（`tests/l1_runner_e2e.rs`）の spawn 対象となる。
- `src/bin/ruviform-plugin-l1-video-runner.rs` — video リファレンス runner（`run_video` 駆動）。決定的 transform `InvertVideo`（`dev.ruviform.mock.video.invert` — 反転/フラット fill + plain-param 面・`EchoWidth` で stash/replay が wire 観測可能）・panic fixture `PanicVideo`（`dev.ruviform.mock.video.panic` — `initialize`/factory `descriptor()`/`create()` の panic → `PluginFailed` 正規化）・encode obligation fixture `OverLimitVideo`/`OverLimitInfoVideo`（`dev.ruviform.mock.video.overlimit`/`overlimit-info` — over-limit params/metadata → `PluginFailed`）の 4 factory を内蔵し、E2E（`tests/l1_video_runner_e2e.rs`）の spawn 対象となる。
- 内部モジュール: `shm`（audio slot セグメントの attach/検証・`wire::shm` レイアウト）と `vshm`（video in/out frame 領域セグメントの attach/検証・`wire::vshm` 定盤 — 別レイアウト・別マジック `"RVLV"`）。`shared_memory` はこの 2 モジュールに閉じ込める。

## argv 契約

```
<runner>
```

`run`/`run_video` ベースの runner は**引数を取らない**。v1 の `--ruviform-plugin-id`/`--ruviform-shm-key` bootstrap フラグは wire v2（Issue #117）で廃止済み — プラグイン id と shm セグメント名はインスタンス毎の `CreateInstance`/`CreateVideoInstance` payload が運ぶ。`--ruviform-*` を含む任意の argv 入力は usage エラー（stderr + exit 2・`Hello` 非送出）。host は `spec.args` を runner の argv へそのまま前置するため、`run*` ベースの runner では `OutOfProcessSpec::args`/`VideoOutOfProcessSpec::args` は空を維持する。

起動直後に `Hello { version: WIRE_VERSION, capabilities: 0 }` を最初のフレームとして emit する。バージョン交渉機構は存在しない — `Hello.version` は `WIRE_VERSION` との厳密一致で親側が検証し（Issue #119 の単一最新版方針）、不一致は handshake 失敗（child kill + `HostError::Ipc`）として扱われる。

**factory 選択は `plugin_id` のみ**: `run`/`run_video` は create payload の `plugin_id` で内蔵 factory を選択する（host は handshake の `Scanned` 応答を descriptor 全体で field-match するが、wire の `CreateInstance`/`CreateVideoInstance` が運ぶのは `plugin_id` のみ）。同一 id を持つ factory を 1 binary に 2 つ内蔵するのは authoring error — `position` が先頭一致を返すため 2 個目は到達不能。

## 終了コード

| code | 意味 |
| --- | --- |
| 0 | `Shutdown` またはクリーンな stdin EOF（lifecycle を best-effort 正規化して終了） |
| 1 | トランスポート/プロトコルの失敗（malformed frame、制限超過、decode 失敗、子側のみの inbound kind 等）— shm attach/header 検証失敗は終了ではなく `CreateInstance` への `Err(HostError::Ipc)` 応答 |
| 2 | argv usage エラー（`Hello` 送出前） |

## 設計境界

- **非 RT**: この runner 全体（stdio ループ、shm ステージング）は非リアルタイムスレッドで動作する。RT 契約はホスト側のデータプレーンタスクの責務。
- **shared_memory の封じ込め**: `shared_memory` クレートは内部 `shm`/`vshm` モジュールに閉じ込め、公開 API に露出しない。子はセグメントを attach/検証するのみで作成・unlink しない（所有は親）。
- **unsafe**: `shared_memory 0.12.4` の `as_slice`/`as_mut_slice` が unsafe fn のため、`shm.rs`/`vshm.rs` に SAFETY コメント付き `#[expect(unsafe_code)]` の局所例外を置く（設計の「runner-owned unsafe ゼロ」想定からの逸脱 — plan「依存の確定」に記録）。

## 層構造

plugin 拡張層。依存: `ruviform-plugin-core`・`ruviform-video-core`・`ruviform-media-core`（Issue #118 追加 — `ErasedVideoPlugin`/`VideoPluginFactory`/`VideoFrame`/`MediaTime` を `run_video` 面が名指すため。`ruviform-audio-core` の #113 dev-dep → prod 昇格と同経路）+ 内部 `shared_memory`。詳細は `docs/designs/plugin-l1-out-of-process.md`・`docs/designs/issue-118-video-l1.md` と `docs/tasks/plugin-export-crate-and-l1-runner.md`。
