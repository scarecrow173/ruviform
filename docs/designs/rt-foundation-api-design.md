---
id: "DESIGN-034JgsK9khgDsPqG04Sp6i"
type: "design"
status: "approved"
title: "RT 基盤 API 設計（ruviform-rt プリミティブ・ruviform-exec スレッド優先度機構）"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/21"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/implement-rt-foundation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "ruviform-architecture-design-baseline.md"
    - "overview.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/implement-rt-foundation.md"
    - "../adr/rt-foundation-primitives-implementation.md"
  derived-by:
    - "../plans/implement-rt-foundation.md"
    - "../test-specs/rt-channel-and-snapshot-contracts.md"
    - "../test-specs/rt-context-and-processor-contract.md"
    - "../test-specs/thread-priority-lifecycle.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# RT 基盤 API 設計（ruviform-rt プリミティブ・ruviform-exec スレッド優先度機構）

## Context

GitHub Issue #21 の実装設計。承認済み spec `SPEC-034Jcr3dOT5Uxp4znj42II`（[implement-rt-foundation.md](../specs/implement-rt-foundation.md)）と accepted ADR `ADR-034Jcra57UlCiVE1Poq2DO`（[rt-foundation-primitives-implementation.md](../adr/rt-foundation-primitives-implementation.md)）の決定を、`ruviform-rt` と `ruviform-exec` の具体的な公開 API 形状（型・シグネチャ・`**RT contract:**` ラベル）に落とす。本書は後続の plan / tasks が分解する実装入力契約であり、方式の採否そのものは ADR が正本、要件は spec が正本である。

前提となる確定事項（再評価対象外）:

- `ruviform-rt` は最下位クレート。他 ruviform クレート（media-core 等）に依存しない。採用依存は `rtrb`・`arc-swap` のみで、いずれも private 実装詳細 — 公開 API のシグネチャ・公開エラー・feature 名に外部型を露出しない（ベースライン A.2 `ruviform-rt` 行）。
- `ruviform-exec` は非 RT 実行基盤。RT プリミティブは持たないが、OS スレッドの詳細を隠蔽する責務としてスレッド優先度機構を所有する。採用依存は `audio_thread_priority`（`default-features = false`、private）。
- RT 契約ラベルは API 単位（RT ADR）: `**RT contract:** RT-suitable`（alloc/dealloc なし・ノンブロッキング・有界）または `**RT contract:** non-RT only`（RT 契約区間内からの呼び出しは契約違反）。
- 暫定規約: edition 2024・MSRV 1.85・stable のみ、`unsafe` 原則禁止（本設計の API 面・内部実装ともに `unsafe` を要求しない — 採用クレートはすべて safe API）、全公開 enum に `#[non_exhaustive]`、公開 API の例は doctest。
- 現行スケルトン: `ruviform-rt` は `RtContext`（`_private` フィールドのみ）+ `version()`、`ruviform-exec` は `version()` のみ。本設計は既存構造を拡張する。

## Design Overview

### モジュールマップ（`ruviform-rt`）

private `mod` + フラット `pub use` の構成（media-core と同型）。`mod` 宣言自体は公開せず、公開 API 面はクレート直下の名前に集約する。

| モジュール | 公開型・関数（`pub use` 後のパス） | RT contract 概要 | 内部実装 |
| --- | --- | --- | --- |
| `context.rs` | `RtContext`（`#[doc(hidden)] mint`） | `mint`: RT-suitable | opaque struct（`_private`） |
| `processor.rs` | `Processor`・`RtProcessor` | `prepare`: non-RT only / `process`: RT-suitable | trait 定義のみ |
| `spsc.rs` | `channel`・`Producer`・`Consumer`・`TryPushError`・`TryPopError` | `channel`: non-RT only / `try_push`・`try_pop`・`pop_latest`・容量アクセサ: RT-suitable | `rtrb` ラップ |
| `retire.rs` | `retire_channel`・`Retirer`・`DropCollector` | `retire`: RT-suitable / `retire_channel`・`drain`: non-RT only | `spsc` プリミティブ上に構成 |
| `snapshot.rs` | `slot`・`SnapshotWriter`・`SnapshotReader`・`SnapshotGuard` | `read`・guard `Drop`: RT-suitable / `slot`・`publish`・`collect`: non-RT only | `arc-swap` + `spsc` retire channel |
| `clock.rs` | `MasterClock` | 全メソッド RT-suitable | `AtomicU64` + 構築時固定 timebase |
| `error.rs` | `Error`・`Result` | RT-suitable（`Copy`・alloc なし） | media-core error.rs の写像 |

```rust
// crates/ruviform-rt/src/lib.rs の公開面（mod 宣言は private）
mod clock;
mod context;
mod error;
mod processor;
mod retire;
mod snapshot;
mod spsc;

pub use clock::MasterClock;
pub use context::RtContext;
pub use error::{Error, Result};
pub use processor::{Processor, RtProcessor};
pub use retire::{retire_channel, DropCollector, Retirer};
pub use snapshot::{slot, SnapshotGuard, SnapshotReader, SnapshotWriter};
pub use spsc::{channel, Consumer, Producer, TryPopError, TryPushError};
```

`retire` 側のチャネル構築関数はモジュール内でも `pub fn retire_channel` と命名する（`spsc::channel` と同名関数をフラット再公開すると衝突するため。private mod 規約上の必須調整であり、公開パスは `ruviform_rt::retire_channel`）。

### `ruviform-exec` モジュール

| モジュール | 公開型・関数 | RT contract 概要 | 内部実装 |
| --- | --- | --- | --- |
| `thread_priority.rs` | `promote_current_thread_to_realtime`・`demote_current_thread_from_realtime`・`ThreadPriorityParams`・`ThreadPriorityGuard`・`PriorityError` | `promote`・`demote`・guard `Drop`: non-RT only / パラメータ構築: RT-suitable | `audio_thread_priority`（`default-features = false`）ラップ |

### 依存の追加（private 実装詳細）

```toml
# crates/ruviform-rt/Cargo.toml
[dependencies]
rtrb = "0.4"
arc-swap = "1"

# crates/ruviform-exec/Cargo.toml
[dependencies]
audio_thread_priority = { version = "0.38", default-features = false }
```

