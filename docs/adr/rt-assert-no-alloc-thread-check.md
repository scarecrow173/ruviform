---
id: "ADR-034JOU73yGYKDc21f7C2SY"
type: "adr"
status: "accepted"
title: "RT 検証機構の選定（assert_no_alloc 標準化・thread-check 意味論）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/22"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/block.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "realtime-contract-and-verification-policy.md"
    - "../designs/rt-foundation-api-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "select-audio-device-backend.md"
    - "../specs/rt-verification-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/rt-verification-lane.md"
    - "realtime-contract-and-verification-policy.md"
  derived-by:
    - "../designs/rt-verification-lane-design.md"
    - "../plans/implement-issue-22-rt-verification.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/22"
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# RT 検証機構の選定（assert_no_alloc 標準化・thread-check 意味論）

## Context and Problem Statement

RT 契約 ADR（`realtime-contract-and-verification-policy.md`）は「検証可能な違反は CI で自動検出」を確定済みだが、allocation 検出の機械検査は `ruviform-audio-device` の `assert_no_alloc`（`AllocDisabler`）と `ruviform-rt` の crate-local `alloc_counter`（std-only counting `GlobalAlloc`）に分散しており、プラグイン境界（`plugin-clap` の `process`・`plugin-vst3` の `prepare_block`/`finish_block`）は「構造上 allocation-free」の静的論証のみで機械検証がない。スレッド契約（audio thread での `process` 呼出）は検出機構が存在しない。本 ADR は allocation 検出のワークスペース標準と CLAP `thread-check` 拡張の意味論（何を・どこで・どう失敗させるか）を確定する。

## Decision Drivers

* Issue #22 受け入れ条件: `assert_no_alloc` 型の allocation 検出レーン・CLAP thread-check 拡張相当のスレッド契約検査の統合
* 既存の 2 系統（`assert_no_alloc` crate・crate-local `alloc_counter`）が共存する現状をどう正規化するか
* `process` の thread-check 照会が RT パスに 1 FFI call を追加する点の妥当性
* 「CI 成功 ≠ RT 安全性の完全証明」の保証範囲の限定（RT ADR Verification 項目）

## Considered Options

**allocation 検出の標準:**

* `assert_no_alloc` crate に統一し、`ruviform-rt` の `alloc_counter` も置き換える
* 新規 lane には `assert_no_alloc` を採用し、`ruviform-rt` の `alloc_counter` は既存のまま維持する（二系統併存を許容）
* `alloc_counter` をワークスペース共有に昇格させ `assert_no_alloc` を使わない

**thread-check の適用:**

* `process` でホスト提供 `HostThreadCheck::is_audio_thread` を照会し、`Some(false)` を defined rejection（処理拒否）とする
* `is_audio_thread` 照会を `debug_assert` 相当の検査のみに留め、rejection は行わない
* ホスト側に thread-check を実装するのみで、プラグイン側は照会しない（片方向のみ）

## Decision Outcome

* **allocation 検出**: 新規 lane には `assert_no_alloc` crate を採用する。`ruviform-rt` の `alloc_counter` は std-only で外部依存を持たない実装であり、RT crate 側の検証目的（リタイア・スナップショットの非 alloc 証跡）は既に果たしているため、統一のための機械的書き換えは行わない。二系統併存を許容し、新規 RT 契約テストは `assert_no_alloc` を使う。`assert_no_alloc` がグローバルアロケータを差し替える crate である点と、`alloc_counter` が thread-local の `Cell` で同じ目的を果たす点を、検証文書で機構一覧として併記する
* **thread-check**: `clack-extensions` の `thread-check` feature を有効化し、プラグイン `process` でホスト提供の `HostThreadCheck::is_audio_thread` を照会する。応答 `Some(false)` は defined rejection（process の error status 返却）とし、`None`（ホスト非対応）は従来動作を維持する。テストホストは `HostThreadCheckImpl` を実装して is_audio_thread を応答し、両経路（許容・拒否）を ABI 通しで検証する。`debug_assert` 相当の検査のみ案は、誤スレッド呼出を本番環境で素通しするため不採用。片方向のみ案は「検査機構の統合」として不十分なため不採用

### Consequences

* Good: プラグイン境界で誤スレッド `process` 呼出を defined rejection として検出できる。allocation 検出が plugin-clap/plugin-vst3 の RT 契約テストにも実効化する。`alloc_counter` の書き換えを避けて二系統併存を許容することで、既存の rt 契約テストを壊さない
* Bad: `process` に 1 FFI call が追加される（軽量だが非ゼロ）。`assert_no_alloc` と `alloc_counter` の二系統併存は統一感に欠ける。`is_audio_thread` の応答に依存する検査はホスト非対応（`None`）環境では無検査に留まる

### Confirmation

