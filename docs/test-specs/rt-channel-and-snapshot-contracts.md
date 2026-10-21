---
id: "TSPEC-034JxNwt3ReMphshEaeFVe"
type: "test-spec"
status: "approved"
title: "SPSC・退役・スナップショット交換の RT 契約保証"
created: "2026-10-21"
updated: "2026-10-21"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/rt-foundation-api-design.md"
  derived-by: []
  verifies:
    - "../specs/implement-rt-foundation.md"
    - "../designs/rt-foundation-api-design.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# SPSC・退役・スナップショット交換の RT 契約保証

## Purpose

Issue #21 が導入する RT ↔ 非 RT 受け渡しプリミティブ——SPSC チャネル、退役（遅延ドロップ）チャネル、不変スナップショット交換——の振る舞い契約の「なぜテストが存在するか」を保持する。[spec](../specs/implement-rt-foundation.md) の受け入れ基準（SPSC の満杯時値返却・retire の非 RT drop・満杯時 leak-on-full・guard 経由スナップショット）と [design](../designs/rt-foundation-api-design.md) の不変条件 4–6 を、実装の改変や採用依存の差替（`rtrb` → `ringbuf` 再評価等）の際に「どの振る舞いを守るべきか」「そのテストはまだ必要か」を判断できる形で固定する。廃止してよいのは、対象契約自体を変更する ADR / spec 改訂が行われたとき（例: 退役経路の満杯時方針変更、スナップショットの複数 reader 化、満杯時方針のリングレベル一般化）。

## Feature

RT ↔ 非 RT 間の有界・ノンブロッキング受け渡し契約: SPSC が構築時固定容量の FIFO で満杯時に値を呼び出し元へ返し、退役チャネルが退役オブジェクトの `Drop` を非 RT 側 drain に限定し満杯時 `core::mem::forget` の leak-on-full とし、スナップショット交換が専用 guard 経由参照で guard 解放と書き込み側差替の双方を同一退役経路へ送出すること。

## Rules

- **SPSC は構築時固定容量の有界 FIFO である**。容量は構築時に確定し以後の伸長はない（事前確保型の受け渡し構造）。`try_push`/`try_pop` はノンブロッキング・alloc なし・有界で、push した順に値を保全して返す。
- **満杯時方針は API 単位で表現し、値を呼び出し元へ返す**。`try_push` は満杯時に `Err(TryPushError::Full(value))` で値を返却し（`into_inner` で回収可能）、drop/coalesce/keep のポリシー適用は呼び出し側に委ねる。汎用のリングレベル方針 enum は持たない。`try_pop` は空時に `Err(TryPopError::Empty)` を返す。
- **latest-wins は consumer 側 `pop_latest` が担う**。保留要素を最後まで pop して最新 1 件だけを返す（パラメータ統合向け）。ループ回数は固定容量で有界。空なら `None`。
- **容量・空満診断は近似値である**。`is_full`/`is_empty`/`capacity` の戻り値は観測時点の近似として扱い、正確性を契約としない（SPSC リングの同時更新を考慮した診断用）。
- **RT 側操作は alloc しない**。`try_push`/`try_pop`/`pop_latest`/スナップショット `read`/guard `Drop` は、std のみのテストスコープ counting `#[global_allocator]`（外部依存なし。enable/reset/assert-zero/disable フラグで対象区間を限定し他テストの確保を誤計しないテスト内部装置）で確保ゼロを挙動検証する。
- **退役オブジェクトの `Drop` は非 RT 側 drain でのみ実行される**。`Retirer::retire` は `try_push` 意味（ノンブロッキング・alloc なし・有界）で RT-suitable。`DropCollector::drain` は non-RT only で、受信済み分と `push_local` ローカル投入分の双方を drain 呼び出しスレッド上で drop する（戻り値は drop 件数）。
- **退役経路の満杯時は `mem::forget` leak-on-full**。`retire` 満杯時に RT 側で drop/dealloc/blocking を行わず、`core::mem::forget` でリークに留める（drop-on-full は RT 側 dealloc になるため不採用 — ADR）。容量は drain 周期ごとの最大退役数を見込む呼び出し側契約。**リーク総量の上限は drain 間隔・生涯ともに保証しない**（有界なのはキュー容量と各 `retire` の処理量のみ。`retire` 呼び出し回数は契約上無制限）。飽和は `overflows()` カウンタで観測可能。
- **RT 側スナップショット参照は `SnapshotGuard` 経由のみ**。`read` は lock-free load + `Arc` clone（`&self` 受け・`RefCell` 内部可変で複数 guard 同時存在可）で alloc なし。guard は `Deref<Target = T>` で内容を読み、`!Send`・`!Sync`（取得した RT スレッドから逃げない）。
- **guard `Drop` は内部 `Arc` を退役経路へ送出する**。inline drop せず reader 内の `RefCell<Retirer<Arc<T>>>` へ `retire` する（`Drop` 内の `borrow_mut` は瞬間的で逐次解放。reader 側 last-drop の穴を塞ぐ中核契約）。`SnapshotWriter::publish` で追い出された旧 `Arc` も `DropCollector::push_local` 経由で同一 drain 経路へ渡し、dealloc は「非 RT 側 drain（`SnapshotWriter::collect`）」または「非 RT の `publish` 内部」でのみ発生する。
- **構築系 API は non-RT only**。`channel`/`retire_channel`/`slot`/`drain`/`collect`/`publish` は確保・伸長を伴うため `non-RT only` ラベルを持つ。
- **非 RT のテスト・doctest・ヘルパー名は `rt_` 部分文字列を含まない**。`mise run test-rt`（`cargo test --workspace rt_`）は部分文字列フィルタのため、`assert_*`・`start_*` 等の `rt_` を内包する命名は禁止（`expect_*`・`spawn_*` 等を使う）。RT 契約テストのみ `rt_` プレフィックス。