外部型非露出: `rtrb`・`arc-swap`・`audio_thread_priority` の型・エラー・feature 名は公開シグネチャに一切出さない。`default-features = false` は `with_dbus`（→ `dep:dbus`、ネイティブ libdbus 依存）を排除するためのもので、依存 feature 名の再露出に相当する独自 feature も追加しない。いずれの依存も `path` 依存ではなく crates.io 通常依存。`Cargo.lock` は追跡対象のまま更新する。

## API Sketch

各項目の `**RT contract:**` ラベルは公開 API ドキュメントに verbatim で記載する契約文言である。

### `context.rs` — `RtContext`

```rust
/// RT 契約区間内であることを示す capability トークン。
///
/// 発行規則: RT 契約区間を開始した駆動者（RT 駆動者 = デバイスコールバック
/// 入口の `ruviform-audio`、オフライン駆動者 = `ruviform-exec` の非 RT
/// ドライバ）のみが mint できる。アプリケーション等の任意コードは構築
/// できないよう公開コンストラクタ・公開フィールドを持たない
/// （`_private` がリテラル構築を防止）。
///
/// 単一の opaque 型であり、オフライン/RT を区別する実行時モードフラグや
/// variant は持たない（「同一 API 内部で if rt 分岐しない」原則）。
/// オフライン発行コンテキストでの RT 制約の緩和は実行時状態ではなく、
/// 発行した駆動者に紐づく契約・ドキュメントレベルの記述である
/// （発行コンテキストの区別であってランタイム状態ではない）。
#[derive(Debug)]
pub struct RtContext {
    _private: (),
}

impl RtContext {
    /// **RT contract:** RT-suitable — 契約区間開始時の純粋なトークン構築。
    /// alloc・ブロッキングなし。
    ///
    /// 駆動者入口点専用。利用制限は「doc-hidden + 駆動者限定利用の契約
    /// 規約 + コードレビュー」による規約レベルの強制であり、型レベルで
    /// 構築を完全に抑止するものではない（ADR がベースライン L284 文言の
    /// 弱体化として明示記録）。
    #[doc(hidden)]
    #[must_use]
    pub fn mint() -> Self;
}
```

- `mint` は `#[doc(hidden)] pub` のため cross-crate の駆動者から到達可能だが、公開 API 面・docs には表れない。呼び出し箇所は駆動者入口点（デバイスコールバック・オフライン駆動）に限定する規約。capability factory 型は不採用（現行駆動者 2 箇所では間接層が過剰 — ADR）。
- `RtContext` は `process` に `&RtContext` で渡す参照トークンであり、clone/copy・内部データ・振る舞いを持たない。

### `processor.rs` — `Processor` / `RtProcessor`

```rust
/// 非 RT の構築・再構成フェーズを表す trait。RT パスから呼ばない。
pub trait Processor {
    /// `prepare` に渡すセットアップ入力（構成・レイアウト等）。呼び出し側が定義。
    type Setup;
    /// `prepare` が生成する RT 処理オブジェクト。
    type Rt: RtProcessor;
    /// `prepare` の失敗分類。クレート共通 `Error` ではなく各プロセッサ固有の
    /// エラー型を `core::error::Error` で束縛する。
    type Error: core::error::Error;

    /// **RT contract:** non-RT only — alloc・I/O・ブロッキング可の
    /// 構築/再構成経路。呼び出すたびに新しい RT 処理オブジェクトを返す
    /// （再 `prepare` は型上は許容。退役した旧 Rt オブジェクトの扱いは
    /// 呼び出し側の責務 — retire channel で退役可能）。
    fn prepare(&mut self, setup: Self::Setup) -> Result<Self::Rt, Self::Error>;
}

/// RT 契約区間内で駆動される処理オブジェクトの trait。
pub trait RtProcessor {
    /// ブロック単位の I/O 型（バッファ・イベント列等）。ジェネリックに保ち
    /// 上位クレートが具体化する（rt は最下位のため media-core 型に依存しない）。
    type Io;

    /// **RT contract:** RT-suitable — alloc/dealloc・ブロッキング・無制限
    /// 待機・非有界処理を行わない契約。infallible: 処理上の失敗は
    /// `io` ドメインの状態として表現する（`Result` を返さない）。
    ///
    /// `ctx` は RT 契約区間の証明 — `RtContext` を持たないコードは
    /// `process` を呼べないため、契約区間外実行を型で抑止する。
    fn process(&mut self, io: &mut Self::Io, ctx: &RtContext);
}
```

- 二 trait 分離はベースライン「`prepare(setup)` が `process` を持つ**別の処理オブジェクト**を生成」の型レベル具現化。単一 trait 同居では「prepare 済みか」を型で表せないため不採用（ADR）。
- associated type を採用し `dyn` 前提の形状にしない。object safety の制約は消費側（グラフ実行層等）が必要に応じて adapter で解決する — 最下位契約クレートで dispatch 方針を固定しない（`dyn` は明示的に deferred）。
- `Rt` 型に `Send` 境界は課さない。非 RT スレッドでの `prepare` → RT スレッドへの引き渡しが必要な駆動側は、自身の API で `Rt: Send` を追加要求する（trait レベルで固定しない）。

### `spsc.rs` — SPSC チャネル

RT ↔ 非 RT の有界・ノンブロッキング通知プリミティブ（RT → 非 RT: メータ・イベント・退役オブジェクト。非 RT → RT: パラメータ・制御コマンド）。内部は `rtrb` の SPSC リングをラップする。

