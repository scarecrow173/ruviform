---
id: "TASK-034JghH9i5p72LmOoWIPGe"
type: "task"
status: "todo"
title: "ruviform-media-io 抽象境界クレート実装"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/20"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-media-io/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-media-io/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/error.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/packet.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/codec.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/frame.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/capability.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/demux.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/mux.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/decode.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/encode.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/issue-20-i-o-poc.md"
  implemented-by: []
  depends-on:
    - "../plans/issue-20-i-o-poc.md"
  blocks:
    - "implement-io-symphonia-backend-poc.md"
    - "sync-docs-and-verify-issue-20.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
    - "../specs/implement-media-io-abstraction-and-first-backend-poc.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/issue-20-i-o-poc.md"
    - "../specs/implement-media-io-abstraction-and-first-backend-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/media-io-abstraction-and-backend-poc.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform-media-io 抽象境界クレート実装

## Work

plan `PLAN-034JfKRCFCu8oJr5iCe6AV` の Phase 1（直列クリティカルパス `media-io → io-symphonia → docs sync` の先頭）。spec `SPEC-034JcaMXubILS7j1FX5qob` の Requirements・media-io ADR を正本として、バックエンド実装を持たない抽象境界クレート `ruviform-media-io` を新設し、Issue #18 で確定したデータモデル型を入出力型として再利用する。

- [ ] `crates/ruviform-media-io/Cargo.toml` 新設: workspace 継承（version / edition / rust-version / license / publish）+ `[lints] workspace = true` + `ruviform-media-core` / `ruviform-audio-core` / `ruviform-video-core` への `path` 依存のみ（外部クレート依存ゼロ・ベースライン L198 の許容範囲）。ルート `Cargo.toml` の `members` へ追加
- [ ] `src/error.rs`: media-io 固有の `Error` / `Result`。正規化分類（I/O 失敗・非対応・破損/不正データ・ハード終端 EOS 等）＋ 型消去済みバックエンド診断（`std::error::Error::source` 経由。非 RT のため boxed source 可）＋ 上流値型 `Error`（`InvalidValue` / `TimeOverflow` 等）の正規化写像。正常終端（EOS）は `Err` ではなく `Option` / 専用終端表現で返す doc 規約を明記（`Error` の EOS 分類は異常系のみ）
- [ ] `src/packet.rs`: `Packet` 値型（`StreamId` によるストリーム参照・`MediaTime` PTS/DTS・キーフレーム等の符号化単位フラグ・所有バイト列）＋ `Send` 契約の doc 記載（I/O・デコードスレッド → 有界キュー越境前提。借用/zero-copy は spec Deferred・Issue #26 系）
- [ ] `src/codec.rs`: codec 記述型（Ruviform 管理の codec ID/名による識別・codec private data の opaque バイト列保持・`AudioFormat` / `VideoFormat` への写像）。AC11 の 3 要件を単体テストで固定。media-core の `StreamDescriptor` は codec 非依存のまま改変しない
- [ ] `src/frame.rs`: `AudioFrame`（`AudioBuffer` + `MediaTime` PTS + `Option<MediaDuration>` duration を束ねる専用名前付き型。`VideoFrame` との形状対称・タプル返却不採用）＋ `Send`
- [ ] `src/capability.rs`: 照会可能な能力情報型（`Error` の非対応分類と並ぶ二機構表現のうち照会側）
- [ ] `src/demux.rs` / `src/mux.rs` / `src/decode.rs` / `src/encode.rs`: `Demuxer`（ストリーム列挙 → `StreamDescriptor` + codec 記述型・パケット読み出し・時刻指定シーク要求 API）/ `Muxer` / `Decoder`（packet → frame・内部状態リセットのフラッシュ機構）/ `Encoder` の 4 系統 trait 分離（単一抽象への併合禁止）。seek/flush は受け口のみ提供し、再同期・提示順序・VFR 等の実行時振る舞いは engines 責務と doc 明記（ベースライン L239-L240）。`Muxer` / `Encoder` は実装者ゼロの未検証契約である旨を doc に記録（spec Deferred）
- [ ] `src/lib.rs`: crate doc（責務・許可依存層・非 RT 専用・RT パスからの呼び出しは契約違反）＋ 自クレート型のみのフラット再エクスポート（`AudioBuffer` / `VideoFrame` 等 domain 型の再エクスポートは行わない — `ruviform-io-*` が domain クレートへ直接 path 依存する余地を残す）。全公開 API に `**RT contract:** non-RT only` ラベル、クレート外到達可能な全公開 enum に enum レベル `#[non_exhaustive]`、全公開 API に doctest/単体テスト
- [ ] `Send` 契約のコンパイル時表明（`fn assert_send<T: Send>()`）を単体テストに含める。テスト名に `rt_` 部分文字列を含めない（`assert_` / `start_` / `convert_` 等も誤選択されるため禁止 — `cargo test --workspace rt_` 部分文字列フィルタ対策）
- [ ] Phase 1 検証（下記 Verification 節）を実施し結果を記録する

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/issue-20-i-o-poc.md](../plans/issue-20-i-o-poc.md) — `PLAN-034JfKRCFCu8oJr5iCe6AV`（approved）。Phase 1（step 1-1〜1-9）・File Map・検証 V-1 の正本
- [../specs/implement-media-io-abstraction-and-first-backend-poc.md](../specs/implement-media-io-abstraction-and-first-backend-poc.md) — `SPEC-034JcaMXubILS7j1FX5qob`（approved）。Requirements・AC1-AC4/AC7/AC9/AC11・Deferred Design Concerns の正本
- [../adr/media-io-backend-abstraction.md](../adr/media-io-backend-abstraction.md) — 抽象境界・外部型非露出・4 系統分離・Verification 節の根拠
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — 全公開 API `**RT contract:**` ラベル規則の正本
- [../adr/public-enum-non-exhaustive-policy.md](../adr/public-enum-non-exhaustive-policy.md) — 公開 enum `#[non_exhaustive]` ポリシーの正本
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。L198 依存許容・L263-L274「メディア I/O 抽象」節・L301 I/O・デコードスレッド（`Send` 契約の根拠）

