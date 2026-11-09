---
id: "DESIGN-034UoQ3mxByzCnjgdAGpe1"
type: "design"
status: "approved"
title: "Issue #290 OBS-08 Profiling — allocation-probe 設計（CountingAllocator/AllocationScope）"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/290"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/290"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-290-observability-allocation-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/designs/rt-verification-lane-design.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/rt-assert-no-alloc-thread-check.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-290-observability-allocation-probe.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/293"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
  deferred-by: []
---

# Issue #290 OBS-08 Profiling — allocation-probe 設計（CountingAllocator/AllocationScope）

## Context

Issue #290（OBS-08）は CPU sampling・Instrumentation・RT Timing・Memory
Profiling を含むが、本設計はそのうち **開発・テスト用 CountingAllocator +
TLS AllocationScope** の slice のみを対象とする。`ruviform-observability`
は `allocation-probe` feature（既定 OFF・外部依存ゼロ）で gate された
`profiling` モジュールを新設し、Host/Test Binary が `#[global_allocator]`
として install できる `GlobalAlloc` アダプタ型と、スレッドローカルな
計測ウィンドウを提供する。

前提となる既存資産:

- `ruviform-rt/src/alloc_counter.rs`: `#[cfg(test)]` 限定の private
  TLS counter `GlobalAlloc`。イベント数のみ（bytes なし）・test binary
  専用・install 済みのため他バイナリから利用不可。
- `crates/ruviform-audio-device/examples/play_sine.rs`: example 内の
  `CountingAlloc`（static `AtomicUsize` 群 + `GlobalAlloc` 転送）。
  example バイナリ専用。
- `assert_no_alloc`（dev-dependency）: lib test binary の
  `#[global_allocator]`（`AllocDisabler`）上で割当検出を行う既存
  RT 検証レーン。`rt_` プレフィックス試験が使用。

`foundation-observability-minimal-api`（approved）の alloc_counter
所有者節は「共有可能なのは install しない `GlobalAlloc` 実装型のみ」
「移動先候補は `observability::debug`」としたが、本 slice では
Coordinator 指定により **`profiling` モジュール名**を採用する
（`debug` は OBS-10/#292 の領域）。機能的な責務（計測アダプタの提供）
は同じであり、モジュール名の差異は責務境界を変えない。

## Scope

### In Scope

- `src/profiling/` モジュール（`allocation-probe` feature gate）:
  - `CountingAllocator<A: GlobalAlloc>` と `AllocationStats`/`OpStats`
  - `AllocationScope`/`AllocationWindow`（TLS 計測ウィンドウ）
  - 契約試験（4 API・ネスト/同時/teardown・no-alloc/no-panic 検査）
- `alloc_counter`/`play_sine` との共存判断・オーバーヘッド・
  CPU sampling/Tracy 制約の文書化（本書 + rustdoc）

### Out of Scope

- RT Timing（Clock/overrun/Callback budget + Histogram 統合）— 別 slice
- CPU Profiler エンジン・Allocation stack・Leak/RSS 解析 — 外部ツール責務
- `#[global_allocator]` の自動登録、`alloc_counter` の変更、
  `play_sine` example の変更（共存判断の文書化のみ）
- `diagnostics`/`collector`/`metrics`/`source` の変更、他クレート

## Components and Boundaries

```
src/profiling/
  mod.rs                 — モジュール契約・RT contract 表・feature 説明
  stats.rs               — OpStats / AllocationStats（読み出しスナップショット）
  allocator.rs           — CountingAllocator<A> + グローバル計数（atomics）
  scope.rs               — AllocationScope / AllocationWindow + TLS chain
```

