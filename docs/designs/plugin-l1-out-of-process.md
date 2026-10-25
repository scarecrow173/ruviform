---
id: "DESIGN-034LNAalREIHyUQNVXLLin"
type: "design"
status: "approved"
title: "プラグイン L1 アウトプロセス分離（plugin-host L1 プロキシ + plugin-export 薄ランナー）"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/plugin-l1-out-of-process.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/designs/rt-verification-lane-design.md"
    - "docs/adr/select-audio-device-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/discovery/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/designs/plugin-host-l0.md"
    - "docs/specs/plugin-host-l0.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# プラグイン L1 アウトプロセス分離（plugin-host L1 プロキシ + plugin-export 薄ランナー）

## Context

Issue #76 — #74（L0・CLOSED）で確定したハンドル + 直列化コマンド境界の上に、#31 の分離設計が規定した **opt-in L1 アウトプロセス経路**を実装する。本書は discovery `docs/discovery/plugin-l1-out-of-process.md` の Open Questions 全項目と spec `docs/specs/plugin-l1-out-of-process.md` の 7 受入基準を確定する。

上流確定事項（再決定しない）: L1 は opt-in・公開面は `PluginHandle` + `HostCommand`/`HostReply`/`HostError` と L0 同一・`process` はコマンド語彙外・L1 経路は共有メモリリング + 非 RT ドライブ・RT スレッドは `try` 読取りのみ・クラッシュは `Failed` + 通知・自動再起動なし・遅延は先行ブロック数 × ブロック長・応答なし時は pass-through。

## 決定サマリ（discovery Open Questions → 本書の結論）

| # | 論点 | 決定 |
| --- | --- | --- |
| a | `ruviform-plugin-export` 形状 | 薄いランナーエントリ `run(factories)` を提供するライブラリ + 参照 runner bin。依存は `ruviform-plugin-core` のみ（ベースライン A.2 適合）。ワイヤ型は **plugin-core の `wire` モジュール**に置く |
| b | `Failed` 表現 | **plugin-core `LifecycleState` 拡張**（`Failed` state + `LifecycleEvent::Failed`）。host 側フラグは `lifecycle_state()` が報告できず spec の観測契約を満たさないため棄却 |
| c | ワイヤプロトコル | 手書きリトルエンディアン codec（serde 不採用 — core を無依存に保つ）。`&'static` 文字列は **wire 側は所有 `String`、公開面の `PluginDescriptor` は registration 供給**で回避（leak なし・型変更なし） |
| d | shm IPC 機構 | **`shared_memory` 採用**（named segment・安全な slice API）。`raw_sync`・`iceoryx2`・自前 shm/OS 直は棄却（§依存追加の判断） |
| e | コマンド媒体 vs データ面 | 制御 = **stdio パイプ**（stdin/stdout・フレーム化メッセージ）。音声データ = **共有メモリスロット**。ブロック doorbell もパイプ上の制御メッセージとして多重化 |
| f | クラッシュ検出 | driver スレッドが `Child::try_wait` を周期監視 + stdout EOF を即時検出。`Failed` 遷移 + `HostNotification`（std mpsc キュー）で呼出し側へ通知 |
| g | pass-through 意味 | 健全時アンダーラン = **直前に受信した出力ブロック**（受信歴なしは無音）。`Failed` 後 = **無音**。`process` 等価は `Ok(CONTINUE)` + pass-through 出力を返しハングしない。既定遅延 = 先行ブロック **2** × ブロック長 |
| h | RT 契約 | RT 側は `ruviform-rt` SPSC の `try_push`/`try_pop` と事前確保ステージングのみ — `rt_` 命名テストで assert_no_alloc 実測 |
| i | L0/L1 選択面 | **`PluginHost` コンポジット**（統一ハンドル空間 + 登録時ルーティング）。`register_out_of_process(spec)` が plugin_id → L1 ルートを宣言し、`Instantiate` 呼出し側は層を意識しない |
| j | モック/fixture | export `src/bin/ruviform-plugin-l1-runner`（参照 runner・モック gain 内蔵）を `CARGO_BIN_EXE_*` で検証。host 側は独自の最小 worker bin `l1-test-worker` で proxy を E2E 検証（`--crash-after-msgs` で確定的クラッシュ模擬） |