```rust
/// 容量 `capacity` の SPSC チャネルを構築する。容量は構築時に固定で
/// 以後の伸長はない（事前確保型の受け渡し構造）。
///
/// **RT contract:** non-RT only — リング確保を伴う構築 API。
/// `T: Send` はスレッド越境ペイロードの API 契約（端点はそれぞれ
/// 別スレッドへ `Send` される前提）。
pub fn channel<T: Send>(capacity: usize) -> (Producer<T>, Consumer<T>);

/// 送信端点。`T: Send` 時に `Send`（`!Sync` — 単一スレッド所有）。
pub struct Producer<T> { /* rtrb::Producer<T> を抱える */ }

impl<T> Producer<T> {
    /// **RT contract:** RT-suitable — wait-free・alloc なし・有界。
    /// 満杯時は `Err(TryPushError::Full(value))` で値を呼び出し元へ返す。
    /// drop/coalesce/keep のポリシー適用は呼び出し側に委ねる
    /// （汎用のリングレベル方針 enum は持たない — 満杯時方針は API 単位）。
    pub fn try_push(&mut self, value: T) -> Result<(), TryPushError<T>>;

    /// **RT contract:** RT-suitable — 現在満杯か。
    pub fn is_full(&self) -> bool;

    /// **RT contract:** RT-suitable — 構築時の固定容量を返す。
    pub fn capacity(&self) -> usize;
}

/// 受信端点。`T: Send` 時に `Send`（`!Sync`）。
pub struct Consumer<T> { /* rtrb::Consumer<T> を抱える */ }

impl<T> Consumer<T> {
    /// **RT contract:** RT-suitable — wait-free・alloc なし・有界。
    /// 空なら `Err(TryPopError::Empty)`。
    pub fn try_pop(&mut self) -> Result<T, TryPopError>;

    /// **RT contract:** RT-suitable — 保留要素を最後まで pop して
    /// 最新 1 件だけを返す（latest-wins のパラメータ統合向け）。
    /// ループ回数は固定容量で有界。空なら `None`。
    pub fn pop_latest(&mut self) -> Option<T>;

    /// **RT contract:** RT-suitable
    pub fn is_empty(&self) -> bool;

    /// **RT contract:** RT-suitable
    pub fn capacity(&self) -> usize;
}

/// `try_push` の失敗。`Full` は値を保持して返す（呼び出し側ポリシー適用用）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TryPushError<T> {
    /// チャネルが満杯。値は返却される。
    Full(T),
}

impl<T> TryPushError<T> {
    /// 返却された値を取り出す。
    ///
    /// **RT contract:** RT-suitable — ムーブのみ。
    pub fn into_inner(self) -> T;
}

/// `try_pop` の失敗。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TryPopError {
    /// チャネルが空。
    Empty,
}
```

- 両エラー型は `Display` + `core::error::Error` を実装（`TryPushError<T>` の `Error` 実装は `T: Debug` 境界を要求する形でよい — 表示は値を含めない）。`#[non_exhaustive]` は enum レベル。
- `is_full`/`is_empty`/`capacity` の戻り値は観測時点の近似値として扱う（SPSC リングの同時更新を考慮した診断用であり、正確性を契約としない）。

### `retire.rs` — 遅延ドロップ（retire channel）

RT パスで退役した重いオブジェクトを非 RT 側へ送出して drop する。`spsc` プリミティブ上に構成する（basedrop 型パターンを自前で実現。`basedrop` 依存は不採用 — ADR）。

```rust
/// 容量 `capacity` の退役チャネルを構築する。容量は構築時に固定（伸長なし）。
///
/// **RT contract:** non-RT only — リング確保を伴う構築 API。
/// `T: Send + 'static`: ペイロードはスレッドを越境し、非 RT 側の
/// drain 地点に借用を残さない。
pub fn retire_channel<T: Send + 'static>(
    capacity: usize,
) -> (Retirer<T>, DropCollector<T>);

/// 退役端点（RT 側）。
pub struct Retirer<T> { /* spsc::Producer<T> を抱える */ }

impl<T: Send + 'static> Retirer<T> {
    /// オブジェクトを退役させる。内部は `try_push` 意味（ノンブロッキング・
    /// alloc なし・有界）。満杯時のオーバーフロー方針:
    /// **RT 側では drop/dealloc/blocking を絶対に行わず、
    /// `core::mem::forget` による bounded-leak とする**。
    /// drop-on-full は RT 側 dealloc になるため採用しない（ADR 決定）。
    /// 容量は呼び出し側契約（drain 周期ごとの最大退役数を見込んで確保する）。
    ///
    /// **RT contract:** RT-suitable
    pub fn retire(&mut self, value: T);
}

/// 回収端点（非 RT 側のみ所有）。
pub struct DropCollector<T> { /* spsc::Consumer<T> を抱える */ }

impl<T: Send + 'static> DropCollector<T> {
    /// 受信済みの退役オブジェクトをすべて drop する。`T` のデストラクタは
    /// この呼び出しスレッド上で実行される（非 RT 側のみで dealloc が
    /// 発生する契約の実行点）。戻り値は drop した件数。
    ///
    /// **RT contract:** non-RT only
    pub fn drain(&mut self) -> usize;

    /// **RT contract:** non-RT only — 未回収件数の有無を返す診断用。
    pub fn is_empty(&self) -> bool;
}
```

- 実装メモ: `DropCollector` は内部に `spsc::Consumer<T>` に加えてローカル pending キュー（`Vec<T>`）を持ち、`drain` はチャネル受信分とローカル投入分の双方を drop する。`snapshot` モジュールが書き込み側で追い出した `Arc` を同一 drain 経路に乗せるための `pub(crate) fn push_local(&mut self, value: T)` を持つ（公開 API には出さない）。
- `Retirer`/`DropCollector` 端点は `T: Send` 時に `Send`・`!Sync`。

### `snapshot.rs` — 不変スナップショット交換

非 RT 側で構築した構成・実行プランの `Arc<T>` スナップショットを atomic に差替え、RT 側がブロック境界でノンブロッキングに参照するプリミティブ。内部の atomic セルは `arc_swap::ArcSwap<T>`、退役経路は `spsc` 上の retire channel。

```rust
/// スナップショット交換スロットを構築する。`initial` が初期スナップショット、
/// `retire_capacity` は reader 側退役チャネルの固定容量（guard 解放経路の
/// 収容数 — drain 周期あたりの最大同時 guard 退役数を見込む）。
///
/// **RT contract:** non-RT only — セル・チャネル確保を伴う構築 API。
/// `T: Send + Sync`: スナップショットは writer（非 RT）と reader（RT）の
/// 間で共有されるため、共有 `Arc<T>` の内容には `Send + Sync` を要求する。
pub fn slot<T: Send + Sync + 'static>(
    initial: Arc<T>,
    retire_capacity: usize,
) -> (SnapshotWriter<T>, SnapshotReader<T>);

