---
id: "TSPEC-034JxNxYK4O6s8NA8gnaah"
type: "test-spec"
status: "approved"
title: "スレッド優先度ライフサイクルの契約保証"
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

# スレッド優先度ライフサイクルの契約保証

## Purpose

Issue #21 が `ruviform-exec` に導入するスレッド優先度機構——`promote_current_thread_to_realtime` / `demote_current_thread_from_realtime` / `ThreadPriorityParams` / `ThreadPriorityGuard` / `PriorityError`——のライフサイクル契約の「なぜテストが存在するか」を保持する。[spec](../specs/implement-rt-foundation.md) の受け入れ基準（`non-RT only` ラベル・self-promotion 呼び出しモデル・`Result` 失敗表出・非特権 Linux の失敗挙動の文書化）と [design](../designs/rt-foundation-api-design.md) の不変条件 9–10 を、実装の改変や依存バージョン更新の際に「どの振る舞いを守るべきか」を判断できる形で固定する。廃止してよいのは、優先度機構の配置・API 形状・呼び出しモデル・dbus 方針を変更する ADR / spec 改訂が行われたとき（例: dbus 経路 opt-in feature の追加、配置クレートの再評価）。

## Feature

OS スレッド優先度の付与・解除ライフサイクル: 対象スレッド自身が RT 契約区間の開始前に自己昇格（self-promotion）を呼び、失敗が `Result` で表出されてパニックせず、返された guard が解除トークンとして機能すること。

## Rules

- **`promote_current_thread_to_realtime` は `non-RT only` の self-promotion API**。ブロッキング的な OS syscall を伴うため、active な RT 契約区間内からの呼び出しは契約違反。呼び出しモデルは対象スレッド自身からの呼び出しで、exec 管理スレッドでは spawn 時セットアップ、デバイスコールバック由来スレッドでは初回コールバックの preamble（`RtContext` mint・初回 `process` 前）に呼ぶ。
- **失敗は `Result` で表出しパニックしない**。戻り値は `Result<ThreadPriorityGuard, PriorityError>` で、promotion 失敗は致命的でなく呼び出し側は promotion なしで継続できる（best-effort）。
- **promotion の成否はプラットフォーム依存で `Ok`/`Err` いずれも合法**。`default-features = false` 構成では rtkit/`CAP_SYS_NICE`/`RLIMIT_RTPRIO` のない非特権の標準デスクトップ Linux で promotion は典型的に失敗し `Err` が返る（仕様上の失敗として文書化済み）。対象外プラットフォームでは no-op 成功を返しうる（内部クレートのセマンティクスに従う）。テストは `Result` が返ることのみ検証し、成否を assert しない。
- **`demote_current_thread_from_realtime` は guard を消費する**。`demote(guard)` は `ThreadPriorityGuard` を by-value で受け取るため二重 demote は型で防止される（`non-RT only`・syscall 伴う）。
- **guard `Drop` は demotion を best-effort 試行する**。`Drop` での demotion 失敗は捨てる（失敗検出が必要なら `demote` を明示呼び出しする）。`Drop` が syscall を伴うため、RT 契約区間内で guard を生成・破棄してはならない。
- **`ThreadPriorityParams::audio` は純粋な値構築**。`buffer_frames`/`sample_rate` から内部クレートがスケジューリング周期を算出するパラメータを alloc なしで構築する（`RT-suitable` ラベル。ただし用途は非 RT セットアップ区間専用）。
- **`PriorityError` は `#[non_exhaustive]` の失敗分類**。`PromotionFailed(&'static str)`・`DemotionFailed(&'static str)` を持ち、`Copy`・`Display` + `core::error::Error` 実装。exec に crate 共通 `Error`/`Result` は作らない（失敗系統が 1 系統のみのため YAGNI — design 決定）。
- **外部型・feature 名を露出しない**。`audio_thread_priority` の型・エラー・feature 名は公開 API に一切出さない（private 実装詳細。`default-features = false` で `with_dbus`/`dep:dbus` 経路を排除し、依存 feature 名の再露出に相当する独自 feature も追加しない）。
- **テスト命名は `rt_` 部分文字列を含まない**。promotion テストは RT 契約テストではないため `rt_` を含まない命名（例: `promotion_returns_result`）とし、モジュールパス・関数名・ヘルパー名に `rt_` 部分文字列が現れないことを確認する（`assert_*`・`start_*` 禁止 → `expect_*`・`spawn_*` 等）。