## Architecture

### クレート構成と依存方向（ベースライン A.2 適合）

| クレート | 追加要素 | 追加依存 |
| --- | --- | --- |
| `ruviform-plugin-core` | `LifecycleState::Failed`/`LifecycleEvent::Failed`・`wire` モジュール・移設されるプラグイン側契約群 | なし |
| `ruviform-plugin-host` | `PluginHost` コンポジット・L1 プロキシ内部（spawn/監視/ドライバ/shm/RT 脚）・`OutOfProcessSpec`/`HostNotification`・`src/bin/l1-test-worker` | `ruviform-rt`（Primitives — 許可範囲内）+ `shared_memory` |
| `ruviform-plugin-export`（新設） | ランナーエントリ `run`・argv 規約・`src/bin/ruviform-plugin-l1-runner` | `ruviform-plugin-core` のみ |

**プラグイン側契約の所有移転（host → core、re-export で互換）**: `Plugin`/`PluginFactory`/`ErasedPlugin`/`erase`/`HostInput`/`HostOutput`/`HostError`/`GuiHandle`/`GuiParentToken`/`PluginHandle` を `ruviform-plugin-core` へ移設し、plugin-host は全て `pub use` で再エクスポートする（`ruviform_plugin_host::Plugin` 等の既存パス・列挙バリアント参照は無変更でコンパイル継続 — L0 公開面・挙動は変えない）。理由: export（制作側）が plugin を駆動する契約を host から参照できない（A.2 で同順位の host/export 相互依存は不許可）。`Plugin` は `PluginProcessor` の直接拡張であり本来 core の契約族 — 正しい責務境界への修正。`PluginHandle::new` は `#[doc(hidden)] pub` 化（host が mint するが、型は共有語彙）。

### 子プロセスモデル

**1 子プロセス = 1 プラグインインスタンス（per-`Instantiate` spawn）**。`PluginHandle` ↔ child が 1:1 で、child の死亡が即そのインスタンスの `Failed` となる spec の粒度契約に一致する。runner が複数 factory を内蔵する場合は argv `--ruviform-plugin-id` で選択する。

### スレッド構成（per L1 インスタンス = 子プロセス 1 + スレッド 2）

| スレッド | 所有 | 責務 |
| --- | --- | --- |
| `driver` | `Child`（`Mutex` 不要 — 単独所有）・stdin・shm in-slot・command mpsc・submit `Consumer` | コマンド直列化送信・ブロック投入（shm 書込 + doorbell）・`try_wait` 周期監視（約 200µs ループ、poll 間隔は実装定数 `DRIVER_POLL_INTERVAL`）・シャットダウン監視犬（猶予超過で `child.kill()`） |
| `reader` | stdout・shm out-slot・result `Producer` | 子→親メッセージの多重解除（`CommandReply`→待機者起床・`BlockDone`→staging 複写 + result push）・EOF 検出（`seen_eof` flag） |

クラッシュ検出は 2 経路が冪等に収束する: (i) reader が stdout EOF を検出 → `seen_eof` セット、(ii) driver の `try_wait` が `Some(ExitStatus)` を返す。先に発火した方がインスタンスを `Failed` 遷移させ、`HostNotification` をキューへ送り、pending コマンド待機者へ `HostError::Ipc` で起床する。`try_wait` は reaping を兼ねる（`Some` 返却時に回収済み）。ライブ中の child exit はコード/シグナル不問で `Failed`（自発 exit 0 を含む — プロトコル違反）。`Shutdown` 送信後の exit のみ clean teardown。

### L1 `process` 等価経路（RT 脚 = プロセス内 SPSC × 2）

```
RT caller ──staging in[seq%D]──try_push(seq)──> submit ch ──> driver ──shm in-slot──> stdin doorbell BlockIn{seq}
RT caller <──staging out[seq%D]──try_pop─────── result ch <── driver <──shm out-slot──< stdout BlockDone{seq}
```