/// 書き込み端点（非 RT 側）。`T: Send + Sync` 時に `Send`・`!Sync`。
pub struct SnapshotWriter<T> { /* Arc<ArcSwap<T>> + DropCollector<Arc<T>> */ }

impl<T: Send + Sync + 'static> SnapshotWriter<T> {
    /// スナップショットを atomic に差替える。追い出された旧 `Arc` は
    /// 退役経路（自身が抱える `DropCollector` の drain 経路）へ渡し、
    /// dealloc 地点を非 RT 側に統一する。
    ///
    /// **RT contract:** non-RT only — swap 自体はノンブロッキングだが、
    /// 追い出した `Arc` の dealloc がこのスレッド上で起きうるため
    /// RT 側では呼べない。
    pub fn publish(&mut self, snapshot: Arc<T>);

    /// RT reader の guard 解放で退役した `Arc`（および publish で追い出した
    /// `Arc`）をすべて回収して drop する。非 RT 駆動スレッドが定期的に
    /// （例: 制御 tick ごと・`publish` 前に）呼ぶ。戻り値は drop 件数。
    ///
    /// **RT contract:** non-RT only
    pub fn collect(&mut self) -> usize;
}

/// 読み取り端点（RT 側ハンドル）。`T: Send + Sync` 時に `Send`・`!Sync`
/// （内部の `RefCell` が `!Sync` — 単一 RT スレッドへの移譲専用）。
pub struct SnapshotReader<T> {
    /* Arc<ArcSwap<T>> + RefCell<Retirer<Arc<T>>> */
}

impl<T: Send + Sync + 'static> SnapshotReader<T> {
    /// 現在のスナップショットを guard 経由で参照する。lock-free load +
    /// `Arc` clone で alloc なし（arc-swap の load は稀な同時 publish 時に
    /// 理論上再試行しうるが、writer 単一・低頻度の前提で有界）。
    ///
    /// 受けは `&self`（`&mut` ではない）: `RefCell` の内部可変により
    /// 複数 guard の同時存在を許容する。guard が `&mut` 借用を握ると
    /// guard 生存中に再 `read` できなくなり「同時 guard 可」の要件と衝突する。
    ///
    /// **RT contract:** RT-suitable
    pub fn read(&self) -> SnapshotGuard<'_, T>;
}

/// スナップショット参照 guard。`Deref<Target = T>` で内容を読む。
/// `&'a RefCell<_>` を保持するため `!Send`・`!Sync` — guard は
/// 取得した RT スレッドから逃げない。
///
/// **RT contract:** RT-suitable — `Drop` は内部 `Arc` を inline drop
/// せず reader の `Retirer` へ `retire()` で送出する（`try_push` +
/// 満杯時 `core::mem::forget` bounded-leak は `Retirer` が内包 — retire
/// モジュールと同一方針・同一実装）。
pub struct SnapshotGuard<'a, T> { /* Option<Arc<T>> + &'a RefCell<Retirer<Arc<T>>> */ }

impl<T> core::ops::Deref for SnapshotGuard<'_, T> {
    type Target = T;
}

impl<T> Drop for SnapshotGuard<'_, T> { /* retire channel へ送出 */ }
```

重要不変条件（**本プリミティブの中核契約**）:

- `SnapshotGuard` の `Drop` が内部 `Arc<T>` を reader 内 `RefCell<Retirer<Arc<T>>>` へ送出する（`Retirer::retire` = `try_push` + 満杯時 `mem::forget` をそのまま再利用 — オーバーフロー方針を単一実装に集約）。`RefCell` の `borrow_mut` は `Drop` 内で瞬間的（モメンタリ）に取得するだけなので、guard が複数同時に生存しても解放は逐次で競合しない（reader は `Send`・`!Sync` の単一 RT スレッドハンドル）。
- チャネル満杯時のフォールバックは `core::mem::forget`（bounded-leak）。**guard の `Drop`・`read`・`publish` 経路のいずれでも RT スレッド上で dealloc/drop/ブロッキングが起きない**。
- したがってスナップショットの dealloc が起きうる地点は「非 RT 側 drain（`SnapshotWriter::collect`）」または「`SnapshotWriter::publish` 内部（非 RT）」のみ。
- **arc-swap 単体ではこの保証を実現できない理由**（reader 側 last-drop の穴）: `ArcSwap::load_full` で得た `Arc` を生で保持すると、writer が swap して非 RT 側が自らのコピーを drop した後、RT reader が最後の参照を手放す瞬間に `Arc` の dealloc が RT スレッド上で実行されうる。guard 型経由でのみ参照を許し、guard `Drop` が常に退役チャネルへ迂回させる組合せでこの経路を塞ぐ。writer が swap で追い出した `Arc` も同一 drain 経路へ渡す（`DropCollector::push_local` 経由 — publish は非 RT のため直接 drop でも契約上は等価だが、dealloc 地点を drain に統一するため退役経路を使う）。

### `clock.rs` — `MasterClock`

A/V 同期の基準時計。単一 `AtomicU64` の tick + 構築時固定の有理数 timebase で表現する（timebase は以後不変のため tearing/seqlock は不要）。

```rust
/// 共有時計。`Arc<MasterClock>` で共有する（`Send + Sync`）。
/// 単一の論理 writer（現役の駆動者 — RT・オフラインいずれでも可）が
/// `publish_tick` で tick を更新し、読み取り側は wait-free。
/// writer の単一性は契約レベルの規約（型では強制しない — 複数 writer が
/// 同時に書いても atomic なので破綻しないが、時計の単調性は単一 writer
/// 規約に依存する）。
pub struct MasterClock {
    /* tick: AtomicU64 + timebase: (u32, u32) — 不変 */
}

