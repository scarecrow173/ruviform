---
id: "DISC-034LNekQmplu83WT5PUMzs"
type: "discovery"
status: "resolved"
title: "プラグイン L1 アウトプロセス分離（plugin-host L1 プロキシ + plugin-export 薄ランナー）の探索（Issue #76）"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/31"
    - "https://github.com/scarecrow173/ruviform/issues/74"
    - "https://crates.io/crates/iceoryx2"
    - "https://lib.rs/crates/shared_memory"
    - "https://lib.rs/crates/raw_sync"
    - "https://lib.rs/crates/ipc-channel"
    - "https://lib.rs/crates/interprocess"
    - "https://lib.rs/crates/postcard"
    - "https://pranitha.dev/posts/rust-ipc-ping-pong/"
    - "https://vadosware.io/post/yet-another-ipc-in-rust-experiment/"
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
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/plugin-architecture-clap-vst3.md"
    - "../designs/rt-foundation-api-design.md"
    - "evaluate-rt-foundation-primitives.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/plugin-isolation.md"
    - "../adr/plugin-isolation-strategy.md"
    - "../specs/plugin-isolation-design.md"
    - "../specs/plugin-host-l0.md"
    - "../designs/plugin-host-l0.md"
    - "../designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "../specs/plugin-l1-out-of-process.md"
    - "../designs/plugin-l1-out-of-process.md"
    - "engine-plugin-host-integration.md"
    - "issue-114-real-adapter-l1-e2e.md"
    - "../specs/issue-114-real-adapter-l1-e2e.md"
    - "../designs/issue-115-l1-gui-cross-process-parenting.md"
    - "issue-117-bundle-child-process-discovery.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# プラグイン L1 アウトプロセス分離（plugin-host L1 プロキシ + plugin-export 薄ランナー）の探索（Issue #76）

## Exploration Goal