- `submit`/`result` の両脚は `ruviform_rt::channel`（rtrb・Send 要素 `u64` seq）。RT スレッド側は `try_push`/`try_pop` のみ — pipe write・shm 書込・ブロッキング read は全て driver/reader 側。
- **staging**: `Activate` 確定時に `ProcessConfig::max_frames() × チャネル総数` で `in`/`out` 各 D スロット（D = `lookahead_blocks + 1`）を事前確保（非 RT コマンド内での alloc は合法）。RT 側は staging への bounded memcpy のみ。
- RT `process(handle, ...)` フロー: (1) ローカル lifecycle ミラーが `Processing` か const 検査 → (2) 入力を `staging_in[seq%D]` へ複写、`submit.try_push(seq)`（満杯なら投入放棄 → pass-through）→ (3) `expected = seq - lookahead`（アンダーフロー = 定義済み無音）→ (4) result ch を `expected` まで drain（`< expected` は stale 破棄・有界回数）→ (5) 一致 seq があれば `staging_out` を `outputs` へ複写し `last_output` を更新、なければ pass-through。
- **pass-through 規則（確定）**: 健全インスタンスのアンダーラン → 直前に受信した出力ブロック（受信歴なし → 無音）。`Failed` インスタンス → 無音。いずれも `Ok(ProcessResult::CONTINUE)` を返す — RT 面の「定義済み拒否」は pass-through 出力そのもの（分離設計「pass-through で継続」）。コマンド面の拒否は `Err(HostError::Failed)`（下表）。
- **遅延契約**: `lookahead_blocks`（既定 **2**、per-spec 設定可能・`OutOfProcessSpec.lookahead_blocks`）。遅延 = `lookahead_blocks × block_frames / sample_rate`（例: 2 × 512f @ 48 kHz ≈ 21.3 ms）。API doc・crate doc に明記。
- child 遅れ・無応答へのタイムアウトは **underrun 機構そのもの**（result が間に合わなければ pass-through で継続）— ブロック面に別個の wall-clock タイムアウトは置かない。

### ワイヤプロトコル（所有: `ruviform-plugin-core::wire`）

ワイヤ型・codec は plugin-core の `wire` モジュールに単一定義する — discovery 案 (iii)「双方個別定義」は同一プロトコルの実装二重化で相互運用不良の温床となり、規約（同一責務の authoritative 実装は一箇所）に反するため棄却。core は codec を**手書きリトルエンディアン**とし serde 系を持ち込まない（serde を core に入れると全 adapter の依存木に流れる）。

フレーム: `[u32 LE payload_len][u8 kind][payload]`。バージョンは `Hello` のみが運ぶ（`WIRE_VERSION = 1` — 不一致は handshake 失敗として kill + `HostError::Ipc`）。

| メッセージ | 方向 | 内容 |
| --- | --- | --- |
| `Hello { version: u16, capabilities: u32 }` | 子→親 | 起動直後の最初のフレーム |
| `Command { seq: u64, cmd: wire::Command }` | 親→子 | コマンド相関 seq 付き |
| `Reply { seq: u64, result: Result<wire::Reply, HostError> }` | 子→親 | `HostError` は core 共有語彙をそのまま encode |
| `BlockIn { seq: u64 }` | 親→子 | shm in-slot `seq % D` への doorbell |
| `BlockDone { seq: u64, result: ProcessResult }` | 子→親 | shm out-slot `seq % D` への doorbell |
| `Shutdown` | 親→子 | `HostCommand::Destroy` の wire 意味。runner は状態機械が許す範囲で deactivate→destroy 正規化後に exit(0) |

```rust
// plugin-core::wire（概念形 — シグネチャの正本）
pub enum Command {        // HostCommand の handle 非修飾ミラー（子は singleton）
    Scan,
    Initialize,
    Activate { config: ProcessConfig },
    StartProcessing, StopProcessing, Deactivate,
    SaveState,
    RestoreState { bytes: Vec<u8> },
    OpenGui { parent: GuiParentToken },
    CloseGui,
}
pub enum Reply {
    Ok,
    Scanned(Vec<Descriptor>),          // 所有型 — 下記
    State { bytes: Vec<u8> },
    Opened { gui: GuiHandle },
}
pub struct Descriptor {                 // PluginDescriptor の所有 wire 形
    pub id: String, pub name: String, pub vendor: String,
    pub version: String, pub features: Vec<String>,
}                                       // From<PluginDescriptor> / フィールド等価比較を提供
```