impl MasterClock {
    /// `timebase` は `(numerator, denominator)` で 1 tick = num/den 秒。
    /// 0 を含むなら `Err(Error::InvalidValue("timebase"))`。
    ///
    /// **RT contract:** RT-suitable — 純粋な値構築（alloc なし）。
    /// 共有用 `Arc` の確保は呼び出し側の責務（典型的には非 RT セットアップ）。
    pub fn new(timebase_num: u32, timebase_den: u32) -> Result<Self>;

    /// **RT contract:** RT-suitable — wait-free の atomic store。
    /// 単一の論理 writer（現役駆動者）のみが呼ぶ契約。
    pub fn publish_tick(&self, tick: u64);

    /// **RT contract:** RT-suitable — wait-free の atomic load。
    pub fn tick(&self) -> u64;

    /// **RT contract:** RT-suitable — 構築時固定値の load。
    pub fn timebase(&self) -> (u32, u32);
}
```

- メモリオーダリング: `publish_tick` は Release store、`tick` は Acquire load を既定とする（tick の公開と他データの可視性を組み合わせる呼び出し側に備える。単独の値読みに限れば Relaxed でも動作するが、Acquire/Release を契約上の既定とする）。
- media-core の `MediaTime`/`TimeBase` には依存しない（rt は最下位）。raw tick + 有理数 timebase `(u32, u32)` のみを持ち、tick→秒・他 timebase への変換は上位層の責務。

### `error.rs` — `Error` / `Result`

media-core `error.rs` の規約を転記する共通エラー型。

```rust
/// `ruviform-rt` 公開 API の共通エラー。`Copy` + `&'static str` ペイロード
/// で alloc なし — RT パス上でも生成・保持・返却できる。
/// enum レベル `#[non_exhaustive]`（公開 enum 統一方針）。
///
/// **RT contract:** RT-suitable — `Copy`、alloc なし。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Error {
    /// パラメータが不正。ペイロードは静的なフィールド名相当の説明
    /// （例: `"timebase"`、`"capacity"`）。
    InvalidValue(&'static str),
}

impl core::fmt::Display for Error { /* ... */ }
impl core::error::Error for Error {}

/// `ruviform-rt` 操作の Result エイリアス。
///
/// **RT contract:** RT-suitable — `Error` が `Copy` ・alloc なしのため
/// `Result` の生成も alloc しない。
pub type Result<T> = core::result::Result<T, Error>;
```

API 別の失敗分類（`TryPushError<T>`・`TryPopError`・`PriorityError`）は値の返却や API 固有の分類が要るため独自型とし、共通 `Error` は汎用の失敗（無効パラメータ等）に使う。

### `lib.rs` — crate-level doc の更新

`ruviform-rt/src/lib.rs` の crate doc を media-core の形式に倣って実装済み状態へ同期する:

- **責務**: `Processor`/`RtProcessor` 契約、`RtContext` capability、`MasterClock`、SPSC・retire channel・スナップショット交換・事前確保型受け渡し構造の RT-safe プリミティブ。スケジューリング・I/O を持たない（現行記述の維持・更新）。
- **Allowed dependency layers**: なし（`rtrb`・`arc-swap` は private 実装詳細で型非露出）。
- **RT contract**: 全公開 API は `RT-suitable` または `non-RT only` の明示ラベル。
- **Contract source**: ベースライン（クレート表 A.2・「RT／非 RT の分離方式」・「RT 契約」節）+ [RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md) + [実装方式 ADR](../adr/rt-foundation-primitives-implementation.md)。
- `version()` は維持（`**RT contract:**` ラベルを付与 — pure・alloc なしで RT-suitable）。
- 「Phase 1 skeleton」の記述とプレースホルダー注記（`RtContext` doc の "Placeholder for Issue #17"）を除去・更新する。

`ruviform-exec/src/lib.rs` の crate doc も責務記述へ `thread_priority` モジュールを追加する（「OS スレッドの詳細は隠蔽」責務の一部としてスレッド優先度機構を記載）。「RT プリミティブは持たない」の記述は維持する — 優先度 promotion は RT プリミティブではなく非 RT の OS スレッドサービス。`Non-RT only` の RT 制約記述・Contract source（ベースライン A.2 + 実装方式 ADR）を更新。

### `ruviform-exec::thread_priority` — スレッド優先度機構

OS 提供の RT スレッド優先度付与（Windows MMCSS「Pro Audio」・macOS time-constraint policy・Linux `SCHED_FIFO`）を `non-RT only` API として提供する。内部は `audio_thread_priority`（`default-features = false`）のラップで、外部型・エラー・feature 名は露出しない。

```rust
/// 優先度付与の要求パラメータ。オーディオバッファのブロック長と
/// サンプルレートから内部クレートがスケジューリング周期を算出する。
pub struct ThreadPriorityParams { /* private フィールド */ }

impl ThreadPriorityParams {
    /// `buffer_frames` = オーディオバッファフレーム数、
    /// `sample_rate` = サンプルレート（Hz）。
    ///
    /// **RT contract:** RT-suitable — 純粋な値構築（alloc なし）。
    /// ただし用途は非 RT セットアップ区間専用。
    pub fn audio(buffer_frames: u32, sample_rate: u32) -> Self;
}

/// **呼び出し元スレッド**を RT 優先度へ昇格させる（self-promotion）。
/// 成功時は demotion 用の `ThreadPriorityGuard` を返す。
///
/// **RT contract:** non-RT only — ブロッキング的な OS syscall を伴う。
/// 呼び出しモデルは対象スレッド自身からの自己昇格:
/// - exec 管理スレッド: spawn 時セットアップで呼ぶ
/// - デバイスコールバック由来スレッド（将来の audio-device）: 初回
///   コールバックの preamble で呼ぶ — **RT 契約区間の開始前**
///   （`RtContext` mint・初回 `process` 呼び出し前）
/// active な RT 契約区間内からの呼び出しは契約違反。
///
/// 失敗は致命的でなく best-effort の `Err(PriorityError)` として返す
/// （パニックしない）。呼び出し側は promotion なしで継続できる。
/// `default-features = false` 構成では rtkit/`CAP_SYS_NICE`/
/// `RLIMIT_RTPRIO` のない非特権の標準デスクトップ Linux で promotion は
/// 典型的に失敗し `Err` が返る — これは仕様上の失敗であり文書化済み。
/// 対象外プラットフォームでは no-op 成功を返しうる（内部クレートの
/// セマンティクスに従う）。
pub fn promote_current_thread_to_realtime(
    params: ThreadPriorityParams,
) -> Result<ThreadPriorityGuard, PriorityError>;