Issue [#76](https://github.com/scarecrow173/ruviform/issues/76) は、Issue #31（spec [plugin-isolation-design.md](../specs/plugin-isolation-design.md) → ADR [plugin-isolation-strategy.md](../adr/plugin-isolation-strategy.md)・design [plugin-isolation.md](../designs/plugin-isolation.md)・いずれも確定済み）の実装単位分割に基づく後続実装 Issue — L0（インプロセス既定）は #74（spec/design [plugin-host-l0.md](../specs/plugin-host-l0.md)・[plugin-host-l0.md](../designs/plugin-host-l0.md)・CLOSED・`crates/ruviform-plugin-host` 実装済み）で確定したハンドル + 直列化コマンド境界を基盤に、**opt-in の L1 アウトプロセス経路**を要求する。

Issue 本文と上流文書が既に固定している構造: L1 プロキシは `ruviform-plugin-host` に実装（子プロセス spawn・監視・共有メモリ SPSC）、子プロセス側エントリは `ruviform-plugin-export` の薄いランナー（クレート新設を含む）、L1 の `process` 等価経路は**非 RT**（RT スレッドは結果を `try_recv` で読むのみ）、クラッシュ検出は子プロセス wait/exit status → `Failed` 遷移 + ホスト通知・自動再起動なし、遅延は先行ブロック数 × ブロック長、クラッシュ・応答なし時は pass-through（既受信データ or 無音）。

本探索は **issue framing・確認済みギャップ・IPC 機構候補の比較・クラッシュ検出/失敗表現/ランナー形態の選択肢整理**を記録する。採否・方式の確定権限は design フェーズにあり、本書は最終判断を含めない。

## Key Issues and Assumptions

### 上流で確定済み（confirmed — design 論点ではない）

- **L1 の形状**: opt-in のみ・L0 が既定のまま・呼出し側は `PluginHandle` + `HostCommand`/`HostReply`/`HostError` の同一公開面を見る（#31 境界契約・#74 spec 要件 2・plugin-host `command.rs` doc が L1 直列化を明示的に予約）
- **`process` の分岐**: L0 は `InProcessHost::process` の直接呼出し（RT-suitable・実測 0 alloc）、L1 は共有メモリリングで audio block を送受信し**非 RT スレッドが駆動**、RT スレッドは `try_recv` で結果を読むのみ — プロセス間通信を直接待機しない
- **クラッシュ契約**: 子プロセス監視（wait/exit status）→ インスタンスを `Failed` へ遷移・ホスト通知・自動再起動しない（再起動判断と状態復元は呼出し側/プラグイン責務）
- **遅延契約**: 先行ブロック数 × ブロック長。クラッシュ・応答なし時は pass-through（直前の既受信データ or 無音）
- **`ruviform-plugin-export` の責務枠**: ベースライン A.2 で「制作側: Ruviform 製プラグインのビルド・エクスポート基盤（ライフサイクル実装・規格準拠エントリ生成）」として予約済み — L1 子プロセスエントリは「エントリ生成」責務の一形態として整合。許可依存は Domain foundations + Primitives + 拡張層（`ruviform-plugin-core`）

### 確認済みギャップ（confirmed — 前提が最初に崩れる地点）

- **`ruviform-plugin-export` は未存在**: ルート `Cargo.toml` の `members`・`crates/README.md`（計 20 クレート）ともに不在。本 Issue が新設を含む
- **`LifecycleState` に `Failed` がない**: plugin-core `lifecycle.rs` は `Created`/`Initialized`/`Activated`/`Processing`/`Destroyed` の 5 状態のみ。設計の「`Failed` 状態へ遷移」を表現するには (i) plugin-core 状態機械の拡張（`#[non_exhaustive]` により non-breaking）または (ii) host 側のインスタンス単位フラグ（`lifecycle_state()` が `Failed` を報告できない制約あり）が必要 — 表現の所有境界は design 論点
- **`HostCommand`/`HostReply` に直列化機構がない**: 型は「直列化可能な純データ」として設計済みだが serde derive 等は未付与。特に `HostReply::Scanned(Vec<PluginDescriptor>)` の `PluginDescriptor` は全フィールド `&'static str`/`&'static [&'static str]`（plugin-core `descriptor.rs`）— 参照はプロセスをまたげず、受信側で所有文字列 or 借用ビュー（`PluginDescriptorView`）への写像が要。`ProcessConfig`/`PluginHandle`/`GuiParentToken` 等は `Copy` 値型で素直に marshal 可能
- **共有メモリ・プロセス間プリミティブがワークスペースにない**: `ruviform-rt` の `channel`（`spsc.rs` — `rtrb` ラップの heap 上 SPSC）はインプロセス専用で IPC には使えないが、L1 ドライバスレッド → RT スレッドの**プロセス内**結果受け渡し経路としては再利用候補（plugin-host の許可依存に Primitives は含まれる）。`Cargo.lock` に shm/IPC 系クレートは未導入（serde/serde_json は推移的に存在・直接依存ではない）
- **L1 ドライバのスレッド所有**: ADR は「エンジン側は非 RT スレッドから駆動（ベースラインの非 RT 制御スレッド層）」とするが、`ruviform-audio` は plugin-host を未消費 — L1 プロキシがインスタンス毎にドライバスレッドを所有するのか、`ruviform-exec`（非 RT 実行基盤）へ載せるのかは design 論点
- **ワイヤプロトコルの所有境界**: L1 proxy（host 側）と runner（export 側）が共有するプロトコル型をどこが持つか。ベースライン A.2 の許可依存は plugin-host = 拡張層（plugin-core）のみ・plugin-export = 同左で、host・export は同順位 — 相互依存は層ルール上の要確認事項。共有プロトコルを plugin-core へ置くか、同順位依存を許容と解釈するか、wire 型を各側で定義するかは design 論点
- **`InProcessHost` は具象型のみ**: `Send` 不要求・trait 抽象なし。L0/L1 の選択面を共通 trait・enum・並存具象型のどれにするかは design 論点（呼出し側が層を意識しない契約は維持）
- **暫定規約との緊張**: `unsafe` 原則禁止（`-D warnings` ゲートで実質 deny）・default feature 最小・依存追加は理由記録。shm IPC は内部が `unsafe` 主体の外部クレート頼みになるか、自前実装は局所許可コストが高い — `rtrb` 先例（安全 API を持つ外部クレートを内部依存に閉じ込める）の適用が論点
- **L1 GUI**: `OpenGui`/`CloseGui` をプロセス境界越しに実行するには親ウィンドウ埋め込みの規格・プラットフォーム固有課題がある。#30 の GUI PoC はインプロセス前提 — L1 での GUI は defined rejection or スコープ外の判断が要（Issue 本文も GUI に言及せず）

### 依存メタデータ（観測値 — 採否証明ではない）

宣言値は package メタデータ由来で実コンパイル証明ではない。保守状況・依存木・実効 MSRV の確証は採用前に別途確認する。

| crate | 観測 version | declared MSRV | license | 依存（宣言） | 構造 |
| --- | --- | --- | --- | --- | --- |
| `iceoryx2` | 0.10.0（2026-09 更新・活発） | 1.89（宣言 — workspace MSRV と一致） | MIT OR Apache-2.0 | 要精査（ミドルウェア型・複数クレート） | lock-free zero-copy IPC — pub/sub・request-response・event の SOA 型。SPSC ring より高機能 |
| `shared_memory` + `raw_sync` | 0.12.4（2022-03・停滞）/ 0.1.5（2020-10・停滞） | 未確認 | MIT OR Apache-2.0 | `cfg-if`・`libc`・`nix`・`rand`・`win-sys`/`winapi` | 薄い OS shm ラッパ + shm 内 Mutex/RwLock/Event 同期（RwLock は Windows 非対応） |
| `ipc-channel` | 0.23.0（2026-09・servo プロジェクト） | 未確認 | MIT OR Apache-2.0 | `crossbeam-channel`・`serde_core`・`postcard`・`uuid`・`thiserror`・`windows` 等 | serde 直列化チャネル — unix socket/Mach port/named pipe 上の unbounded channel（shm ではない） |
| `interprocess` | 2.4.6（2026-10 更新・活発） | 1.75+ | 0BSD OR Apache-2.0 | 要精査 | local socket・named/unnamed pipe 主眼 — 2.x は shm を提供しない |
| `postcard` | 1.1.3（wire format は 1.0 で安定化） | 要確認 | MIT OR Apache-2.0 | `serde`・`heapless`・`cobs` | serde ベースのコンパクトバイナリ — `no_std` 対応・varint 系 |

- 観測ベンチマーク（外部ブログ・未検証）: ping-pong で shared_memory ≈ 0.12 µs・iceoryx2 ≈ 0.9 µs・unix socket ≈ 15–18 µs・stdin/stdout ≈ 16 µs — 非 RT コマンド経路ならパイプ/ソケット帯でも実用上十分、process 経路は shm のレイテンシ優位が材料となる

## Alternatives and Comparison

### 軸 1: 共有メモリ SPSC の実現機構（「shm SPSC 経路」自体は design 確定済み — 実装選択は design）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) `iceoryx2` | lock-free zero-copy・MSRV 1.89 で workspace と一致・活発・dual license・wait-free notify が要件に素直 | ミドルウェア型で依存木・API 表面が大きい — SPSC ring 用途には過剰の恐れ。依存追加規約の妥当性証明が要 | candidate |
| (b) `shared_memory` + `raw_sync` | 薄い OS ラッパで構造が単純・観測レイテンシ最小級 | 双方とも更新停滞（2022/2020）・`raw_sync` の RwLock は Windows 非対応・保守状況証明が弱い | maybe |
| (c) `memmap2`/自前 shm + 手書き SPSC + イベント通知 | 依存最小・制御最大（RT 由来の ring 実装知見を移植可） | 自前の `unsafe` が大きく暫定規約と衝突・futex/WaitForXxx 等の OS 通知実装と検証責務が重い | no（規約不整合寄り） |
| (d) OS API 直（`libc`/`windows-sys`） | 依存ゼロ | `unsafe` 全面・全 OS 分岐を自前 — (c) よりさらに重い | no |

### 軸 2: 非 RT コマンド経路の媒体（`HostCommand`/`HostReply` の直列化送受）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) stdio/匿名パイプ + 直列化（`serde`+`postcard` 等） | 実装最小・プラットフォーム中立・spawn 時に stdio をそのまま使える。コマンドは非 RT で遅延許容（~µs 単位でも不要） | `PluginDescriptor` の `&'static` 写像・wire バージョニング設計が要 | candidate（仮説） |
| (b) shm リングをコマンドにも流用 | 機構単一化 | 可変長 `String`/`Vec<u8>` の shm レイアウト設計が必須になり (a) より設計負荷が高い | maybe |
| (c) local socket（`interprocess`/`ipc-channel`） | 移植性のある成熟プリミティブ | 依存追加。stdio と比べ機能過剰の恐れ | maybe |

### 軸 3: クラッシュ検出と失敗表現（検出方式は design 確定 — 実装と `Failed` 表現の境界が論点）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) 監視スレッド + `Child::wait`（exit status 取得） | 最も単純確実・異常終了/正常終了を exit status で区別可・std のみ | 監視スレッドの所有（インスタンス毎 or 共有）設計が要 | candidate（design 確定の方式） |
| (b) `try_wait` ポーリング | 監視スレッド不要 | 検出遅延・周期設計が要・応答なしハングとの区別は別途 | maybe |
| (c) shm ハートビート | 機構内完結 | プロセス死亡とハングを区別できず OS 監視の代替にならない | no |

`Failed` 表現: (i) plugin-core `LifecycleState::Failed` 追加 — 意味的に正しい・`#[non_exhaustive]` で non-breaking だが状態機械・失敗イベントの遷移表設計が要、vs (ii) host 側インスタンスフラグ — plugin-core 非侵襲だが `lifecycle_state()` が失敗を報告できない。（確定は design）

### 軸 4: 子プロセスランナーの形態とワイヤ所有

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) `ruviform-plugin-export` が runner エントリ API を提供し、利用側/規格アダプタが子プロセスバイナリを構成 | ベースライン責務（「規格準拠エントリ生成」）と整合・外部プラグインもアダプタ経由で load 可能・薄い runner の設計文そのまま | 子側で plugin を instantiate する導線（`PluginFactory` 同等の登録面）設計が要 | candidate（design 確定の配置） |
| (b) host 自身の同一バイナリを `--plugin-host-worker` で再 spawn | 配布が単純・実行ファイル解決が容易 | 単一 exe にアダプタを含める必要・ホストアプリのバイナリ構成に侵入 | maybe |
| (c) 汎用 runner がプラグインを動的 load（dlopen/LoadLibrary） | 単一バイナリで規格横断 | 動的ロードの `unsafe`/ABI 管理が規約と衝突 | no |