## Dependencies

- 前提: plan `implement-issue-18-data-model.md`（completed）適用済み — 入出力型（`MediaTime` / `MediaDuration` / `TimeBase` / `StreamId` / `StreamKind` / `StreamDescriptor` / `AudioBuffer` / `AudioFormat` / `VideoFrame` / `VideoFormat`）は確定済みで非改変（AC7）
- 後続をブロック: [implement-io-symphonia-backend-poc.md](implement-io-symphonia-backend-poc.md)（本タスクの trait/型を実装するため抽象 API 確定が必須）、[sync-docs-and-verify-issue-20.md](sync-docs-and-verify-issue-20.md)（総合検証対象を提供）
- 外部クレート依存ゼロ・`unsafe` 不使用（局所許可なしで成立）

## Affected Files

- `crates/ruviform-media-io/Cargo.toml` — 新規（workspace 継承 + domain 3 クレートへの path 依存のみ）
- `crates/ruviform-media-io/src/lib.rs` — 新規（crate doc + フラット再エクスポート）
- `crates/ruviform-media-io/src/error.rs` — 新規（`Error` / `Result`・正規化分類・型消去診断）
- `crates/ruviform-media-io/src/packet.rs` — 新規（`Packet` 値型）
- `crates/ruviform-media-io/src/codec.rs` — 新規（codec 記述型）
- `crates/ruviform-media-io/src/frame.rs` — 新規（`AudioFrame`）
- `crates/ruviform-media-io/src/capability.rs` — 新規（能力情報型）
- `crates/ruviform-media-io/src/demux.rs` / `mux.rs` / `decode.rs` / `encode.rs` — 新規（4 系統 trait。モジュール分割は推奨であって、`traits.rs` 等への併合は実装者裁量）
- `Cargo.toml` — `members` へ `crates/ruviform-media-io` 追加 + メンバーコメント同期
- `Cargo.lock` — path 依存記録（`cargo build` / `cargo generate-lockfile` で自動更新）

## Done When

- [ ] spec Requirements の抽象境界要件（4 系統 trait 分離・`Packet`・codec 記述型・`AudioFrame`・エラーモデル・能力交渉照会側・seek/flush 受け口・`Send` 契約・型純度）が公開 API として存在し crate root から再エクスポートされる
- [ ] AC2（`cargo check -p ruviform-media-io` がバックエンド非依存で成功・`cargo tree` に `ruviform-io-*` 辺なし）・AC3（4 trait が別抽象として存在）・AC7 うち本タスク範囲（`AudioFrame` の PTS/duration・`Decoder` flush・`Demuxer` 時刻指定シーク API・audio-core 非改変）・AC11（codec 記述型 3 要件の単体テスト）を満たす
- [ ] 公開 API に外部クレート由来の型・エラー・feature 名が存在しない（AC1 うち本タスク範囲）
- [ ] 全公開 API に `**RT contract:** non-RT only`・全公開 enum に `#[non_exhaustive]`（AC9 うち本タスク範囲）
- [ ] 検証結果が記録され、実施記録 `docs/impl/ir/implement-media-io-abstraction.md` へ残せる状態（IR 作成・集約はタスク 3 で実施）

## Verification

- [ ] `mise exec -- cargo test -p ruviform-media-io` — 全単体テスト + doctest 合格
- [ ] `mise exec -- cargo clippy -p ruviform-media-io --all-targets -- -D warnings` — 0 warnings（`exhaustive_enums` ゲート含む）
- [ ] `mise exec -- cargo check -p ruviform-media-io` — バックエンド非依存で成功（AC2）
- [ ] `mise exec -- cargo tree -p ruviform-media-io` — 依存辺が `ruviform-media-core` / `ruviform-audio-core` / `ruviform-video-core` のみ（`ruviform-io-*` 辺・外部依存なし。AC2/AC4）
- [ ] `mise exec -- cargo doc -p ruviform-media-io --no-deps` — 公開 API レビュー（外部型非露出・4 trait 分離が TOC で確認。AC1/AC3）
- [ ] `grep -rn "pub enum" crates/ruviform-media-io` — 公開 enum の `#[non_exhaustive]` 監査（AC9）
- [ ] `grep -rn "unsafe" crates/ruviform-media-io` — コード内 0 件（AC8 関連）