`wire::Command`/`wire::Reply`/`wire::Descriptor` は core の公開型であるため、公開 enum 規約に従い `#[non_exhaustive]` を付与する（`wire::Descriptor` は struct なので対象外）。wire プロトコル自体は `WIRE_VERSION` の handshake 照合で厳格一致を強制するため、フィールド追加はバージョン更新とセットで行う。

- **語彙写像**: `HostCommand::Instantiate` ↔ spawn + handshake、`Destroy` ↔ `Shutdown` + reap。残り 10 動詞は `wire::Command` に 1:1 で写す（handle は子が singleton のため落とす — 将来の bundle 子は `Instantiate`/handle 修飾を wire に追加するプロトコル改訂）。
- **`PluginDescriptor` の `&'static` 問題**: 公開 `HostReply::Scanned(Vec<PluginDescriptor>)` は L1 ルートでも **registration 供給の 'static 値**を返す（下記 `OutOfProcessSpec.descriptor`）。child は `Hello` 後の `Scan` 応答で内蔵 descriptor を `wire::Descriptor` として報告し、proxy が spec の descriptor とフィールド等価で照合する（不一致 → kill + `HostError::Ipc`）。**所有文字列→`&'static` への変換は設計上発生せず、`Box::leak` は使わない**。
- handshake: spawn → argv `--ruviform-plugin-id <id>` `--ruviform-shm-key <key>` → 子 `Hello` → 親 `Scan` → 子 `Scanned`（照合）→ `Instantiated{handle}` 応答。いずれか失敗・タイムアウト（`command_timeout`）で `Err(HostError::Ipc)` + child kill/reap。
- データ型写像: `ProcessConfig`/`ProcessResult`/`GuiHandle`/`GuiParentToken`/`HostError` は core 共有型を codec が直接 encode/decode。`TransportInfo`/`Event`/`EventList`/`MediaTime` 系は **repr 固定の wire レイアウト**（accessor 経由で encode、ctor で decode — `repr(Rust)` の memcpy 横断はしない）。

### 共有メモリデータ面

- `shared_memory` の named segment を **インスタンス毎に 1 本**。proxy が `os_id` に一意 key（pid + slot + counter 由来）を生成して作成し、argv `--ruviform-shm-key` で子へ引き渡して attach。所有権は proxy、Destroy/drop で unlink。
- レイアウト: `[header][in-slots][out-slots]`。header = `{magic, wire_version, in_depth, out_depth, slot_bytes, in_off, out_off}`。slot = `{seq u64, frames u32, channels u32, event_count u32, flags, transport wire, result wire, events wire, samples f32[]}`。固定上限は wire 定数 `WIRE_MAX_FRAMES = 8192`・`WIRE_MAX_CHANNELS = 16`・`WIRE_MAX_EVENTS = 1024`（slot ≈ 550 KB、D=3 で ≈ 3.3 MB/インスタンス）。`Activate` の `ProcessConfig` が上限超なら `Prepare(UnsupportedConfig)` で defined rejection — codec 境界での検証は untrusted データの検証として wire 層に置く。
- **shm 内に共有制御状態を持たない**: head/tail インデックス・ロック・atomic 配置を置かず、スロット選択は doorbell の `seq` が担い、in/out は厳密な交互制御（producer は `done seq-D` 受領後にのみ slot `seq%D` を再利用）。doorbell バイト列がプロセス間の release/acquire エッジを構成する — writer は doorbell 送信前に `std::sync::atomic::fence(Ordering::Release)`、reader は受信後に `atomic::fence(Ordering::Acquire)` を打ち（実装は hardware fence — `compiler_fence` は同一 process 内のコンパイラ並べ替え規制に留まり cross-process 可視性を保証しないため Issue #76 Task 4 で強化・Task 7 closeout で本書へ同期）、shm アクセスは安全な `as_slice`/`as_mut_slice` のみ（**自前 `unsafe` ゼロ**が shm 機構選定の条件）。

### plugin-export（薄いランナー）

```rust
// 子プロセスのエントリ。プラグイン作者/adapter のバイナリが main から呼ぶ。
// argv 規約: --ruviform-plugin-id <id>（複数 factory 時必須・単一なら省略可）
//           --ruviform-shm-key <key>（shm attach 名）
// 返却: プロセス exit code として直返し
pub fn run(factories: Vec<Box<dyn PluginFactory>>) -> std::process::ExitCode;
```

責務: argv 解釈 → `Hello` 送信 → shm attach + header 検証 → シングルスレッドのメッセージループ（stdin フレーム読取り → `wire::Command` を `ErasedPlugin` へ dispatch → `Reply`。`BlockIn` → `Lifecycle` が `Processing` を確認後 `process_host` を shm スロット上の `BufferView`/`BufferViewMut` adapter 経由で呼出し → `BlockDone`）→ `Shutdown`/stdin EOF で lifecycle 正規化して終了。runner は panic を catch して `HostError::PluginFailed` に変換（プラグインパニックで子を殺さない — ホスト検出手段と区別するため）。shm 上の in/out スロットは固定レイアウトの planar `f32` — core の `BufferView`/`BufferViewMut` を実装する軽量 view で wrap する（runner 内 unsafe なし）。

## 公開面（plugin-host 追加）

```rust
pub struct PluginHost { /* l0: InProcessHost + l1 instances + 統一スロット表 */ }
impl PluginHost {
    pub fn new() -> Self;
    pub fn register_factory(&mut self, f: Box<dyn PluginFactory>);            // L0 ルート
    pub fn register_out_of_process(&mut self, spec: OutOfProcessSpec)
        -> Result<(), HostError>;                                            // L1 ルート
    pub fn execute(&mut self, cmd: HostCommand) -> Result<HostReply, HostError>;
    pub fn process(&mut self, h: PluginHandle, i: &[HostInput<'_>],
                   o: &mut [HostOutput<'_>], e: EventList<'_>,
                   t: Option<TransportInfo>) -> Result<ProcessResult, HostError>;
    pub fn lifecycle_state(&self, h: PluginHandle) -> Option<LifecycleState>;
    pub fn plugin_mut(&mut self, h: PluginHandle) -> Option<&mut dyn ErasedPlugin>; // L0 スロットのみ
    pub fn try_recv_notification(&mut self) -> Option<HostNotification>;
    #[doc(hidden)] pub fn kill_child(&mut self, h: PluginHandle) -> Result<(), HostError>; // 検証支援
}

pub struct OutOfProcessSpec {
    pub descriptor: PluginDescriptor,   // 'static — Scan/Instantiate の公開面を担う
    pub program: std::ffi::OsString,    // runner 実行ファイル
    pub args: Vec<std::ffi::OsString>,
    pub lookahead_blocks: u32,          // 既定 2
    pub command_timeout: std::time::Duration, // 既定 10s（Reply 待ちの defined timeout）
}

#[non_exhaustive]
pub enum HostNotification {
    InstanceFailed { handle: PluginHandle, exit: ChildExit },
}
pub struct ChildExit { pub code: Option<i32>, pub signal: Option<i32> } // signal は unix のみ
```

- **選択面の形**: 共通 trait・enum dispatch・並存具象の 3 案のうち、**コンポジット + 登録時ルーティング**を採る。呼出し側のコマンド系列は L0 と完全に同一で、層の選択は登録（管理操作）時のみに局所化される。`Instantiate{plugin_id}` の形を変えないため呼出し側変更ゼロ — spec の「呼出し側は層を意識しない」に最も忠実。`PluginHost` は plugin_id → ルート表を持ち、スロット表が `PluginHandle` を `(L0 inner handle | L1 instance)` に写す（ハンドル空間は統一・generation 規則は L0 と同一）。同一 plugin_id の L0/L1 重複登録は `Err(HostError::DuplicatePluginId)` で defined rejection。
- `InProcessHost` は公開のまま無変更 — 純 L0 利用者は従来どおり使う（`PluginHost` 内部の L0 ルートにもそのまま使う）。

## エラー・拒否面（定義済み拒否の一覧）

| 条件 | 結果 |
| --- | --- |
| 違法 lifecycle 遷移（コマンド） | `Err(HostError::Lifecycle(_))` — 子側 `Lifecycle` が判定し wire で返す |
| `Failed` インスタンスへのコマンド（`Destroy` 除く） | `Err(HostError::Failed)` — 新 variant（`#[non_exhaustive]` で non-breaking） |
| `Failed` インスタンスへの `Destroy` | `Ok` — kill 必要なら実行、reap・スロット解放 |
| `Failed` インスタンスへの `process` 等価 | `Ok(CONTINUE)` + 無音（pass-through 規則） |
| spawn/handshake/バージョン/transport 失敗・コマンド応答タイムアウト | `Err(HostError::Ipc)` — 新 variant。タイムアウト時は併せて `Failed` 遷移 |
| L1 インスタンスへの `OpenGui`/`CloseGui` | `Err(HostError::Unsupported)` — proxy が子へ送らず拒否（L1 GUI は deferred） |
| `Activate` の config が wire 上限超過 | `Err(HostError::Prepare(PrepareError::UnsupportedConfig))` |
| submit/result チャネル満杯・応答遅延 | pass-through（上記規則）— エラーでなくフォールバック |
| plugin_id 重複登録 | `Err(HostError::DuplicatePluginId)` |

`Failed` 遷移は `LifecycleEvent::Failed` をローカルミラーへ適用（`Created|Initialized|Activated|Processing → Failed`、`(Failed, Destroy) → Destroyed`、その他の `Failed` からの遷移は illegal）。`Failed` 以後に新たに送らない — 子は既に死んでいるか殺される前提。

### plugin-core `Lifecycle` 拡張（b の確定）

- `LifecycleState::Failed` — 終端準状態。`Destroy` のみ `Destroyed` へ進める（解放経路）。`accepts_processing()` は `false`、`is_failed()` ヘルパを追加。
- `LifecycleEvent::Failed` — *報告* 遷移（`InitFailed`/`ActivateFailed` 系の naming）。子プロセス死亡・応答喪失・プロトコル喪失など実行コンテキストの終端失敗を host が注入する。retry 可能な `InitFailed`/`ActivateFailed` とは意味が異なり terminal である旨を doc で区別。
- `#[non_exhaustive]` 済みのため外部 `match` への非破壊追加。遷移表は const のまま。

## 依存追加の判断（暫定規約の理由記録）

| 依存 | 採否 | 理由 |
| --- | --- | --- |
| `ruviform-rt`（path） | 採用（plugin-host） | Primitives 層で A.2 許可範囲。driver→RT の結果受け渡し SPSC — wait-free・RT 検証済みの社内正本を流用 |
| `shared_memory = "0.12"` | **採用（plugin-host）** | named shm の薄い OS ラッパ（MIT OR Apache-2.0）。`as_slice`/`as_mut_slice` の安全 API で shm への読み書きに自前 `unsafe` が要らない（`rtrb`/`cpal` 先例: 安全 API を持つ外部クレートを内部依存に閉じ込める）。依存木は `cfg-if`/`libc`/`rand`/win 系の小規模群。Windows+POSIX named object をカバー。停滞（2022-03 最終リリース）は採用条件として記録: 利用を内部 `shm` モジュールに隔離し公開 API・feature 名へ露出しない（置換コスト ~200 LOC に封じ込め）、3 OS でのコンパイル/E2E 動作を plan の検証条件とする |
| `serde`/`postcard` | 不採用 | wire codec は手書き LE で ~15 動詞に十分。serde を core に入れると全 adapter の依存木に波及するため |
| `iceoryx2` | 不採用 | 0.10 で ~19 直接依存（内部 15 クレート + flatbuffers + serde + toml）・伝搬含め ~40 級の SOA ミドルウェア。1:1 の固定リング用途に対し service discovery・設定ファイル機構が過剰（YAGNI）。宣言 MSRV 1.89 は一致するが pin 済み toolchain との余白がない |
| `raw_sync` | 不採用 | 停滞（2020-10）・`RwLock` は Windows 非対応。同期はパイプ doorbell で代替するため不要 |
| `memmap2`/自前 shm・OS API 直 | 不採用 | named object 生成・atomic 配置・通知の `unsafe` を自前で負うことになり暫定規約と衝突 |
| `ipc-channel`/`interprocess` | 不採用 | shm を提供しない・stdio パイプが既に無料であり依存追加の見合いがない |

## 検証戦略（mock/fixture — j の確定）

| 層 | fixture | 検証 |
| --- | --- | --- |
| `ruviform-plugin-export` tests | `src/bin/ruviform-plugin-l1-runner`（`CARGO_BIN_EXE_*`）— 参照 runner。内蔵モック `GainPlugin`（core `Plugin` 実装・state 往復・param event 反映） | 実プロセス spawn → `Hello` → `Scan`/lifecycle コマンド往復 → shm ブロック処理（gain 正当性）→ `Shutdown` で exit(0) を test 側の最小 host 実装で確認 |
| `ruviform-plugin-host` tests | `src/bin/l1-test-worker`（`CARGO_BIN_EXE_*`）— wire 適合の最小 worker。`--crash-after-msgs=N`/`--hang-after-msgs=N` で確定的異常系 | proxy E2E: scan→instantiate→lifecycle→state→process 経路・`kill`/クラッシュ→`Failed`+通知+以後拒否・pass-through・違法遷移の `Lifecycle` エラー意味一致 |
| 単体 | codec 往復・wire↔public 写像・スロット index 計算・`Failed` 遷移表・重複登録拒否・descriptor 不一致 kill | crate 内 test |
| RT レーン | `rt_` 命名テスト（host tests）: `rt_l1_process_path_no_alloc`（assert_no_alloc 窓で `process` try 経路 0 alloc）・`rt_l1_result_leg_try_ops` | `mise run test-rt` で選択 |

spawn 引数の受け渡し・`CARGO_BIN_EXE_*` の利用は各 crate 内に閉じる（cross-package の bin 解決に依存しない）。

## Deferred Design Concerns

- L1 GUI（プロセス境界越しの親ウィンドウ埋め込み）| 規格・プラットフォーム固有の大課題・本 Issue 範囲外 | L1 インスタンスへの GUI 要求 | `Unsupported` 拒否のままでは GUI 持ちプラグインが L1 で使えない
- 自動再起動・supervisor ポリシー | 状態復元はプラグイン責務の #31 確定判断 | supervisor 型運用要求 | 手動復旧の運用負荷
- 規格アダプタ（clap/vst3/ofx）の子プロセス組込み・実プラグイン L1 E2E | runner 形態の前提が本書で確定・別 Issue 級の配線 | 実プラグインでの L1 需要 | 検証がモック範囲に留まる
- bundle 子（1 子 N インスタンス）| per-instance spawn の方が `Failed` 粒度に整合・単純 | メモリ/spawn コスト要求 | インスタンス数スケールに弱い
- エンジン/利用側への L1 配線 | plugin-host の engine 消費自体が未実装 | engine プラグイン統合 Issue | L1 が consumer から到達不能
- wire バージョニング/進化 | 初版 `WIRE_VERSION=1` で十分 | プロトコル改変要求 | 互換性対応コスト
- stderr ログ配管 | inherit で運用可能・診断価値は副次的 | 子プロセスのログ解析要求 | クラッシュ時の手がかりが stderr 依存
- video/NONRT プラグインの L1 経路 | 本 Issue は audio `process` 経路のみ | NONRT video port の分離要求 | video plugin は L1 不可
- ドライバの共有リーパー/io 多重化 | per-instance 2 スレッドが最も単純明確 | インスタンス数増大 | スレッド数 = 2×instance のコスト

## Verification Notes

- [ ] spec AC1: L1 バックエンドが L0 と同一コマンド系列を子プロセス側で実行し同一リプライ/エラー意味で返す（E2E + `Lifecycle` エラー一致）
- [ ] spec AC2: `process` 等価でブロック送信 → 定義遅延（先行 2 ブロック）後に try 読取りで処理済み出力 — gain でデータ正当性
- [ ] spec AC3: RT 側読取り経路が no-block・no-alloc で `rt_` テストが `mise run test-rt` グリーン
- [ ] spec AC4: 強制終了検出 → `Failed` 遷移 + 通知 + 以後定義済み拒否・自動再起動なし
- [ ] spec AC5: クラッシュ/応答なしで pass-through（直前受信 or 無音）を返しハングしない
- [ ] spec AC6: `ruviform-plugin-export` が workspace メンバー・薄いランナーエントリを持ち結合をモックで検証
- [ ] spec AC7: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` + doc 監査 findings 0
- [ ] 設計レビュー: 層ルール（host/export 相互依存なし・ワイヤ所有 = core）・unsafe ゼロ方針・`#[non_exhaustive]` 方針・遅延/pass-through 契約の doc 明記
