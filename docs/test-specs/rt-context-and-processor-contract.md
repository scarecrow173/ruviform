---
id: "TSPEC-034JxNxcYxa29rPklpknWh"
type: "test-spec"
status: "approved"
title: "RtContext・Processor 契約・MasterClock の RT 契約保証"
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
    - "../adr/realtime-contract-and-verification-policy.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
---

# RtContext・Processor 契約・MasterClock の RT 契約保証

## Purpose

Issue #21 が導入する RT/非 RT 分離の型レベル境界——`RtContext` capability、`Processor`/`RtProcessor` 二 trait、`MasterClock`、共通 `Error`/`Result`、および端点型の `Send`/`Sync` 境界——の契約意図を保持する。[spec](../specs/implement-rt-foundation.md) の受け入れ基準（opaque トークン・二 trait 分離・`process` の `RtContext` 要求・Send/Sync 境界・時計・共通エラー・`rt_` 命名）と [RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md) の「保証範囲は API 単位で定義・明示」規則を、上位層（engines・audio-device・exec の駆動側）が依存する境界として固定する。廃止してよいのは、capability/mint 方式・trait 分離・時計モデル・RT ラベル規則を変更する ADR / spec 改訂が行われたとき（例: capability factory 型への移行、`dyn` 前提形状への変更）。

## Feature

RT 契約区間の型レベル表現: `RtContext` が発行制限付きの opaque トークンとして `process` 呼び出しの証明となり、`prepare`/`process` のライフサイクル分離が二 trait で表現され、`MasterClock` が単一論理 writer の wait-free 時計を提供し、全端点型が `Send`/`Sync` 境界を API 契約として宣言すること。

## Rules

- **`RtContext` は単一の opaque 型である**。公開コンストラクタ・公開フィールドを持たず（`_private` がリテラル構築を防止）、オフライン/RT を区別する実行時モードフラグや variant を持たない（「同一 API 内部で if rt 分岐しない」原則）。オフライン発行コンテキストでの制約緩和は実行時状態ではなく発行駆動者に紐づく文書契約。
- **mint は `#[doc(hidden)]` 経由で駆動者入口点にのみ提供される**。`mint()` は `RT-suitable` ラベル付きで、mint した `&RtContext` を `RtProcessor::process` に渡せる。利用制限は「doc-hidden + 駆動者限定規約 + コードレビュー」の規約レベル強制であり、型レベルの完全抑止ではない（ADR 記録済みの弱体化）。
- **`Processor::prepare` は non-RT only で `RtProcessor` を生成する**。`Result<Self::Rt, Self::Error>` を返し、呼び出すたびに新しい RT 処理オブジェクトを返す（再 `prepare` は型上許容。退役は呼び出し側責務）。`type Error: core::error::Error` でプロセッサ固有の失敗分類を束縛する。
- **`RtProcessor::process` は `&RtContext` を要求し infallible**。`process(&mut self, io: &mut Self::Io, ctx: &RtContext)` は `Result` を返さず、処理上の失敗は `io` ドメインの状態として表現する。`RtContext` を持たないコードは `process` を呼べないため、契約区間外実行を型で抑止する。
- **associated type 形状であり `dyn` 前提にしない**。`type Rt: RtProcessor`、`type Io` はジェネリックに保ち上位クレートが具体化する。`Rt` に `Send` 境界は課さない（駆動側が自身の API で追加要求する）。
- **端点型の `Send`/`Sync` 境界は API 契約として宣言される**。`Producer`/`Consumer`/`Retirer`/`DropCollector`/`SnapshotWriter`/`SnapshotReader` は `Send`・`!Sync`（単一スレッド所有）、`SnapshotGuard` は `!Send`・`!Sync`、`MasterClock` は `Send + Sync`（`Arc` 共有前提）。ペイロード境界は `T: Send`（チャネル）・`T: Send + 'static`（退役）・`T: Send + Sync`（共有スナップショット）。検証は std のみの generic ヘルパー（`expect_send::<T>()`・`expect_send_sync::<T>()`・`expect_not_sync::<T>()` 等）で行う。
- **`MasterClock` の timebase は構築時固定で以後不変**。`new(num, den)` は 0 成分に対し `Err(Error::InvalidValue("timebase"))` を返す。timebase 不変のため tearing/seqlock は不要。`publish_tick` は単一論理 writer（現役駆動者）の文書契約による atomic store、`tick()` は wait-free load で公開値を観測できる。`Arc<MasterClock>` で共有する。
- **共通 `Error` は `Copy` + `&'static str` で alloc なし**。`#[non_exhaustive]`・`Display` + `core::error::Error` 実装・`InvalidValue(&'static str)` のみを持ち、RT パス上でも生成・保持・返却できる（`RT-suitable`）。
- **全公開 enum に enum レベル `#[non_exhaustive]`、全公開 API に `**RT contract:**` ラベルと doctest**。構築・確保系は `non-RT only`、RT 側操作は `RT-suitable`（RT 契約 ADR の API 単位ラベル規則）。
- **`rt_` 命名規約**: RT 契約テストは `rt_` プレフィックス名（`mise run test-rt` で選択）。非 RT のテスト・doctest・ヘルパー関数名は `rt_` 部分文字列を含まない（`assert_*`・`start_*` は `rt_` を内包するため禁止 — `expect_*`・`spawn_*` 等を使う）。

