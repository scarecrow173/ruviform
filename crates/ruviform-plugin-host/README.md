# ruviform-plugin-host

Ruviform メディア基盤のプラグインホスト層: ホストアプリケーションと
プラグインインスタンスの間の規格中立境界。プラグイン分離設計
（Issue #31 → `docs/designs/plugin-isolation.md`,
`docs/adr/plugin-isolation-strategy.md`）に基づき、L0（インプロセス、
デフォルト — Issue #74）と L1（opt-in アウトプロセス — Issue #76 →
`docs/designs/plugin-l1-out-of-process.md`）の 2 バックエンドを実装する。

## 責務

- 境界: 呼び出し側は plain data — `HostCommand`/`HostReply`/`HostError` と
  不透明 `PluginHandle` — のみを見る。非 RT 操作（スキャン・インスタンス化・
  `LifecycleEvent` 駆動のライフサイクル動詞・状態保存/復元・GUI open/close）は
  全てこの一面を通る。
- `PluginHost` は 2 バックエンドを単一 `PluginHandle` 空間の後ろに統合する
  コンポジット: `register_factory` が `plugin_id` を L0 に、
  `register_out_of_process(OutOfProcessSpec)` が L1 に登録時点で claim し、
  `Instantiate { plugin_id }` の呼び出し形は変わらない — 呼び出し側は層を
  意識しない。`plugin_id` の重複 claim（順序・ルート不問）は
  `Err(HostError::DuplicatePluginId)` の defined rejection。
- L1 経路: `OutOfProcessSpec::isolation` で子プロセス形態を選択 — `Bundled`
  （既定）は route の pool が共有子へ最大 `WIRE_MAX_INSTANCES` 個の
  インスタンスを詰め、`Dedicated` は `Instantiate` 毎に専用子を spawn する。
  stdio の framed コントロールプレーン + `shared_memory` データプレーン +
  `ruviform-rt` SPSC try 脚。ライブ中の子プロセス終了はその子がホストする
  各 live インスタンスを `Failed` へ収束させ、
  `PluginHost::try_recv_notification` が影響を受けたインスタンス毎に
  `HostNotification::InstanceFailed` を 1 つずつ報告する（bundled route の
  子 kill = N 通知列 — 粒度正本は
  `docs/adr/issue-117-bundle-child-failed-granularity.md`）。`Failed`
  インスタンスは `Destroy` 以外の全コマンドを `Err(HostError::Failed)` で
  拒否し、`process` は `Ok(CONTINUE)` + 無音を返す。
- L1 video 経路（Issue #118 — wire v3 の `Command` tag 12-22・`Reply`
  tag 4-7）: `PluginHost::video() -> VideoPluginHost<'_>` が `&mut` 借用
  façade を貸し、`VideoHostCommand`/`VideoHostReply`（`#[non_exhaustive]`）
  が audio `HostCommand`/`HostReply` と並立する別コマンド面を構成する
  （`Scan`・`Instantiate{plugin_id, config}`・lifecycle 動詞・`Render`・
  plain-param 動詞）。登録は `video().register_out_of_process(
  VideoOutOfProcessSpec)` — `VideoOutOfProcessSpec` は `isolation`・
  `lookahead_blocks` 字段を**持たない**構造 pin で Dedicated 固定
  （pooled video route は表現不能）。内部は `Route::OutOfProcessVideo`/
  `Slot::OutOfProcessVideo`（`DescriptorEntry` の kind-tag で audio/video
  registration を分離 — `plugin_id` は単一名前空間で cross-kind `Instantiate`
  は `Err(UnknownPluginId)`・登録衝突は `Err(DuplicatePluginId)`・cross-kind
  動詞は `Err(Unsupported)`）。`l1::video` の `VideoL1Instance` は `L1Child`
  共有機構の上に乗る audio データ面なしの instance façade（RtLeg・staging
  ring なし）、`l1::vshm` は host 所有の video セグメント（`wire::vshm`
  レイアウト — in/out frame 領域・`shared_memory` は `l1::shm` + ここに
  封じ込め）。`Instantiate` の `VideoProcessConfig` は host 側 bound 検査
  （`Err(Prepare(UnsupportedConfig))` — spawn/vshm 作成より前）、`Render`
  の `input` は instance config と照合して不一致は wire 非送信の
  `Err(InvalidValue)`、`SetParamValue` の非有限 `Double` も同じく
  `Err(InvalidValue)`。reply wait の timeout は `command_timeout`（既定
  10 s・handshake/lifecycle/param 動詞）と `render_timeout`（既定 60 s・
  `Render` のみ）に分離される。video instance の `Err` reply は mirror を
  変えない（`Err` reply mirror 不変 — audio doorbell の `Err`→`Failed`
  収束とは意図的に異なる L0-faithful 規則）。`Failed` 粒度・
  `InstanceFailed` exactly-once fan-out は #117 ADR 規則を video にも
  そのまま適用する。video 向け検証 bin は `src/bin/l1-test-video-worker`
  （`[[bin]]` — wire v3 最小 video worker）、公開 E2E は
  `tests/l1_video_e2e.rs`。