ワイヤプロトコル所有: (i) plugin-core に wire 型 — host/export 双方が依存可能だが core に host 概念を持ち込む、(ii) export → host 依存 — 同順位の許容解釈が要、(iii) 双方で wire 型を個別定義（仕様は文書で一元化）— 重複だが層非侵襲。（確定は design）

## Tentative Conclusions and Hypotheses

- **confirmed**: L1 の構造決定（プロキシ配置・shm SPSC 分岐・非 RT 準同期・pass-through・Failed + 通知・自動再起動なし・遅延モデル）は accepted ADR + approved design で確定済み — **本 briefing で新規 ADR は不要**
- **confirmed**: `ruviform-plugin-export` 新設はベースライン A.2 の予定クレートであり、Issue #76 がその初回実装 — クレート追加自体は設計済み構造の実施行動
- **confirmed**: RT 側結果読取りのプロセス内経路は `ruviform-rt` SPSC（`channel`/`Producer`/`Consumer`）の再利用候補 — Primitives 依存は plugin-host の許可範囲内
- **confirmed**: `PluginDescriptor` の `&'static` フィールドと `HostCommand`/`HostReply` への直列化機構未付与が、コマンド経路実装で最初に破れる前提 — design の事前課題
- **confirmed**: plugin-core に `Failed` 状態がない — `Failed` 表現の所有（plugin-core 拡張 vs host 側状態）は design 論点
- **hypothesis**: shm 機構は `iceoryx2`（MSRV 一致・活発）が最有力だが依存木サイズの精査が採否前提。`shared_memory`+`raw_sync` は停滞リスク — 確定は design（採用時は先例 `select-audio-device-backend` 型の依存追加記録/ADR 判断が別途要）
- **hypothesis**: コマンド経路は stdio/パイプ + serde 系直列化が最小整合、`Failed` は plugin-core 拡張が意味的に正しい、runner は (a) の提供形がベースラインと整合 — いずれも確定は design
- **deferred（Issue スコープ外として確認済み）**: 自動再起動・L2+ サンドボックス（seccomp/job object）・エンジン側の L1 配線・規格アダプタ（clap/vst3）実プラグインの子プロセス E2E・L1 GUI の本格対応