### `stats.rs` — `OpStats` / `AllocationStats`

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpStats {           // Copy・公開フィールド
    pub calls: u64,            // 要求回数（alloc/alloc_zeroed/dealloc/realloc 別）
    pub succeeded: u64,        // 成功回数（dealloc は常に succeeded=calls）
    pub failed: u64,           // 失敗回数（null 返却。dealloc は常に 0）
    pub bytes: u64,            // 操作に伴うバイト数（後述の規則・飽和和）
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocationStats {   // Copy・公開フィールド
    pub alloc: OpStats,
    pub alloc_zeroed: OpStats,
    pub dealloc: OpStats,
    pub realloc: OpStats,
    pub current_bytes: i64,    // net live requested bytes（負になり得る。後述）
    pub peak_bytes: u64,       // current_bytes の max(0, ·) の最大値
}
```

`bytes` の規則: `alloc`/`alloc_zeroed` は `layout.size()`、`dealloc` は
`layout.size()`（解放サイズ）、`realloc` は `new_size`（要求サイズ）。
要求ベースであり、成功/失敗に関わらず要求が発生した時点で加算する。

`current_bytes` の規則: `alloc`/`alloc_zeroed` 成功で `+size`、
`dealloc` で `-size`、`realloc` 成功で `+(new_size - old_size)`
（new_size < old_size なら負）。realloc 失敗・alloc 系失敗では不変。
プロセス全体の `CountingAllocator` では全割当が同一アロケータを通る
ため通常は非負だが、`AllocationScope` では window 開始前に確保され
window 内で解放されたブロックが負のドリフトを生み得る — `i64` で
正直に表現し、負値を 0 に丸めない。`peak_bytes` は `max(0, current)`
の最大値。

### `allocator.rs` — `CountingAllocator<A>`

```rust
pub struct CountingAllocator<A: GlobalAlloc> {
    inner: A,
    stats: GlobalStats,   // AtomicU64 × ops + AtomicI64 current + AtomicU64 peak
}
```

- `pub const fn new(inner: A) -> Self` — `#[global_allocator]` 用に
  const 構築可能（`CountingAllocator::<System>::new(System)`）。
- `pub fn inner(&self) -> &A`、`pub fn stats(&self) -> AllocationStats`
  （Relaxed load の非トランザクション・スナップショット）。
- `unsafe impl<A: GlobalAlloc> GlobalAlloc` — 4 API 全て override:

| API | 委譲 | 計数 |
| --- | --- | --- |
| `alloc(layout)` | `inner.alloc(layout)` | alloc.calls/bytes、結果で succeeded/failed、成功時 current/peak |
| `alloc_zeroed(layout)` | `inner.alloc_zeroed(layout)` | 同上（alloc_zeroed op として別計数） |
| `dealloc(ptr, layout)` | `inner.dealloc(ptr, layout)` | dealloc.calls/bytes、current -= size |
| `realloc(ptr, layout, new_size)` | `inner.realloc(ptr, layout, new_size)` | realloc.calls/bytes(=new_size)、成功時のみ succeeded + current += new-old、失敗時 failed（旧ブロック生存・current 不変） |

- default impl に依存しない理由: `alloc_zeroed` の default は
  `self.alloc` + `write_bytes` を呼ぶため override しないと
  alloc_zeroed が alloc イベントに化け、realloc の default は
  alloc+copy+dealloc に分解され realloc 独立イベントを失う。

#### アロケータ経路の禁止事項（契約）

`GlobalAlloc` trait 自身が「allocator が unwind するのは UB」
「現在割当済みメモリに触れない」を要求する。本実装はさらに:

- **alloc しない**: TLS は const 初期化 `thread_local!`（`std::thread_local`
  は `#[global_allocator]` を使わないことが std で保証済み）、
  カウンタは `Cell`/`Atomic*` のみ。`Vec`/`Box`/`format!` 等を使わない。
- **panic しない**: `try_with` の `Err`（TLS 破棄中）は握り潰して
  「計数しない」に縮退。indexing/unwrap/expect を経路に置かない。
- **tracing/ログを出さない**: 出力系は一切呼ばない。
- **重い lock を取らない**: atomic のみ。Mutex/RwLock 不使用
  （std::sync::Mutex はプラットフォームにより内部で割当し得る）。
- **再帰しない**: 内側 `A` は leaf allocator（`System` 等）を想定。
  `A` が global allocator 経由で割当する実装だと再帰するが、それは
  `A` の性質であり本 wrapper の責務外 — rustdoc に明記。

#### カウンタの更新規則（global / scope 共通）

- 単調カウンタ（calls/succeeded/failed/bytes）: u64 飽和和
  （`saturating_add` 相当。global は `compare_exchange` の CAS ループ、
  scope は `Cell::set(get().saturating_add())`）。`fetch_update` は
  toolchain で deprecated、`try_update` は MSRV 1.89 を超えるため
  CAS を明示する。
- `current_bytes`: 同じく CAS ループで `i64` 飽和加算し、成功値
  `new` から `peak.fetch_max(max(0, new))`。scope 側は単一スレッド
  アクセスのため `Cell` で同じ更新則を適用。

### `scope.rs` — `AllocationScope` / `AllocationWindow`

TLS に const 初期化のスタックを置かず、**active scope を intrusive
linked list** で繋ぐ。各ノードは scope が所有する `Box` 上にあり、
アドレスは scope 生存中安定。

```rust
thread_local! {
    static HEAD: Cell<usize> = const { Cell::new(0) };  // *mut ScopeNode as usize
}

struct ScopeNode {               // AllocationScope が Box で所有
    parent: Cell<usize>,         // 1 つ上の active scope（0 = 根）
    active: Cell<bool>,          // window が生存中か
    // OpStats 相当の Cell<u64> ×4 op + Cell<i64> current + Cell<u64> peak
}

pub struct AllocationScope {
    node: Box<ScopeNode>,        // new() で 1 回だけ割当（非 RT）
    _thread_affine: PhantomData<*const ()>,  // !Send + !Sync を強制
}

pub struct AllocationWindow<'a> {
    scope: &'a mut AllocationScope,
    _not_send: PhantomData<*mut ()>,   // !Send + !Sync を強制
}
```

#### enter / exit

- `AllocationScope::new()` — `Box<ScopeNode>` を確保（**非 RT only**、
  割当 1 回）。カウンタは 0 初期化。
- `scope.enter() -> AllocationWindow<'_>`:
  1. `node.active` が既に true なら panic（前の window が
     `mem::forget` 等で閉じられていない — safe code の論理エラーであり
     allocator 経路ではないので panic は契約違反にならない）。
  2. `HEAD.try_with` で現 head を取得し `node.parent` に保存、
     `HEAD` を node に更新、`active = true`。
     `try_with` が `Err`（TLS 破棄中）なら **push せず**
     `active = false` のまま inert window を返す（計数されない）。
  3. window は `&mut scope` を保持するため scope の move/drop を
     borrow 規則で防ぐ。
- `AllocationWindow::stats() -> AllocationStats` — 生存中の
  ライブスナップショット。
- `AllocationWindow::drop` — `HEAD` チェーンから自ノードを unlink
  （head 一致なら差し替え、それ以外は `parent` を辿る walk で
  predecessor の `parent` を自ノードの `parent` に付け替え）、
  `active = false`。**LIFO 順でなくても正しく除去される**。
  `try_with` が `Err` なら TLS が既に死んでいるため何もしない
  （チェーンごと消える）。
- `AllocationScope::stats()` — window 非生存時の累積スナップショット。
  `AllocationStats` は `Copy` のプレーン構造体であり、Cell 読み出し
  のみで割当を伴わない。
- `AllocationScope::reset()` — カウンタを 0 に戻す（window 非生存時
  のみ呼べる。borrow が強制）。Cell 書き込みのみ。
- `AllocationScope::measure(f) -> (R, AllocationStats)` 利便 API —
  new→enter→f→close を一括し、戻り値を先・統計を後で返す。
  f が panic しても window の Drop で unlink される。内部で
  `new()`（Box 割当）を行うため非 RT。

#### 計数セマンティクス

- allocator の各 API は global stats を更新した後、`HEAD` チェーンを
  先頭から `parent == 0` まで辿り、**全 active scope のノードを
  更新**する。ネストした scope は内側・外側ともに同じイベントを
  計数する（外側 = 内側を含む全区間、内側 = その部分区間）。
- スコープは**呼出スレッドのみ**を計数する。他スレッドの割当は
  そのスレッドの `HEAD` にしか現れず、process-wide の把握は
  `CountingAllocator` 自身の stats が担う。TLS Scope が全 process の
  無割当を保証することはない（設計上の明示的ギャップ）。
- `current_bytes`/`peak_bytes` は scope ごとに独立。window 前に
  確保されたブロックの解放は負ドリフトを生む（`i64` で表現）。
- scope のカウンタは **scope 生成時（または `reset()`）からの累積**
  であり、window 単位のリセットは行わない。window 毎の独立計測には
  `AllocationScope::measure`（毎回新規 scope）を使うか、window 間で
  `reset()` を挟んで差分を取る。

#### 安全性（unsafe の局所化と不変条件）

unsafe は 2 箇所のみ、いずれも `#[expect(unsafe_code)]` + SAFETY
コメント付き:

1. `GlobalAlloc` impl の内側 `A` への委譲呼出（ptr/layout 契約の
   転送。`unsafe_code` は impl ブロックに付与）。
2. TLS チェーンの `usize → *const ScopeNode` 復元と `parent` 走査。
   **不変条件**: チェーン上のノードは「active window または未 close
   の scope」が生存する限り有効であり、除去は必ず free より先に
   行われる。これを保証するため:
   - node の free は `AllocationScope::drop`（Box 解放）のみ。
   - window drop・scope drop の両方が unlink を試行する
     （window drop が通常経路、scope drop は `mem::forget` や
     TLS-teardown で残った場合の保険）。unlink は冪等
     （見つからなければ no-op）。
   - `mem::forget(window)` は「window が閉じない」だけで dangling
     を生じない — node は scope の Box が保持し、scope drop 時に
     unlink→free の順で処理される。ただし `active` が true のまま
     残るため、当該 scope は再 enter すると panic し**再利用不可**
     となる（drop して新しい scope を作る。計測値も失われる）。
     これは「計測ウィンドウを閉じ忘れた」ことへの正直な応答であり、
     リーク・UB にはならない。
   - enter の `active` チェックが同一 node の二重 link（自己
     参照 cycle）を防ぐ。

`AllocationScope` は **!Send + !Sync**（`PhantomData<*const ()>` で
固定）。`Cell` 含有により `!Sync` は自動だが、`Send` も明示的に
剥がす: `mem::forget(window)` で window を漏らした scope は node が
**enter したスレッドの** TLS chain に残ったままになり、scope を別
スレッドへ move して drop すると unlink が誤った TLS を走査して
node が dangling になる（UAF）。スレッド親和の型にすることでこの
経路を型レベルで閉じる。計測は常に「そのスレッドで scope を作り、
そのスレッドで enter する」形になり、これは TLS 計測の自然な使い方
と一致する。`AllocationWindow` も `PhantomData<*mut ()>` で
**!Send + !Sync** — enter したスレッド以外で drop されると誤った
TLS を unlink するため、型レベルで禁止する。

#### RT contract

| 操作 | 契約 |
| --- | --- |
| `AllocationScope::new` / `AllocationScope::measure` | **非 RT only** — `new` は Box 割当を伴う（`measure` は内部で `new` を呼ぶ） |
| `scope.enter()` / `window.stats()` / `window` Drop / `scope.stats()` / `scope.reset()` | RT-suitable（設計上）— TLS `try_with` + Cell/atomic のみ、alloc/panic/lock なし。`enter` の panic は `active` 二重 enter（前 window 未 close）の論理エラー時のみで、allocator 経路には到達しない |
| `CountingAllocator` の 4 API + `stats()` | 計数部は no-alloc/no-panic。委譲先 `A` の性質に依存（`System` は RT 不適だが計装自体は bounded） |

`enter`/window Drop を RT callback の計測区間に使う場合、scope は
そのスレッドで RT 開始前に `new` しておき（`new` は 1 回だけの
割当を伴う）、callback 毎に enter/exit する運用を想定する
（window の生成・破棄・stats 読み出しに割当はない）。

## Data and Control Flow

1. Host/Test Binary が `#[global_allocator] static ALLOC:
   CountingAllocator<System> = CountingAllocator::new(System);` を宣言。
2. 全プロセスの割当が `CountingAllocator` の 4 API を通過 →
   global stats（atomics）を更新 + 呼出スレッドの `HEAD` チェーン上の
   全 active scope を更新。
3. Host は `ALLOC.stats()` でプロセス累積を、`AllocationScope` で
   スレッド/区間別の内訳を読む。

## Non-Functional Constraints

- **feature `allocation-probe` は既定 OFF・依存ゼロ**。OFF 時は
  `profiling` モジュールごとコンパイル対象外（`#[cfg]` gate）。
- **オーバーヘッド**: 計装 ON の各割当は atomic 数回 + TLS walk
  （通常 depth 1）。OFF（feature 無効）ではコード自体が存在しない
  ためオーバーヘッドゼロ。計測 ON/OFF の性能差は #281 gate の
  実機評価に委ね、本 slice では機構上のコスト構造のみを記録する。
- **正確さの限界**: `AllocationStats` は要求ベースの計数であり、
  System RSS・実ヒープ占有・正確な leak 件数とは同義でない
  （アロケータ内部の丸め・OS 割当は見えない）。`realloc` の
  in-place 成否は内側 `A` 依存で区別しない（new_size ベースで計数）。
- **process-wide gap**: TLS Scope は呼出スレッドのみ。全スレッドを
  測るには各スレッドに Window を設置するか `CountingAllocator` の
  global stats を使う — 両者は別物として文書化。

## Risks and Trade-offs

| # | Risk | Mitigation |
| --- | --- | --- |
| 1 | allocator 経路での再帰・panic・割当による UB/ハング | const-init TLS + `try_with`、Cell/atomic のみ、panic 経路を置かない、`std::thread_local` が global allocator を使わない std 保証を利用。`active` flag で cycle 防止 |
| 2 | `mem::forget`/非 LIFO drop での dangling・計数破壊 | node は scope の Box が所有、unlink を window/scope 両 drop で冪等実行、scope drop で unlink→free 順序を保証。`active` flag で二重 link 防止。forget は「計測リーク」として文書化 |
| 3 | `realloc` 失敗時の current_bytes 誤差 | 失敗時は succeeded/failed のみ更新し current/peak に触れない（旧ブロック生存の契約に一致） |
| 4 | saturating counter の wrap | u64 飽和和を全カウンタに適用。current_bytes は i64 wrapping だが 2^63 bytes の net live は実質不可能として文書化 |
| 5 | scope の Send/Sync 誤用 | Scope を `PhantomData<*const ()>`、Window を `PhantomData<*mut ()>` で共に !Send+!Sync に固定。forgotten window 残存 node の UAF/誤計数経路を型で閉じる |
| 6 | `alloc_counter`/`play_sine` との重複 | 移行ではなく共存判断を文書化（下記）。既存 RT lane は不変 |

## 既存資産との共存・移行判断

- **`ruviform-rt::alloc_counter`**: `#[cfg(test)]` 限定・private・
  イベント数のみ。本 slice は**変更しない・移行しない**。理由:
  (a) test binary 専用 install 済みで共有不能、(b) bytes 計数を
  持たず RT lane の「0 件」検証には十分、(c) 既存 RT test lane
  （`cargo test rt_`）の意味を変えないことが要件。将来
  `#[cfg(test)]` を超えた計測需要が出た場合の移行先は本
  `profiling::CountingAllocator`（install は各バイナリの責務）。
- **`play_sine.rs` の `CountingAlloc`**: example バイナリ専用の
  static counter 群。本 slice では**変更しない**。example を
  `CountingAllocator` へ移行するかは example の計測目的
  （callback 回数/xrun 等の独自 static 群を持つ）に依存し、
  別途判断する — 本 PR の範囲外として記録。
- **棲み分け**: `alloc_counter`/`assert_no_alloc` は「割当が
  起きないこと」の RT 契約検証レーン、本 `profiling` は
  「割当がどれだけ起きたか」の開発・テスト計測。両者は補完関係。

## CPU sampling / 任意 nonRT Layer の制約（文書化）

- **CPU sampling**: 外部プロファイラ（samply 等）で非 RT の
  Operation/Span をプロセスに対応づける。手順: (1) 対象バイナリを
  `allocation-probe` 有効でビルド、(2) `samply record` 等でサンプリ
  ング、(3) 非 RT 側の計装ポイント（tracing span 等、OBS-03 の
  責務）とサンプルを対応づける。AudioLens の samply 経路は置換
  しない。
- **Tracy 等の任意 nonRT Layer**: `tracing`/Tracy 連携は Host が
  明示選択するオプションであり、通常 RT callback から
  `tracing`/Tracy API/Subscriber を呼ばない。`allocation-probe`
  自体は tracing に依存しない（feature 依存ゼロ）。
- **計測の ON/OFF**: feature OFF でコード非存在（オーバーヘッド
  ゼロ）、ON で atomic 数回 + TLS walk/alloc。実機での性能差評価は
  #281 gate。

## Deferred Design Concerns

- RT Timing（Clock/overrun/Callback budget + Histogram 統合）|
  Clock Domain・budget 意味論・記録欠落の独立設計が必要 | 後続 slice /
  Issue | RT 性能観測手段の欠如継続
- CPU sampling の実機手順検証・Tracy PoC | 環境依存ツール・#281 gate | 手順陳腐化
- `play_sine` example の `CountingAlloc` 移行 | example 固有計測との
  適合判断が別途必要 | 重複パターンの残存（影響小）

## Verification Notes

- [ ] `cargo test -p ruviform-observability --features allocation-probe`
  で 4 API 委譲・stats 計数・scope セマンティクスを検証
- [ ] `assert_no_alloc` レーンで計数経路（mock inner 使用）の
  no-alloc を検証
- [ ] `mise run fmt-check`/`check`/`lint`/`test`（default feature）で
  非回帰を検証
- [ ] feature 無効時に `profiling` がコンパイル対象外・依存ゼロを
  `cargo tree`/`Cargo.toml` で確認