## Examples

- Given: `std::thread::spawn` したスレッド内。When: `promote_current_thread_to_realtime(ThreadPriorityParams::audio(256, 48000))` を呼ぶ。Then: パニックせず `Result` が返る（`Ok` なら guard 経由で demote 可能、`Err` なら `PriorityError`）（`promotion_returns_result` — `rt_` 非含有命名）。
- Given: promotion に成功し guard を取得済み。When: `demote_current_thread_from_realtime(guard)` を呼ぶ。Then: guard は消費され、二度目の demote 呼び出しは型で不可能（コンパイル契約 — コード形状として担保）。
- Given: guard を `demote` に渡さず drop だけする。Then: `Drop` で demotion が best-effort 試行される（失敗しても静黙 — 検出が必要な経路では `demote` を使う）。
- Given: `ThreadPriorityParams::audio(buffer_frames, sample_rate)`。When: 構築する。Then: alloc なしの純粋な値構築として `promote` に渡せる。
- Given: 非特権 Linux 環境（rtkit/`CAP_SYS_NICE`/`RLIMIT_RTPRIO` なし）。When: `promote` を呼ぶ。Then: `Err` が返ってもパニックせず、呼び出し側が promotion なしで継続できる（文書化済みの仕様上の失敗）。

## Guarantees

- promotion 失敗が致命的でなく、呼び出し側が `Err` を受けて promotion なしで継続できることが `Result` 形状で保たれる（パニック経路が存在しない）。
- 二重 demote が guard 消費の型で防止され、guard の RAII 的解除（best-effort）と明示的解除（失敗検出可能）の使い分けが可能。
- RT 契約区間との境界（`non-RT only`・RT 区間開始前の呼び出しモデル）が API ドキュメント契約として明確。
- `audio_thread_priority` の外部型・feature 名が公開 API に露出せず、依存差替・feature 変更が breaking 化しない。

## Non-goals

- per-OS 実効差の検証（MMCSS「Pro Audio」/time-constraint/`SCHED_FIFO` のパラメータ写像詳細は実機検証・audio-device Issue の責務 — spec Deferred Design Concerns）。
- promotion 成功の保証（非特権 Linux では失敗が仕様。`Ok`/`Err` のどちらを返すかは assert しない）。
- 実際に優先度が上がったことの検証（内部クレートのセマンティクスに依拠。OS 状態の観測はプラットフォーム依存で CI 不安定）。
- dbus 経路を再有効化する intent 命名 feature（`default-features = false` 固定。将来フォローアップ候補）。
- `ruviform-rt` 側への同等機構の配置（Primitives 層「OS 固有 API・デバイス依存を持たない」制約により exec 確定 — ADR）。
- RT 契約テストとしての配置（`non-RT only` API のため `rt_` レーン対象外 — `rt_` 非含有命名の確認が本 spec の関心事）。
- exec crate 共通 `Error`/`Result` の存在（`PriorityError` はモジュール単位の失敗分類 — YAGNI 決定）。

## Risk

- `promote` が panic 経路を持つと「promotion 失敗は best-effort」契約が崩れ、RT スレッドの起動経路が致命的失敗で中断される。
- guard が二重 demote を許すと OS 側状態との不整合が起きうる（by-value 消費の型防止が崩れる）。
- `rt_` を含むテスト・関数名を付けると非 RT テストが `mise run test-rt` の RT レーンに混入し、選択フィルタの意味が壊れる（`promotion_returns_result` 等の命名規約が守られない場合）。
- 外部型・feature 名を公開 API に露出すると、`audio_thread_priority` の差替・バージョン更新が breaking change 化する。
- RT 契約区間内での promotion/demotion 呼び出しが許容されると syscall が RT パスに混入し、RT 契約全体が空洞化する。
- 非特権 Linux の失敗挙動が文書化されないと、利用者が `Err` を実装バグと誤認する。