## Open Questions

> すべて design フェーズが答える論点 — architecture 未決事項ではなく実装設計の確定対象。

- shm IPC 機構の採否と依存追加の妥当性証明（`iceoryx2` vs `shared_memory`+`raw_sync` vs その他）+ 通知機構（イベント/futex 等）
- ワイヤプロトコル所有（plugin-core / export→host / 双方定義）と `HostCommand`/`HostReply` の直列化方式（serde 系採否・`PluginDescriptor` `&'static` の写像・バージョニング）
- `Failed` 状態の表現（plugin-core `LifecycleState` 拡張 vs host 側状態）と `lifecycle_state()`/コマンドの defined rejection 面
- L1 ドライバスレッドの所有（proxy 内 per-instance vs `ruviform-exec` 等への配置）と RT 側結果受け渡しへの `ruviform-rt` SPSC 適用
- 先行ブロック数（遅延深さ）の既定値・設定面と、max_frames 変動・イベント/transport の shm ブロックレイアウト
- クラッシュ通知の呼出し側インターフェース（状態ポーリング vs コールバック/通知チャネル）
- runner のプロセス形態（export 提供エントリを誰が exe 化するか）・子側で plugin を instantiate する導線・spawn 引数/接続ブートストラップ
- L1 GUI の defined rejection or 明示スコープ外
- L0/L1 選択面の API 形（共通 trait・enum・並存具象型）