## Examples

- Given: 容量 2 の SPSC チャネル。When: 3 件 `try_push` する。Then: 3 件目は `Err(Full(v))` で値が返り `into_inner` で回収できる。`try_pop` は push 順に 2 件を返し、以後は `Err(Empty)`（`rt_push_pop_fifo`・`rt_full_try_push_returns_value`）。
- Given: 複数の保留値を持つ consumer。When: `pop_latest` を呼ぶ。Then: 最新値のみ返り残りは捨てられる。空なら `None`（`rt_pop_latest_returns_newest`）。
- Given: counting allocator を enable した区間。When: `try_push`/`try_pop`/`pop_latest`/`read`/guard drop を実行する。Then: 確保回数は 0（`rt_try_push_pop_no_alloc`・`rt_snapshot_read_no_alloc`）。
- Given: `Drop` で `AtomicBool` を立てるテスト専用型の値。When: `retire` 直後に flag を確認し、その後 `drain` を呼ぶ。Then: retire 直後は flag 未セット（RT 側で drop しない）、drain 後にセットされ drain は drop 件数を返す（`rt_retire_then_drain_drops`）。
- Given: 満杯の退役チャネル。When: さらに `retire` する。Then: 値は `mem::forget` され flag が立たない（drop も dealloc も RT 側で起きない leak-on-full）（`rt_retire_full_forgets_not_drops`）。
- Given: `publish` で差し替えたスナップショット。When: `read` する。Then: 新しいスナップショットを観測する（`rt_snapshot_publish_visibility`）。
- Given: `read` で取得した guard。When: guard を drop する。Then: 内部 `Arc` の `strong_count` が >0 のまま残り（RT 側で dealloc しない）、`SnapshotWriter::collect` 後に解放される（`rt_snapshot_guard_drop_routes_to_retire`）。
- Given: `publish` で追い出された旧 `Arc`。When: `collect` する。Then: 旧 `Arc` が同一 drain 経路で解放される（`rt_snapshot_displaced_arc_freed_on_collect`）。

## Guarantees

- RT パス上の通知・退役・スナップショット参照が alloc/dealloc・ブロッキング・無制限待機なしで行えることが counting allocator と Drop-flag による挙動テストで保たれる。
- 満杯時に値が静かに失われず（`Full(T)` 返却で呼び出し側ポリシーが適用可能）、退役経路で失われる場合も文書化済みの leak-on-full に留まる。
- スナップショットの dealloc が RT スレッド上で発生しない（reader 側 last-drop の穴が guard ルーティングで塞がれる）——`arc-swap` 単体では実現できない本プリミティブの中核契約が保たれる。
- `T: Send`（チャネルペイロード）・`T: Send + 'static`（退役オブジェクト）・`T: Send + Sync`（共有スナップショット内容）のスレッド境界が API 契約で明示される。

## Non-goals

- スループット・レイテンシ等の性能計量（`mise run bench` は入口のみ・計量対象外）。
- 複数 RT reader / MPMC 化（現行は単一 reader — retire channel が SPSC。design Out of Scope）。
- `assert_no_alloc` 型 CI レーン・恒常検査機構（RT 検証 Issue の責務。本 spec は test-scoped counting allocator のみ）。
- `rtrb`/`arc-swap` 内部実装の正当性（採用クレートの保証に依拠。依存差替時は本契約を再検証する）。
- 端点型の `Send`/`!Sync` 宣言検査自体（[rt-context-and-processor-contract](rt-context-and-processor-contract.md) の generic helper 検証に集約）。
- `**RT contract:**` ラベル文言の verbatim 検証（doc レビュー領域。本 spec は挙動契約を扱う）。
- `try_push`/`try_pop` の wait-free 性の厳密証明（`rtrb` の保証に依拠し、テストではノンブロッキング完了を確認するに留める）。

## Risk

- 満杯時方針をリングレベル enum で一般化すると、値返却による呼び出し側ポリシー適用点を失い、drop/coalesce/keep と latest-wins の使い分けが破綻する。
- guard が内部 `Arc` を inline drop すると reader 側 last-drop の dealloc が RT スレッドに乗り、RT 契約違反が検出困難な形で混入する。
- 満杯時フォールバックを `mem::forget` から drop-on-full に変えると RT 側 dealloc が復活し、退役経路契約全体が崩れる。
- `pop_latest` が「先頭 pop」に退化するとパラメータ統合が古い値で上書きされ、latest-wins 適用の意味が失われる。
- `drain`/`collect` が RT 側から呼べると dealloc 地点の分離契約が崩れ、`non-RT only` ラベルの意味が空洞化する。
- `rt_` 命名規約が守られないと非 RT テストが RT レーンに混入し、`test-rt` の選択意味が壊れる（`assert_*`・`start_*` は `rt_` を内包する）。
