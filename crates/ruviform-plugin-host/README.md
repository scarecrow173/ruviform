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
- L1 経路: インスタンスごとに 1 子プロセス。stdio の framed コントロール
  プレーン + `shared_memory` データプレーン + `ruviform-rt` SPSC try 脚。
  ライブ中の子プロセス終了はインスタンスを `Failed` へ収束させ、
  `PluginHost::try_recv_notification` が
  `HostNotification::InstanceFailed` を一度だけ報告する。`Failed`
  インスタンスは `Destroy` 以外の全コマンドを `Err(HostError::Failed)` で
  拒否し、`process` は `Ok(CONTINUE)` + 無音を返す。
- `process` は分離設計上コマンド**ではない**: L0 では `PluginProcessor` への
  直接呼び出し、L1 では shm ステージング + try 脚のみのデータプレーン —
  どちらのルートでも呼び出しスレッドは pipe/shm/ブロッキング I/O に触れない。
- 遅延契約（L1）: 投入したブロックは `lookahead_blocks` ブロック後に回答
  される — 遅延 = `lookahead_blocks × block_frames / sample_rate`（既定
  2 × 512 フレーム @ 48 kHz ≈ 21.3 ms、`OutOfProcessSpec.lookahead_blocks`
  で設定）。期限内に回答が届かない場合もエラーではなく、defined pass-through
  （直前に受信したブロック、初回前は無音）が出力される。
- プラグイン側: `Plugin` は `PluginProcessor` を非 RT インスタンスフック
  （`init`・処理通知・状態・GUI — 全てデフォルト実装付きで、最小プラグインは
  記述子・パラメータ・処理契約のみ実装すればよい）で拡張する。
  `PluginFactory` は規格アダプタが `register_factory` へ渡す登録単位。

## 層・許可依存

- 層: 拡張層（Host 層、設計ベースライン）
- 許可依存: `ruviform-plugin-core`（およびそれが使う foundations の推移的
  依存）+ `ruviform-rt`（Primitives — L1 RT 脚の SPSC）+ `shared_memory`
  （L1 named 共有メモリ — 内部 `l1::shm` モジュールに封じ込め、公開 API・
  feature 名に露出しない）。規格固有・binding クレートの型はここに現れない —
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
`register_*`・`try_recv_notification`）は**非 RT 専用**。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 `ruviform-plugin-host` 行）
- 分離設計（Issue #31）: docs/designs/plugin-isolation.md, docs/adr/plugin-isolation-strategy.md
- L0 ホスト設計（Issue #74）: docs/designs/plugin-host-l0.md
- L1 アウトプロセス設計（Issue #76）: docs/designs/plugin-l1-out-of-process.md, docs/specs/plugin-l1-out-of-process.md