## Examples

- Given: テスト用 `Processor` 実装（`Setup`/`Rt`/`Error` を具体化）。When: `prepare(setup)` → 得た `Rt` に `process(io, &ctx)` を呼ぶ。Then: prepare→process のライフサイクルが成立し、`process` が `&RtContext` を要求するシグネチャであることが利用側から確認できる（`rt_prepare_then_process`）。
- Given: `#[doc(hidden)] mint()`。When: 呼び出す。Then: `RtContext` を得て `process` に渡せる（`rt_mint_token_usable`）。
- Given: `expect_send::<Producer<u8>>()`・`expect_send_sync::<MasterClock>()` 等の generic ヘルパー。When: テストをコンパイルする。Then: 端点型の `Send`/`Sync` 宣言が型レベルで検証される（ヘルパー名は `rt_` 非含有。`expect_*` 命名）。
- Given: `MasterClock::new(0, 48000)` / `new(1, 0)`。When: 呼び出す。Then: `Err(Error::InvalidValue("timebase"))` が返る。`new(1, 48000)` なら `timebase()` が構築値 `(1, 48000)` を返し続ける（`rt_clock_timebase_fixed`）。
- Given: `publish_tick(42)`。When: 別参照から `tick()` を読む。Then: 42 を観測できる（`rt_clock_tick_visibility`・単一 writer 契約）。
- Given: `RtContext` のリテラル構築を試みる。Then: `_private` フィールドにより構築不能（型レベル契約 — API 形状レビューで担保。実行時テストや compile-fail 基盤は不要）。
- Given: クレート外の `match` で `Error` を網羅しようとする。Then: `#[non_exhaustive]` により wildcard arm が要求される（将来の variant 追加が non-breaking）。

## Guarantees

- `process` が `&RtContext` を要求することで契約区間外実行が型で抑止され、「prepare 済みか」が型で表現される（単一 trait 同居では表せない構造）。
- `RtContext` の構築経路が `#[doc(hidden)] mint` のみに集約され、駆動者入口点以外からの発行が規約上追跡可能。
- 端点型の `Send`/`!Sync`・ペイロード境界が API 契約として宣言され、generic ヘルパーによるコンパイル検証で非 `Send` 型の RT スレッド流出が防がれる。
- `MasterClock` の timebase 不変性により wait-free 読み取りが保たれ、単一論理 writer 規約で時計の単調性が維持される。
- `Error` が `Copy`・alloc なしで RT パス上でも返却可能であり、`#[non_exhaustive]` で将来の variant 追加が non-breaking。

## Non-goals

- mint 権限の型レベル完全抑止（規約 + レビュー強制 — ADR が弱体化として記録。capability factory 型は不採用）。
- オフライン発行コンテキストでの RT 制約緩和の実行時表現（発行駆動者に紐づく文書契約でありランタイム状態ではない）。
- `dyn Processor` の object safety（associated type 採用。消費側 adapter で解決 — deferred）。
- `process` の実処理振る舞い・`Io` の具体型（rt は最下位の契約定義のみ。`Io` は generic で上位クレートが具体化）。
- tick→秒・他 timebase への変換（`MediaTime` 等 media-core 型への依存は持たない — 上位層の責務）。
- 複数 writer の同時 `publish_tick` の意味論（atomic なので破綻しないが、単調性は単一 writer 規約依存 — 型で強制しない文書契約）。
- `assert_no_alloc` 型 CI レーン・thread-check 機構（RT 検証 Issue の責務）。
- チャネル・退役・スナップショットの挙動契約（[rt-channel-and-snapshot-contracts](rt-channel-and-snapshot-contracts.md) の保証対象）。
- スレッド優先度 API のライフサイクル（[thread-priority-lifecycle](thread-priority-lifecycle.md) の保証対象）。

## Risk

- `RtContext` が公開構築可能・モードフラグ化すると「同一 API 内部で if rt 分岐しない」原則が崩れ、契約区間の意味が失われる。
- `process` が `Result` を返すと infallible 契約が破れ、処理失敗の表現先（`io` ドメイン状態）が不明確になる。
- `RtContext` を要求しない `process` 形状に退化すると、契約区間外実行の型レベル抑止が失われ「RT-suitable」ラベルが証明なしの宣伝になる。
- `Send`/`Sync` 境界宣言が漏れると非 `Send` 値がスレッドを越境し、単一 RT スレッド前提（`!Sync`）が暗黙に破られる。
- `rt_` 非含有命名規約が漏れると非 RT テスト・doctest が RT レーンに混入し、`test-rt` の選択意味が壊れる（`assert_*`/`start_*` は `rt_` を内包 — Issue #18 で `convert_exact` の誤選択先例あり）。
- timebase が構築後に可変だと tearing 対策（seqlock 等）が必要になり wait-free 読み取りが崩れる。
- 共通 `Error` が `String` ペイロード等の allocating 形状になると RT パス上でエラーを返せなくなる。