下記 Verification のチェックリストで確認する。`assert_no_alloc` の実効性は意図的な alloc 検出テストまたは既存 `AllocDisabler` 実績で担保し、thread-check はテストホストの `HostThreadCheckImpl` 実装経由で両経路を ABI 通しで検証する。

## Implementation Plan

* 影響するパス:
  * `crates/ruviform-plugin-clap/Cargo.toml` — `clack-extensions` に `thread-check` feature 追加、`assert_no_alloc` を dev-dep 追加
  * `crates/ruviform-plugin-clap/src/plugin.rs` — `process` で `HostThreadCheck::is_audio_thread` 照会、`rt_process_*` テストを `assert_no_alloc` 下に
  * `crates/ruviform-plugin-vst3/Cargo.toml`・`crates/ruviform-plugin-vst3/src/block.rs` — `assert_no_alloc` 適用
  * `README.md` — 「CI 成功 ≠ RT 完全証明」「API 単位保証は実装責任」「OS ハード RT 保証なし」明記
  * `docs/` 系 — 検証文書・チェックボックス更新
* 従うパターン: `ruviform-audio-device` の `AllocDisabler`/`assert_no_alloc` 使用形、`ruviform-rt` の warm-up → 測定窓の構成。thread-check の rejection は既存 `process` の defined rejection 系（frames 超過等）と同じ error-status 返却経路
* 避けるパターン: `alloc_counter` を `assert_no_alloc` で機械置換する無差別統一、process 内での allocation を伴う extension 取得、`is_audio_thread` `Some(false)` を panic にする挙動（defined rejection として扱うのが契約）
* 依存関係: `clack-extensions` `thread-check` feature は既に pin 済み 0.2.0 で利用可能、`assert_no_alloc` は dev-dep としての追加（`ruviform-audio-device` 実績あり）

## Verification

* [ ] `plugin-clap`・`plugin-vst3` の RT 契約テストが allocation 検出下で実行され全件パス
* [ ] `process` が `is_audio_thread = Some(false)` で defined rejection、`None` で従来動作を返す ABI 通しの単体テスト
* [ ] `README.md`・設計文書に保証範囲の限定（CI 成功 ≠ 完全証明・API 単位保証は実装責任・OS ハード RT 保証なし）が明記されている
* [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス

## Pros and Cons of the Options

**allocation 検出 — `assert_no_alloc` 統一（alloc_counter 置き換え）:**

* 良い点: 単一機構で統一感・保守性が上がる。
* 悪い点: `ruviform-rt` の既存 RT 契約テストの機械的書き換えで diff が膨らみ、`alloc_counter` が std-only・外部依存なし・const 初期化という設計選択を失う。

**allocation 検出 — 新規 lane のみ `assert_no_alloc`、alloc_counter 併存:**

* 良い点: 既存テストを壊さない・diff が最小・`alloc_counter` の設計選択（std-only）を保持する。
* 悪い点: 二系統が併存し、「どちらを使うか」の基準を文書化する必要がある。

**allocation 検出 — `alloc_counter` ワークスペース昇格:**

* 良い点: 外部依存を増やさず統一できる。
* 悪い点: `alloc_counter` は crate-private の test helper として設計されており、公開・共有化のコストに見合わない。

**thread-check — `is_audio_thread` 照会 + defined rejection:**

* 良い点: 誤スレッド呼出を ABI 境界で確実に検出・拒否できる。ホスト非対応環境（`None`）では従来動作を維持する graceful degradation。
* 悪い点: `process` に 1 FFI call の追加コスト。ホスト非対応環境では検査が無効化される点を文書化する必要がある。

**thread-check — `debug_assert` 相当の検査のみ:**

* 良い点: 本番コードへの変更が最小。
* 悪い点: 誤スレッド呼出を本番環境で素通しし、「検出機構の統合」としての目的を果たさない。

**thread-check — ホスト側のみ実装:**

* 良い点: プラグインコードの変更が不要。
* 悪い点: プラグイン側がスレッド契約違反を能動的に検出しないため、「thread-check 統合」の要件を満たさない。

## More Information

* 一次ソース: GitHub Issue [#22 [chore] RT 検証基盤](https://github.com/scarecrow173/ruviform/issues/22)
* 上流 ADR: [リアルタイム契約と検証方針](realtime-contract-and-verification-policy.md) — 「検証可能な違反は CI で自動検出」の確定決定
* 設計ベースライン: `docs/designs/ruviform-architecture-design-baseline.md` 性能検証計画節（A.4）の `assert_no_alloc` 型検査・thread-check 型機構の指針
* `clack-extensions` 0.2.0: `thread-check` feature（`HostThreadCheck` host 拡張 + `HostThreadCheckImpl` trait）
* `alloc_counter`: `crates/ruviform-rt/src/alloc_counter.rs` — std-only counting `GlobalAlloc`（crate-private、test binary のみ）