- `process` は分離設計上コマンド**ではない**: L0 では `PluginProcessor` への
  直接呼び出し、L1 では shm ステージング + try 脚のみのデータプレーン —
  どちらのルートでも呼び出しスレッドは pipe/shm/ブロッキング I/O に触れない。
- 遅延契約（L1）: 投入したブロックは `lookahead_blocks` ブロック後に回答
  される — 遅延 = `lookahead_blocks × activated max_frames`（frames 整数。
  秒換算は `frames / sample_rate` — 既定 2 × 512 フレーム @ 48 kHz
  ≈ 21.3 ms、`OutOfProcessSpec.lookahead_blocks` で設定。ブロック長は
  可変のため報告量子は activated `max_frames` — #113 で確定）。
  期限内に回答が届かない場合もエラーではなく、defined pass-through
  （直前に受信したブロック、初回前は無音）が出力される。
- プラグイン側: `Plugin` は `PluginProcessor` を非 RT インスタンスフック
  （`init`・処理通知・状態・GUI — 全てデフォルト実装付きで、最小プラグインは
  記述子・パラメータ・処理契約のみ実装すればよい）で拡張する。
  `PluginFactory` は規格アダプタが `register_factory` へ渡す登録単位。

## 層・許可依存

- 層: 拡張層（Host 層、設計ベースライン）
- 許可依存: `ruviform-plugin-core`（およびそれが使う foundations の推移的
  依存）+ `ruviform-rt`（Primitives — L1 RT 脚の SPSC）+ `ruviform-audio-core`
  （#113 prod 昇格 — `endpoint.rs` の `AudioBufferViewMut` 名指し）+
  `ruviform-media-core`（Issue #118 dev-dep → prod 昇格 — `video.rs` の公開
  `Render{at: MediaTime}` 動詞と `l1::video` が名指す。#113 と同経路）+
  `ruviform-video-core`（Issue #118 追加 — `VideoFrame` が公開
  `VideoHostCommand::Render{input}`/`VideoHostReply::Rendered` 語彙に出て
  `l1::video`/`l1::vshm` が plane を repack/復元。Domain foundations で
  plugin-core 経由の推移依存を直接化するのみ）+ `shared_memory`（L1 named
  共有メモリ — 内部 `l1::shm`・
  `l1::vshm` モジュールに封じ込め、公開 API・feature 名に露出しない）。
  規格固有・binding クレートの型はここに現れない —
  `ruviform-plugin-clap` / `-vst3` が `Plugin`/`PluginFactory` へ適合する。
- `l1` モジュールは `pub(crate)` — L1 の spawn/handshake・データ面・通知は
  `PluginHost` 経由でのみ到達可能。

## RT 契約

`InProcessHost::process`/`plugin_mut`/`lifecycle_state` は RT-suitable —
`const` ライフサイクルテーブル参照 + 直接呼び出しで、アロケーション・
ブロッキング・IPC なし。`PluginHost::process` は両ルートで RT-suitable
（L1 = `try_push`/`try_pop` + 事前確保ステージングへの bounded コピーのみ）。
`PluginHost::plugin_mut` は RT-suitable だが L1 スロットでは `None`
（アウトプロセスインスタンスには lend できる `ErasedPlugin` が存在しない）。
`PluginHost::lifecycle_state` は L0 スロットでは const-cheap、L1 スロットでは
共有 mutex を取るため非 RT 扱い。コマンドディスパッチ（`execute`・
`register_*`・`try_recv_notification`）は**非 RT 専用**。video 面
（`VideoPluginHost` の全動詞・`Render` 含む）も**全操作 NONRT** —
video instance は RT leg・endpoint 経路を持たず、`prepare_endpoint`/
`process`/`plugin_mut` への video handle は `Err(EndpointUnavailable)`/
`Err(Unsupported)`/`None` の defined 答え。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 `ruviform-plugin-host` 行）
- 分離設計（Issue #31）: docs/designs/plugin-isolation.md, docs/adr/plugin-isolation-strategy.md
- L0 ホスト設計（Issue #74）: docs/designs/plugin-host-l0.md
- L1 アウトプロセス設計（Issue #76）: docs/designs/plugin-l1-out-of-process.md, docs/specs/plugin-l1-out-of-process.md
- L1 video 設計（Issue #118 — wire v3）: docs/designs/issue-118-video-l1.md, docs/specs/issue-118-video-l1.md, docs/adr/issue-118-video-l1-host-surface.md