/// promotion を解除する。guard を消費するため二重 demote は型で防止。
///
/// **RT contract:** non-RT only — syscall を伴う。
pub fn demote_current_thread_from_realtime(
    guard: ThreadPriorityGuard,
) -> Result<(), PriorityError>;

/// promotion の保持トークン（opaque）。`Drop` で demotion を
/// best-effort 試行する（失敗は捨てる — 失敗検出が必要なら
/// `demote_current_thread_from_realtime` を呼ぶ）。
/// `Drop` が syscall を伴うため、RT 契約区間内で生成・破棄してはならない。
pub struct ThreadPriorityGuard { /* 内部ハンドル（非公開） */ }

/// 優先度機構の失敗分類。enum レベル `#[non_exhaustive]`。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PriorityError {
    /// OS が promotion 要求を拒否・充足できなかった
    /// （権限不足・非対応ポリシー等。`&'static str` は静的な文脈説明）。
    PromotionFailed(&'static str),
    /// demotion に失敗した。
    DemotionFailed(&'static str),
}

impl core::fmt::Display for PriorityError { /* ... */ }
impl core::error::Error for PriorityError {}
```

- モジュール名は `thread_priority`、exec では `mod thread_priority;`（private）+ `pub use` でフラット再公開する。`PriorityError` はモジュール単位の失敗分類とする（exec に crate 共通 `Error`/`Result` は今回作らない — 失敗 API が 1 系統のみのため YAGNI。将来複数系統が増えた時点で crate-level `error.rs` へ昇格を再評価）。
- 非特権 Linux の失敗挙動・self-promotion 呼び出しモデル・「RT 契約区間内からの呼び出しは契約違反」は API ドキュメントに verbatim で記載する。

## Invariants and Contracts

本設計が維持すべき不変条件・契約の一覧（plan/tasks 分解時の検証対象）。

1. **RT 契約ラベル**: 全公開 API に `**RT contract:** RT-suitable` または `**RT contract:** non-RT only` を記載する。`RT-suitable` の意味は alloc/dealloc なし・ノンブロッキング・有界。確保・伸長・syscall を伴う構築系（`channel`・`retire_channel`・`slot`・`promote_*`・`demote_*`・`DropCollector::drain`・`SnapshotWriter::{publish,collect}`）はすべて `non-RT only`。保証範囲は API 単位であり CI 成功は完全証明ではない（RT ADR）。OS レベルのハードリアルタイム保証は提供しない。
2. **`RtContext` 発行**: 単一 opaque 型・公開 ctor なし（`_private`）・実行時モードフラグ/variant なし。mint は `#[doc(hidden)] pub` で駆動者入口点のみが利用 — 強制は規約 + コードレビューであり型レベルの完全抑止ではない（ベースライン L284 の弱体化として ADR に記録済み）。オフライン発行コンテキストでは RT 制約が契約上緩和されるが、それは発行駆動者に紐づく文書契約であり実行時状態ではない。
3. **prepare/process 分離**: `prepare` は非 RT（確保可）で `Rt: RtProcessor` を生成し、`process` は `&RtContext` を要求して契約区間外実行を型で抑止する。`process` は infallible — 処理失敗は `io` ドメイン状態。
4. **チャネル契約**: 容量は構築時に固定（以後の伸長なし）。操作はノンブロッキング・alloc なし・有界。満杯時方針はリングレベル enum ではなく API 単位 — `try_push` は `Full(T)` で値を返して drop/coalesce/keep を呼び出し側ポリシーに委ね、latest-wins は `pop_latest` が担う。
5. **退役経路契約**: `Retirer::retire` と `SnapshotGuard::drop` は RT 側で drop/dealloc/blocking を行わない。満杯時は `core::mem::forget` の bounded-leak（容量は呼び出し側契約）。dealloc は非 RT 側 drain（`DropCollector::drain`・`SnapshotWriter::collect`）と非 RT の `SnapshotWriter::publish` 内でのみ発生する。
6. **スナップショット契約**: RT 側参照は `SnapshotGuard` 経由のみ。guard `Drop` は内部 `Arc` を reader の退役チャネルへ送出（reader 側 last-drop の穴を塞ぐ）。writer 差替で追い出された `Arc` も同一 drain 経路へ。共有 `Arc<T>` には `T: Send + Sync`。`SnapshotReader` は `Send`・`!Sync`（単一 RT スレッドハンドル）、`SnapshotGuard` は `!Send`・`!Sync`（RT スレッドから逃げない）。
7. **時計契約**: `MasterClock` の timebase は構築時に固定・以後不変（tearing/seqlock 不要）。tick 更新は単一の論理 writer（現役駆動者）の文書契約。読み取りは wait-free。
8. **スレッド境界の `Send`/`Sync`**: チャネルペイロード `T: Send`、退役オブジェクト `T: Send + 'static`、共有スナップショット `T: Send + Sync` を API 契約として宣言する。
9. **優先度機構契約**: `non-RT only`・self-promotion（対象スレッド自身が呼ぶ）・RT 契約区間開始前（`RtContext` mint・初回 `process` 前）。失敗は `Result` で表出しパニックしない。非特権 Linux での失敗は仕様として文書化。
10. **外部型非露出**: `rtrb`・`arc-swap`・`audio_thread_priority` の型・エラー・feature 名を公開シグネチャに出さない。`audio_thread_priority` は `default-features = false`（`with_dbus`/`dep:dbus` 無効）固定。
11. **`unsafe` 不使用**: 採用クレートはすべて safe API であり、rt・exec 自身のコードも safe のみで構成する（`#[expect(unsafe_code)]` の局所許可も本設計では要求しない）。
12. **公開 enum**: 全公開 enum（`Error`・`TryPushError<T>`・`TryPopError`・`PriorityError`）に enum レベル `#[non_exhaustive]`。将来の variant 増補を互換的に行える。
13. **命名規約（`rt_` 選択フィルタ）**: `mise run test-rt` は `cargo test --workspace rt_` の**部分文字列フィルタ**で RT 契約テストを選ぶ。RT 契約テストは `rt_` プレフィックス名、非 RT のテスト・doctest・テストヘルパー名は `rt_` 部分文字列を含まない命名とする（collision 例: `assert_*`・`start_*` は `rt_` を含むため不可 — `expect_*`・`spawn_*` 等を使う）。

## Test Strategy

- **doctest**: 全公開 API に使用例 doctest を記述する（AGENTS.md 規約）。単体テストはこれに追加する形。
- **`rt_` プレフィックス RT 契約テスト**（`mise run test-rt` で選択可能）:

| モジュール | テスト例 | 検証する契約 |
| --- | --- | --- |
| spsc | `rt_push_pop_fifo` | push→pop が FIFO で値を保全 |
| spsc | `rt_full_try_push_returns_value` | 満杯で `Full(T)` が値を返却（呼び出し側ポリシー適用可能） |
| spsc | `rt_pop_latest_returns_newest` | 複数保留時に最新のみ返す（latest-wins 統合） |
| spsc | `rt_try_push_pop_no_alloc` | counting allocator で try_push/try_pop が 0 確保 |
| retire | `rt_retire_then_drain_drops` | retire した値は drain でのみ drop（Drop-flag で未 drop を確認後 drain → flag セット） |
| retire | `rt_retire_full_forgets_not_drops` | 満杯時は `mem::forget`（Drop-flag が立たない = RT 側で drop しない） |
| snapshot | `rt_snapshot_guard_drop_routes_to_retire` | guard 解放で `Arc` の `strong_count` が >0 のまま（dealloc しない）、`collect` 後に解放される |
| snapshot | `rt_snapshot_publish_visibility` | `publish` 後の `read` が新値を観測 |
| snapshot | `rt_snapshot_displaced_arc_freed_on_collect` | publish で追い出された `Arc` が drain 経路で解放 |
| snapshot | `rt_snapshot_read_no_alloc` | counting allocator で `read`・guard drop が 0 確保 |
| clock | `rt_clock_tick_visibility` | `publish_tick` した値が `tick()` で観測できる（単一 writer） |
| clock | `rt_clock_timebase_fixed` | `timebase()` が構築値を返し続ける |
| context | `rt_mint_token_usable` | `mint` で得た `&RtContext` を `RtProcessor::process` に渡せる |
| processor | `rt_prepare_then_process` | テスト用 `Processor` で prepare→Rt 生成→`process(io, &ctx)` のライフサイクルが成立（`process` が `RtContext` を要求するシグネチャであることの利用側確認） |

- **allocation 検査**: alloc 不使用の挙動検証が必要な箇所に限り、std のみのテストスコープ counting `#[global_allocator]` ヘルパー（外部依存なし）を使う。テストバイナリ単位で有効化するフラグ（enable/reset/assert-zero/disable）を持ち、他テストの確保を誤計しないよう各テスト内で enable 区間を限定する。`assert_no_alloc` 型 CI レーン・恒常機構は RT 検証 Issue の責務で本設計のスコープ外。
- **Drop-flag サイドチャネル**: 退役・guard 解放の非 RT 側 drop を検証するため、`Drop` で `AtomicBool`/`AtomicUsize` を立てるテスト専用型を使う（dealloc 地点を「flag が立ったタイミング」として観測）。
- **`Send`/`Sync` 境界**: API 契約としての `T: Send`/`Send + Sync` 宣言を、テスト内のジェネリック `fn assert_send<T: Send>()`/`fn assert_send_sync<T: Send + Sync>()` 風の std のみヘルパーで端点型（`Producer`・`Consumer`・`Retirer`・`DropCollector`・`SnapshotWriter`/`Reader`・`MasterClock`）に適用して検証する。
- **exec**: `promote_current_thread_to_realtime` を spawn したスレッドから呼び、`Ok`/`Err` いずれでもパニックせず `Result` が返ることを確認する（非特権 Linux では `Err` が期待値 — プラットフォーム依存のため厳密な成功/失敗は assert しない）。RT 契約テストではないため `rt_` を含まない命名（例: `promotion_returns_result`）とし、モジュールパス名にも `rt_` 部分文字列が現れないことを確認する。
- **検証入口**: `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` がグリーン。`cargo metadata` で依存リスト（`rtrb`・`arc-swap`・`audio_thread_priority` `default-features = false`）と `with_dbus`/`dep:dbus` 無効を確認。`cargo doc` / シグネチャレビューで外部型の非露出を確認。

## Out of Scope

spec の Out of Scope を継承する（詳細は [implement-rt-foundation.md](../specs/implement-rt-foundation.md) が正本）:

- `assert_no_alloc` 型 CI レーン・thread-check 型スレッド契約検査 — RT 検証 Issue の責務
- `ruviform-rt` へのスレッド優先度機構の配置 — Primitives 層の「OS 固有 API・デバイス依存を持たない」制約により `ruviform-exec` に確定済み
- `audio_thread_priority` の dbus 経路を再有効化する feature — `default-features = false` 固定（intent 命名の opt-in feature は将来フォローアップ候補）
- 処理グラフ実行エンジン・pull スケジューリング・グラフ再コンパイル — engines 系 Issue
- `clack-extensions` thread-pool 等のプラグイン補助スレッド基盤 — プラグイン基盤 Issue
- オフライン決定論的実行の厳密保証範囲 — オフライン実行 Issue
- `ruviform-rt` から他 ruviform クレートへの依存 — rt は最下位のため禁止
- `ruviform-rt` へのスケジューリング・I/O の追加 — クレート非責務として確定済み
- 採用依存クレートの型・エラー・feature 名の公開 API への露出
- `ringbuf`・`basedrop` 等の非採用クレートの再評価（再評価条件は spec の Deferred Design Concerns）
- 複数 RT reader（マルチコンシューマ）対応 — 現行スナップショットは単一 reader（retire channel が SPSC）。複数 reader 需要が出た時点で per-reader チャネル等を ADR で再評価

## Deferred Design Concerns

| 据え置く項目 | 理由 | 再着手トリガ | 無視した場合のリスク |
| --- | --- | --- | --- |
| `assert_no_alloc` 型 CI 検査・thread-check 機構 | 本設計は `rt_` 命名のテスト配置と test-scoped counting allocator のみ | RT 検証 Issue | RT 契約の機械検証が遅れ、違反の混入が症状化まで遅れる |
| `ringbuf` への切替 | `rtrb` 採用済み | rtrb が要件（SPSC・依存木・保守）を満たせなくなった時点 | 代替手順が不明確になる |
| `basedrop` 系への切替 | SPSC 上の自前 retire channel を採用 | 退役経路の満杯時設計が複雑化した場合に ADR で再評価 | 切替基準が曖昧になる |
| promotion 失敗の per-OS semantics 詳細 | `Result` + エラー型の形状のみ確定。MMCSS/time-constraint/SCHED_FIFO の実効差・パラメータ写像詳細は実装・実機検証に委ねる | オーディオデバイス実装 Issue・実機検証 | OS 差を考慮しないパラメータ設計を早期に固定する |
| `audio_thread_priority` dbus 経路の再有効化 | `default-features = false` 固定。intent 命名の opt-in feature は将来候補 | 非特権 Linux で RT 優先度需要が顕在化した時点 | 非特権 Linux 利用者の手段が恒久的に失われる |
| `dyn` dispatch 前提の `Processor` 形 | associated type 採用。object safety は消費側 adapter で解決 | グラフ実行層で `dyn` 需要が確定した時点 | 最下位契約クレートで dispatch 方針を早期固定する |
| `RtContext` mint 権限の将来拡張 | doc-hidden mint で現行 2 駆動者に対応 | mint 権限を持つ駆動者が増えた時点で ADR 再評価 | 発行権の追跡可能性が不足する |
| 複数 RT reader のスナップショット | 単一 reader で現行需要を満たす | 複数 RT コンシューマ需要の発生 | 無用な汎用化・過剰設計 |
| exec crate 共通 `Error`/`Result` | 失敗 API が thread_priority 1 系統のみ（`PriorityError` はモジュール単位） | exec に複数の失敗系統 API が増えた時点 | 不要な抽象化の早期導入 |

## Verification Notes

- [ ] `RtContext` が単一 opaque 型で公開 ctor/フィールド・モードフラグを持たず、`#[doc(hidden)]` mint が `RT-suitable` ラベル付きで提供される
- [ ] `Processor`/`RtProcessor` 二 trait が存在し、`process` が `&RtContext` を要求するシグネチャで infallible
- [ ] SPSC が有界・ノンブロッキングで、満杯時 `try_push` が値を返す `TryPushError<T>` と drain-latest ヘルパー `pop_latest` を持つ
- [ ] retire channel で退役オブジェクトの `Drop` が非 RT drain でのみ実行され、満杯時も RT 側で drop/dealloc/blocking しない（`mem::forget`）ことがテストで検証される
- [ ] スナップショット参照が専用 guard 経由のみで、guard 解放・writer 差替の双方が退役経路へ送出し dealloc が非 RT 側でのみ発生することが `rt_` テストで確認できる
- [ ] `MasterClock` が単一 `AtomicU64` tick + 構築時固定 timebase・単一論理 writer・wait-free 読み取りを持つ
- [ ] チャネルペイロード `T: Send`・共有スナップショット `T: Send + Sync` の境界が API 契約で明示されている
- [ ] スレッド優先度 API が `ruviform-exec` に `non-RT only` ラベル・self-promotion モデル・`Result` 失敗表出・非特権 Linux 失敗挙動の文書化を満たす
- [ ] 全公開 enum に `#[non_exhaustive]`、全公開 API に `**RT contract:**` ラベルと doctest が存在する
- [ ] `rt_` 命名テストのみが `mise run test-rt` で選択され、非 RT テスト・doctest 名が `rt_` 部分文字列を含まない
- [ ] `cargo metadata` で `ruviform-rt` の依存が `rtrb`・`arc-swap`、`ruviform-exec` が `audio_thread_priority`（`default-features = false`）で `with_dbus`/`dep:dbus` 無効、外部型が公開 API に非露出
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` がグリーン

## References

- 一次ソース: GitHub Issue [#21 [impl] RT 基盤（rt プリミティブ・スレッド優先度機構・スナップショット交換）](https://github.com/scarecrow173/ruviform/issues/21)
- 実装要件（正本）: [implement-rt-foundation.md](../specs/implement-rt-foundation.md)（`SPEC-034Jcr3dOT5Uxp4znj42II`）
- 実装方式の決定（正本）: [rt-foundation-primitives-implementation.md](../adr/rt-foundation-primitives-implementation.md)（`ADR-034Jcra57UlCiVE1Poq2DO`）
- RT 契約規範: [realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md)（API 単位の保証ラベル・検証分担）
- 評価記録: [evaluate-rt-foundation-primitives.md](../discovery/evaluate-rt-foundation-primitives.md)（`DISC-034JcqEc2b3EHNgH9CRLaC`）
- 関連設計: [ruviform-architecture-design-baseline.md](ruviform-architecture-design-baseline.md)（クレート一覧 A.2 の `ruviform-rt`/`ruviform-exec` 行・「RT／非 RT の分離方式」・「RT 契約」節・要PoC 分類表 L486/L489）、[dev-foundation.md](dev-foundation.md)（検証入口・プロファイル）、[overview.md](overview.md)
- 規約: [`AGENTS.md`](../../AGENTS.md)（unsafe 原則禁止・依存追加規約・enum 方針・doctest 規約）、[public-enum-non-exhaustive-policy.md](../adr/public-enum-non-exhaustive-policy.md)
- 採用依存: `rtrb` 0.4（SPSC リング）、`arc-swap` 1.x（atomic `Arc` swap）、`audio_thread_priority` 0.38（`default-features = false`。MMCSS/time-constraint/`SCHED_FIFO` の一枚抽象化）
- 参考: 親 Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)
