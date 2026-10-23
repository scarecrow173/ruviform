# ruviform-plugin-host

Ruviform メディア基盤のプラグインホスト層: ホストアプリケーションと
プラグインインスタンスの間の規格中立境界。プラグイン分離設計
（Issue #31 → `docs/designs/plugin-isolation.md`,
`docs/adr/plugin-isolation-strategy.md`）に基づき、L0 レベル（インプロセス、
デフォルトパス）として実装される（Issue #74）。

## 責務

- 境界: 呼び出し側は plain data — `HostCommand`/`HostReply`/`HostError` と
  不透明 `PluginHandle` — のみを見る。非 RT 操作（スキャン・インスタンス化・
  `LifecycleEvent` 駆動のライフサイクル動詞・状態保存/復元・GUI open/close）は
  全てこの一面を通り、後続のアウトプロセス（L1）バックエンドが呼び出し側の
  変更なしに同一コマンドを提供できる。
- `process` は分離設計上コマンド**ではない**: L0 では `PluginProcessor` への
  直接呼び出しのままで、`InProcessHost::process`/`InProcessHost::processor`
  が IPC ・ブロッキングなしで公開する。
- プラグイン側: `Plugin` は `PluginProcessor` を非 RT インスタンスフック
  （`init`・処理通知・状態・GUI — 全てデフォルト実装付きで、最小プラグインは
  記述子・パラメータ・処理契約のみ実装すればよい）で拡張する。
  `PluginFactory` は規格アダプタが `InProcessHost::register_factory` へ渡す
  登録単位。

## 層・許可依存

- 層: 拡張層（Host 層、設計ベースライン）
- 許可依存: `ruviform-plugin-core`（およびそれが使う foundations の推移的
  依存）のみ。規格固有・binding クレートの型はここに現れない —
  `ruviform-plugin-clap` / `-vst3` が `Plugin`/`PluginFactory` へ適合する。

## RT 契約

`InProcessHost::process`, `InProcessHost::plugin_mut`,
`InProcessHost::lifecycle_state` は RT-suitable — `const` ライフサイクル
テーブル参照 + 直接呼び出しで、アロケーション・ブロッキング・IPC なし。
コマンドディスパッチ（`execute`, `register_factory`）は**非 RT 専用**。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 `ruviform-plugin-host` 行）
- 分離設計（Issue #31）: docs/designs/plugin-isolation.md, docs/adr/plugin-isolation-strategy.md
- L0 ホスト設計（Issue #74）: docs/designs/plugin-host-l0.md