## Promotion Candidates

- [x] spec-doc: [plugin-l1-out-of-process.md](../specs/plugin-l1-out-of-process.md)（**created — proposed**。issue framing・scope 境界・L1 契約の受入基準を spec として formalize）
- [x] adr-doc: **不要と判断** — L1 の構造的决定（階層化・shm SPSC・非 RT 準同期・pass-through・Failed+通知・境界維持）は accepted ADR `plugin-isolation-strategy` と approved design `plugin-isolation` で網羅済み。残る open 選択（shm 機構・wire 形式・Failed 表現・runner 形態）は実装設計レベル — briefing 時点で決定していない選択を ADR に記録しない（#99 と同型）。依存クレートの採用確定時に先例 `select-audio-device-backend` 型の採否 ADR が要る場合は design/plan フェーズの責務
- [ ] design-doc: 次ノード（design）で起票 — Open Questions の確定が責務

## Sources and Relations

- Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)（本探索対象）・[#31](https://github.com/scarecrow173/ruviform/issues/31)（分離設計 Issue）・[#74](https://github.com/scarecrow173/ruviform/issues/74)（L0 実装 Issue・CLOSED）
- 一次文書: [plugin-isolation.md](../designs/plugin-isolation.md)（approved・L1 契約）・[plugin-isolation-strategy.md](../adr/plugin-isolation-strategy.md)（accepted）・[plugin-isolation-design.md](../specs/plugin-isolation-design.md)（#31 spec）・[plugin-host-l0.md](../specs/plugin-host-l0.md)/[plugin-host-l0.md](../designs/plugin-host-l0.md)（#74 spec/design）・[ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md)（A.2 クレート表 — `ruviform-plugin-export` 行・層内順序・依存方向ルール）
- ローカル一次証拠: `crates/ruviform-plugin-host/src/command.rs`（`HostCommand`/`HostReply`/`HostError`/`PluginHandle` — 直列化未付与・`process` 非コマンド）・`host.rs`（`InProcessHost` — `Send` 不要求・具象型）・`plugin.rs`（`Plugin`/`PluginFactory`/`ErasedPlugin`/`erase`）・`crates/ruviform-plugin-core/src/lifecycle.rs`（5 状態遷移表 — `Failed` なし）・`descriptor.rs`（`PluginDescriptor` — `&'static` フィールド）・`processor.rs`/`event.rs`/`transport.rs`（`ProcessConfig`/`Event`/`EventList`/`TransportInfo` — `Copy` wire 形状）・`crates/ruviform-rt/src/spsc.rs`（プロセス内 SPSC — IPC 不可・ドライバ→RT 経路候補）・ルート `Cargo.toml`（members — export 不在）・`crates/README.md`（計 20 クレート索引）
- IPC 候補クレート観測: crates.io / lib.rs `iceoryx2` 0.10.0・`shared_memory` 0.12.4・`raw_sync` 0.1.5・`ipc-channel` 0.23.0・`interprocess` 2.4.6・`postcard` 1.1.3、および Rust IPC ベンチマーク記事 2 件（上記 `relations.source` — 観測値・未検証）
