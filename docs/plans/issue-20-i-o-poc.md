---
id: "PLAN-034JfKRCFCu8oJr5iCe6AV"
type: "plan"
status: "completed"
title: "Issue #20 メディア I/O 抽象境界と第一バックエンド PoC"
created: "2026-10-20"
updated: "2026-10-21"
owners: ["scarecrow173"]
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
        path: "crates/ruviform-media-io/src/source.rs"
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
      - type: "file"
        path: "crates/ruviform-io-symphonia/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/demuxer.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/decoder.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/map.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/source.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/tests/wav_decode_poc.rs"
      - type: "file"
        path: "docs/impl/ir/implement-media-io-abstraction.md"
      - type: "file"
        path: "docs/impl/ir/implement-io-symphonia-backend-poc.md"
      - type: "file"
        path: "docs/impl/ir/sync-docs-and-verify-issue-20.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/designs/ruviform-architecture-design-baseline.md"
      - type: "file"
        path: "docs/specs/implement-media-io-abstraction-and-first-backend-poc.md"
      - type: "file"
        path: "docs/plans/issue-20-i-o-poc.md"
      - type: "file"
        path: "docs/tasks/implement-media-io-abstraction.md"
      - type: "file"
        path: "docs/tasks/implement-io-symphonia-backend-poc.md"
      - type: "file"
        path: "docs/tasks/sync-docs-and-verify-issue-20.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/media-io-abstraction-and-backend-poc.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/implement-media-io-abstraction-and-first-backend-poc.md"
  implemented-by:
    - "../tasks/implement-media-io-abstraction.md"
    - "../tasks/implement-io-symphonia-backend-poc.md"
    - "../tasks/sync-docs-and-verify-issue-20.md"
  depends-on:
    - "implement-issue-18-data-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../Cargo.toml"
    - "../../mise.toml"
    - "../adr/licensing-and-distribution-policy.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/implement-media-io-abstraction-and-first-backend-poc.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/media-io-abstraction-and-backend-poc.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #20 メディア I/O 抽象境界と第一バックエンド PoC Implementation Plan

## Goal

spec `SPEC-034JcaMXubILS7j1FX5qob`（approved）の受け入れ基準 AC1–AC11 を充足する: 抽象境界クレート `ruviform-media-io`（demux/mux と decode/encode を分離した 4 系統 trait・`Packet`・codec 記述型・`AudioFrame`・能力交渉・正規化エラーモデル・シーク/フラッシュ受け口）と、第一バックエンド PoC `ruviform-io-symphonia`（demux + decode のみ・`Muxer`/`Encoder` 非対応の実例）をワークスペースに追加し、in-memory 合成 WAV の demux → decode → `AudioFrame` 変換を end-to-end で検証する。

## Scope

単一の実装ストリームとして扱う。`ruviform-io-symphonia` は `ruviform-media-io` の trait 実装であり、抽象境界の定義とその実バックエンドによる実証は同一契約の両面であり、`media-io → io-symphonia → docs 同期` の直列クリティカルパス上にある。範囲外（spec Out of Scope と一致）: 個別 codec/container の正式採否・対応範囲拡大、動画 decode 実装（`VideoFrame` 出力経路は抽象契約まで）、Mux/Encode バックエンド実装、シーク・A/V 同期・VFR の実行時振る舞い（engines 責務）、フレームプール・zero-copy バッキングの具体方式、RT 経路での I/O、FFmpeg バックエンド、GUI/engine 統合。

## Upstream Documents

- [../specs/implement-media-io-abstraction-and-first-backend-poc.md](../specs/implement-media-io-abstraction-and-first-backend-poc.md) — `SPEC-034JcaMXubILS7j1FX5qob`（approved）。本 plan が実装する spec。Issue #20 範囲・要件・AC1–AC11・Deferred Design Concerns の正本
- [GitHub Issue #20](https://github.com/scarecrow173/ruviform/issues/20) — 一次ソース（`relations.source`）。設計ベースライン「後続実装 Issue の提案一覧」（L570 付近）の「メディア I/O 実装 Issue（抽象境界 + 第一バックエンド PoC）」から起票

## Design Inputs

- [../specs/implement-media-io-abstraction-and-first-backend-poc.md](../specs/implement-media-io-abstraction-and-first-backend-poc.md) — 全要件の正本。4 系統分離・`Packet` 値型・codec 記述型・`AudioFrame` 決定（タプル返却不採用・`VideoFrame` との形状対称）・エラーモデル（EOS は `Option`/終端表現、`Error` の EOS 分類は異常系限定）・能力交渉の両機構・型純度・`Send` スレッド契約・`rt_` 命名禁止・symphonia PoC 制約（default features のみ・分離クレート・型非露出）
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。
  - L198: `ruviform-media-io` の責務表定義（抽象境界・外部型非露出・非 RT 専用・依存は Primitives + Domain foundations）
  - L203: `ruviform-io-*` バックエンドクレート行（実装詳細・Domain foundations + Primitives 依存の許容範囲）
  - L214: 外部バックエンド = 分離クレート + opt-in。バックエンドクレート内部 API は安定性契約対象外（ただし spec により io-symphonia は参照実装として型純度を維持）
  - L239–L240: シークは engines の責務。media-io は demuxer への時刻指定シーク要求と decoder フラッシュの受け口のみ
  - L263–L274: 「メディア I/O 抽象」節 — 4 系統分離・単一 `ruviform-media-io` 集約・音声/動画共通仕組み採用・能力交渉・エラー正規化の決定根拠。**L265 は本実装との差異発生点**（`Decoder → AudioBuffer / VideoFrame` 記述 → `AudioFrame` へ同期更新対象、AC10）
  - L301: I/O・デコードスレッド（demux/decode はブロッキング可・有界キューで処理系へ供給）— `Send` 契約の根拠
  - L476–L477: 採否表（抽象境界は決定・個別 codec/container と symphonia 採否は要 PoC → 本 spec の Deferred）
- [../adr/media-io-backend-abstraction.md](../adr/media-io-backend-abstraction.md) — media-io ADR（accepted）。抽象境界の背後に全バックエンド配置・外部型非露出・純 Rust も同一境界経由・分離クレート + opt-in・実行時文字列指定の禁止・Verification 5 項目の正本（AC1–AC4 の対応元）
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — RT ADR（accepted）。「保証範囲は API 単位で定義・明示」→ 全公開 API の `**RT contract:** non-RT only` ラベル付け規則の根拠
- [../adr/licensing-and-distribution-policy.md](../adr/licensing-and-distribution-policy.md) — licensing ADR（accepted）。symphonia（MPL-2.0・弱コピーレフト・ファイル単位）を含む外部依存のライセンス個別調査・公開前レビュー必須・依存 feature 制限・`publish = false` 原則の根拠
- [../adr/public-enum-non-exhaustive-policy.md](../adr/public-enum-non-exhaustive-policy.md) — 公開 enum ADR（accepted）。新規 2 クレートのクレート外到達可能な全公開 enum に enum レベル `#[non_exhaustive]` を適用（`workspace.lints.clippy.exhaustive_enums = "warn"` + `-D warnings` ゲートで強制）
- 既存再利用型（Issue #18 確定・本 plan で改変しない前提とする公開 API）:
  - `ruviform-media-core`: `StreamId`・`StreamKind::{Audio,Video,Data}`・`StreamDescriptor`（id/kind/`TimeBase`/`Option<MediaDuration>`。codec 非依存が意図的 — `stream.rs` モジュール doc 明記・codec 記述型を media-io に分置する根拠）・`MediaTime`・`MediaDuration`・`TimeBase`・`MediaRange`・`Rounding`・`Error`/`Result`（`Copy` + `&'static str` payload の RT 適合パターン。media-io は非 RT 専用のため、型消去 `source` 診断を持つ別エラーモデルを新設する）
  - `ruviform-audio-core`: `AudioBuffer<S=f32>`（planar f32 正準・`BufferSpec`）・`AudioFormat`（`SampleRate`/`ChannelLayout`/`SampleFormat`）・`SampleRate`/`SampleIndex`。デコーダ出力は境界で planar f32 正準へ変換
  - `ruviform-video-core`: `VideoFrame`（`MediaTime` PTS + `Option<MediaDuration>` duration 内蔵・`Arc` backing）・`VideoFormat`・`PixelFormat`。`AudioFrame` 設計の形状対称の参照元

## File Map

### `crates/ruviform-media-io`（新規・抽象境界のみ。バックエンド実装を持たない）

- `Cargo.toml` — workspace 継承マニフェスト（`version`/`edition`/`rust-version`/`license`/`publish` = workspace、`[lints] workspace = true`）+ `ruviform-media-core` / `ruviform-audio-core` / `ruviform-video-core` への `path` 依存のみ（外部依存ゼロ・ベースライン L198 の許可範囲）
- `src/lib.rs` — crate doc（責務・許可依存層・非 RT 専用と RT パスからの呼び出し契約違反の明記）+ 自クレート型のフラット再エクスポート。**domain 型（`AudioBuffer`・`VideoFrame` 等）の `pub use` 再エクスポートは行わない**（spec Requirements。`ruviform-io-*` が domain クレートへ直接 `path` 依存できる余地を残す）
- `src/error.rs` — media-io 固有 `Error` / `Result`。正規化分類（I/O 失敗・非対応・破損/不正データ・ハード終端の EOS 等。正常終端は `Err` でなく `Option`/専用終端表現）＋バックエンド詳細の型消去診断（`std::error::Error::source` 経由）＋上流値型 `Error`（`InvalidValue`・`TimeOverflow` 等）の正規化写像。非 RT のため確保可（boxed source）
- `src/packet.rs` — `Packet` 値型（`StreamId` 等のストリーム参照・`MediaTime` PTS/DTS・キーフレーム等の符号化単位フラグ・所有バイト列。spec Deferred どおり所有バイト列とし、借用/zero-copy は後続設計）。`Send` でスレッド間移動可能
- `src/codec.rs` — codec 記述型。codec 識別（Ruviform 管理の codec ID/名）・codec 固有パラメータ（codec private data の opaque バイト列）・`AudioFormat`/`VideoFormat` への写像。`StreamDescriptor` は codec 非依存のまま改変しない
- `src/frame.rs` — `AudioFrame`（`AudioBuffer` + `MediaTime` PTS + `Option<MediaDuration>` duration を束ねる専用名前付き型）。`Send`。デコーダ出力の表現形（enum 分岐 vs ストリーム種別ごとの型付き受け口）は spec Deferred のとおり実装時に確定
- `src/capability.rs` — 能力交渉 API（照会可能な能力情報型。`Error` の非対応分類と併せる両機構表現の照会側）
- `src/demux.rs` / `src/mux.rs` / `src/decode.rs` / `src/encode.rs` — 4 系統 trait（`Demuxer`: ストリーム列挙→`StreamDescriptor` + codec 記述・packet 読み出し・時刻指定シーク要求 API / `Muxer` / `Decoder`: packet→frame・`flush` フラッシュ機構 / `Encoder`）。**モジュール分割は推奨案であり、`traits.rs` への集約等は実装者判断の余地を残す**
- 単体テスト（各モジュール `#[cfg(test)]`）+ doctest — 全公開 API にカバレッジ（`rt_` 部分文字列を含まない命名）

### `crates/ruviform-io-symphonia`（新規・第一バックエンド PoC。分離クレート + opt-in）

- `Cargo.toml` — workspace 継承マニフェスト + `ruviform-media-io` と必要に応じ domain クレートへの `path` 依存 + `symphonia`（default features のみ。`all` 等の特許対象 codec を含む feature は有効化しない）
- `src/lib.rs` — crate doc（symphonia による demux + decode のみ提供する PoC・参照実装としての型純度維持を明記）+ 構築入口・能力照会の公開面（symphonia 型・feature 名を一切露出しない）
- `src/demuxer.rs` — `Demuxer` 実装（symphonia `FormatReader` 系のラップ。in-memory 供給用の `MediaSource` 取り回しを含む）
- `src/decoder.rs` — `Decoder` 実装（symphonia 音声デコーダのラップ → planar f32 `AudioBuffer` 変換 → `AudioFrame` へ PTS/duration 写像）+ `flush`
- `src/capability.rs` — 能力情報（`Muxer`/`Encoder` 非対応報告）。規模次第で `lib.rs` への集約も可（実装者判断）
- `tests/wav_decode_poc.rs` — end-to-end PoC テスト。in-memory で WAV/音声バイト列を合成（LFS フィクスチャ不使用）→ demux → decode → `AudioFrame` 変換の一連実行を検証（音声フレーム数・PTS 進行・サンプル値）
- 単体テスト + doctest — 公開 API カバレッジ・`rt_` 命名禁止・`unimplemented!()`/panic による非対応表現の非存在

### 変更対象（既存）

- `Cargo.toml` — `members` に 2 クレート追加 + メンバーコメントの同期
- `Cargo.lock` — path 依存・symphonia 依存の記録（`cargo build`/`generate-lockfile` で自動更新）
- `README.md` — クレート一覧・実装状態記述の同期（2 クレート追加・media-io は非 RT 専用抽象境界・io-symphonia は PoC）
- `AGENTS.md` — ワークスペース構成段落の同期（メンバー追加・依存配線・非 RT 専用の記載）
- `docs/designs/ruviform-architecture-design-baseline.md` — L265 相当行の同期更新（`Decoder → AudioBuffer / VideoFrame` → `Decoder → AudioFrame / VideoFrame`。spec が決定した `AudioFrame` 専用型への差異解消、AC10）
- `docs/specs/implement-media-io-abstraction-and-first-backend-poc.md` — `relations.derived-by` に本 plan へのリンクを追加（doc グラフ双方向化）

## Implementation Sequence

### Phase 1: `ruviform-media-io` 抽象境界（task: implement-media-io-abstraction）

- [ ] 1-1. `Cargo.toml` 新設（workspace 継承・`[lints] workspace = true`）+ ルート `members` 追加 + domain 3 クレートへの `path` 依存配線（外部依存ゼロを維持）
- [ ] 1-2. `error.rs`: `Error`/`Result` 新設。正規化分類（I/O 失敗・非対応・破損/不正データ・ハード終端 EOS 等）+ 型消去 `source` 診断 + media-core 等の上流値型 `Error` の正規化写像（`From`/変換 fn）。正常終端は `Err` でなく `Option`/終端表現とする規約を doc に明記
- [ ] 1-3. `packet.rs`: `Packet` 値型（`StreamId`・`MediaTime` PTS/DTS・符号化単位フラグ・所有バイト列）+ `Send` 契約の doc 明記
- [ ] 1-4. `codec.rs`: codec 記述型（Ruviform 管理 codec ID/名・opaque codec private data・`AudioFormat`/`VideoFormat` 写像 fn。AC11 の 3 要件を単体テストで固定）
- [ ] 1-5. `frame.rs`: `AudioFrame`（`AudioBuffer` + `MediaTime` PTS + `Option<MediaDuration>` duration。`VideoFrame` と形状対称の名前付き型。`Send`）
- [ ] 1-6. `capability.rs`: 照会可能な能力情報型（`Error` 非対応分類と併せる両機構の照会側）
- [ ] 1-7. 4 系統 trait: `Demuxer`（ストリーム列挙・packet 読み出し・時刻指定シーク要求）/`Muxer`/`Decoder`（packet→frame・`flush`）/`Encoder` を別 trait として定義。シーク/フラッシュは受け口のみ（再同期・実行時振る舞いは engines 責務と doc 明記）。`Muxer`/`Encoder` は実装者ゼロの未検証契約として doc にその旨記録（spec Deferred）
- [ ] 1-8. `lib.rs`: crate doc（責務・許可依存層・非 RT 専用・RT パス呼び出しは契約違反）+ フラット再エクスポート（domain 型の再エクスポートなし）。全公開 API に `**RT contract:** non-RT only`・全公開 enum に enum レベル `#[non_exhaustive]`・全公開 API に doctest/単体テスト・テスト名に `rt_` 部分文字列を含めない
- [ ] 1-9. Phase 1 検証: `mise exec -- cargo test -p ruviform-media-io`・`mise exec -- cargo clippy -p ruviform-media-io --all-targets -- -D warnings`・`mise exec -- cargo check -p ruviform-media-io`・`mise exec -- cargo tree -p ruviform-media-io`（`ruviform-io-*` 辺・外部依存の非存在）

### Phase 2: `ruviform-io-symphonia` PoC（task: implement-io-symphonia-backend-poc。Phase 1 の API 確定に依存）

- [ ] 2-1. `Cargo.toml` 新設 + ルート `members` 追加 + `ruviform-media-io`（+ 必要な domain クレート）`path` 依存 + `symphonia` 依存（default features のみ）。依存追加理由（discovery 評価・MPL-2.0・MSRV 1.85 整合・`forbid(unsafe)`）と MPL-2.0 調査記録を PR に記録（暫定規約・licensing ADR）
- [ ] 2-2. `Demuxer` 実装: symphonia `FormatReader` 系をラップし、ストリーム列挙（`StreamDescriptor` + codec 記述への写像）・packet 読み出し（`Packet` 変換: `StreamId`・PTS/DTS・フラグ・バイト列）・シーク要求受け口を提供。in-memory バイト列供給の `MediaSource` 経路を確立
- [ ] 2-3. `Decoder` 実装: symphonia 音声デコーダをラップし、出力を planar f32 `AudioBuffer` へ変換 → `AudioFrame`（PTS/duration 写像。symphonia timebase → `MediaTime`/`TimeBase` 変換）。`flush` で内部状態リセット
- [ ] 2-4. 能力交渉: 能力照会 API が `Muxer`/`Encoder` を非対応と報告し、取得要求が正規化 `Error` の非対応分類を返す（両機構。panic/`unimplemented!()` 禁止）
- [ ] 2-5. エラー正規化: symphonia 由来エラーを media-io `Error` の分類へ写像し、詳細は型消去 `source` 診断に限定。公開 API に symphonia 型・エラー・feature 名を露出しない（参照実装としての型純度）
- [ ] 2-6. `tests/wav_decode_poc.rs`: in-memory WAV 合成 → demux → decode → `AudioFrame` の end-to-end 検証（フレーム数・PTS 進行・サンプル値。LFS 不使用）
- [ ] 2-7. Phase 2 検証: `mise exec -- cargo test -p ruviform-io-symphonia`・`mise exec -- cargo clippy -p ruviform-io-symphonia --all-targets -- -D warnings`・公開面 grep（`symphonia` 型名・`pub use` の非存在）

### Phase 3: ドキュメント同期・総合検証・impl record（task: sync-docs-and-verify-issue-20）

- [ ] 3-1. `Cargo.toml` メンバーコメント・`README.md`・`AGENTS.md` を実装済み状態へ同期（2 クレート・依存配線・非 RT 専用・PoC 位置付け）
- [ ] 3-2. 設計ベースライン L265 相当行を `AudioFrame` 記述へ同期更新（spec 決定差異の上流反映）
- [ ] 3-3. spec `derived-by` 更新 + 3 タスク doc・impl record（`docs/impl/ir/`）作成 + 本 plan の `changes`/status 同期
- [ ] 3-4. 総合検証スイート実行（Verification 節 V-1〜V-6 全件）
- [ ] 3-5. 独立レビューサイクル（Reviewer → Validator → Judge → 修正 wave）実施

## Dependencies and Risks

### Dependency Graph

| 依存元 | 依存先 | 内容 |
| --- | --- | --- |
| Phase 2（task 2） | Phase 1（task 1） | `ruviform-media-io` の trait・`Packet`・`AudioFrame`・`Error`・能力情報型が io-symphonia の実装対象。抽象 API 未確定では着手不可 |
| Phase 3（task 3） | Phase 1・2 | ドキュメント同期・総合検証は全実装完了後に実施（L265 同期は `AudioFrame` 確定が前提） |
| 本 plan | `implement-issue-18-data-model.md` | 入出力型（`MediaTime`/`MediaDuration`/`StreamDescriptor`/`StreamId`/`StreamKind`/`AudioBuffer`/`AudioFormat`/`VideoFrame`/`VideoFormat`）が確定済みであることを前提（spec `depends-on` の `implement-media-data-model-api.md` の実績側） |

- `ruviform-media-io` → `ruviform-media-core`/`ruviform-audio-core`/`ruviform-video-core` のみ（外部依存ゼロ・`ruviform-io-*` への依存辺なし。分離クレート構造で強制）
- `ruviform-io-symphonia` → `ruviform-media-io` + 必要に応じ domain クレート直接 `path` 依存（ベースライン L203 許容範囲）+ `symphonia`（default features のみ）。media-io は domain 型を再エクスポートしない
- `ruviform-math`/`ruviform-rt`/`ruviform-exec` への影響なし。audio-core/video-core/media-core は非改変（AC7）
- symphonia の `ruviform-media-io` への混入禁止: `[dependencies]` は io-symphonia に限定（AC4）

### Risk Register

| リスク | 影響 | 確率 | 対応 |
| --- | --- | --- | --- |
| symphonia 0.6 系の API churn | 0.5→0.6 で API が rework されており（音声専用→拡張可能基盤へ再設計）、今後の minor/patch でも PoC コードが追随コストを負う | 中 | 最新 0.6.x（MSRV 1.85 整合）を対象に実装し、バージョンを `Cargo.toml`/`Cargo.lock` で pin。symphonia API 接触面を io-symphonia 内部のラップ層に隔離し、抽象境界・公開面への波及を遮断 |
| MPL-2.0（弱コピーレフト・ファイル単位）のライセンス義務 | 依存追加規約・licensing ADR の個別調査義務。配布条件を未記録のまま取り込むと公開方針に未調査義務が残る | 中 | 調査記録（MPL-2.0・ファイル単位 copyleft・default features = royalty-free codec/format のみ・特許対象 codec feature 非有効化）を PR に記録。公開前レビューは licensing ADR どおり後続 Issue |
| in-memory WAV 合成と `MediaSource` 経路 | 正規の WAV バイト列合成・symphonia `MediaSource` の in-memory 取り回し（`Cursor` 系の実装有無）は実装依存 | 中 | symphonia `MediaSource` trait は公開のため、既存実装が適合しなければバイト列 backed の `MediaSource` をバックエンド内部に自前実装（内部詳細のため可・公開露出なし）。RIFF ヘッダ合成 helper をテスト側に用意 |
| `Packet`/`AudioFrame`/デコーダ出力の API 表現形の未確定点 | spec Deferred: デコーダ出力表現形（enum 分岐 vs 型付き受け口）・`Data` ストリーム packet 扱い・`Packet` フラグ集合・`Muxer`/`Encoder` 契約は実装者ゼロの未検証契約 | 中 | 過剰汎用化を避け PoC が通る最小契約で確定し、判断と根拠を doc/impl record に記録。`publish = false`・0.x 下での改訂余地を前提に API レビューで再確認 |
| 非 RT テスト名の `rt_` 部分文字列 | `cargo test --workspace rt_` の部分文字列フィルタが非 RT テストを誤選択（`assert_`/`start_`/`convert_` 等も `rt_` を内包） | 中 | 命名規約を実装バインドに明記し、完了条件に `mise run test-rt` のレーン監査（意図した既存 `rt_` テストのみ選択されること）を含める |
| `Send` スレッド契約の欠落 | `Packet`/`AudioFrame` は I/O・デコードスレッド→有界キューのスレッド境界をまたぐ前提（ベースライン L301）。`Send` 非実装だと engines 側設計が破綻 | 低 | データ型の `Send`（必要なら trait object も）を契約として doc 明記し、コンパイル時断言（`fn assert_send<T: Send>()` 相当）を単体テストで固定 |
| `AudioFrame` 変換コスト | symphonia 出力 → planar f32 `AudioBuffer` 変換で確保・コピーが発生 | 低 | PoC は正しさ優先（値返却まで。プール/zero-copy は spec Deferred・Issue #26 系） |
| 上流値型 `Error` との二重分類 | media-core `Error` を media-io `Error` へ写像する分類規則の一貫性 | 低 | 写像表を error.rs の doc に明記し単体テストで固定（`source` 経由の型消去診断に詳細を委譲） |
| データ損失 | なし（新規クレート追加のみ・既存ファイルの破壊的変更なし） | なし | — |
| 破壊的変更 | `publish = false`・workspace 内限定。既存 6 クレートの API は非改変 | 低 | `cargo tree`/`git diff` で非改変を実証（AC7） |

## Task Breakdown

| Task | 内容 | 対応 Phase |
| --- | --- | --- |
| [implement-media-io-abstraction.md](../tasks/implement-media-io-abstraction.md) | `ruviform-media-io` 抽象境界実装: `Error`/`Packet`/codec 記述型/`AudioFrame`/能力交渉/4 系統 trait・crate doc・RT ラベル・`#[non_exhaustive]`・path 依存配線 | 1 |
| [implement-io-symphonia-backend-poc.md](../tasks/implement-io-symphonia-backend-poc.md) | `ruviform-io-symphonia` PoC: symphonia による `Demuxer`+`Decoder` 実装・能力交渉「非対応」実例・エラー正規化・型純度・in-memory WAV end-to-end テスト・依存追加記録 | 2 |
| [sync-docs-and-verify-issue-20.md](../tasks/sync-docs-and-verify-issue-20.md) | ドキュメント同期（README/AGENTS/ルート `Cargo.toml`/ベースライン L265・spec `derived-by`）・総合検証スイート・独立レビューサイクル・impl record | 3 |

## Verification

### AC 写像（spec AC1–AC11 → 検証方法）

| AC | 内容 | 検証コマンド / レビュー観点 |
| --- | --- | --- |
| AC1 | 両クレート公開 API に外部クレート由来の型・エラー・feature 名なし | `mise exec -- cargo doc -p ruviform-media-io -p ruviform-io-symphonia --no-deps` の公開面レビュー + `grep -rn "symphonia" crates/ruviform-media-io/src crates/ruviform-io-symphonia/src/lib.rs`・`grep -rn "pub use" crates/ruviform-io-symphonia/src`（公開署名への外部型混入・再エクスポートの非存在）。io-symphonia 側は参照実装としての型純度を対象 |
| AC2 | media-io がバックエンド非依存でビルド・`ruviform-io-*` 依存辺なし | `mise exec -- cargo check -p ruviform-media-io`（単独成功）+ `mise exec -- cargo tree -p ruviform-media-io`（依存辺が media-core/audio-core/video-core のみ・外部依存・`ruviform-io-*` 辺の非存在） |
| AC3 | 4 系統 trait 分離 | API レビュー: `Demuxer`/`Muxer`/`Decoder`/`Encoder` が別 trait として `cargo doc` 目次に存在し、単一「読み出し」抽象への併合がない |
| AC4 | io-symphonia が分離クレート + opt-in・media-io `[dependencies]` にバックエンド依存なし | 両 `Cargo.toml` レビュー（`symphonia` は io-symphonia `[dependencies]` に限定）+ `mise exec -- cargo tree -p ruviform-media-io` で symphonia 辺の非存在 |
| AC5 | end-to-end PoC 成功 | `mise run test`（`cargo test -p ruviform-io-symphonia` を含む）— `tests/` 配下で in-memory 合成 WAV の demux→decode→`AudioFrame` がフレーム数・PTS 進行・サンプル値を検証し成功 |
| AC6 | 能力交渉の両機構 | 単体/統合テスト: 能力照会が `Muxer`/`Encoder` 非対応を報告 + 取得要求が正規化 `Error` 非対応分類を返す。`grep -rn "unimplemented!\|unreachable!\|panic!" crates/ruviform-io-symphonia/src` で公開経路の panic 表現がないこと |
| AC7 | `AudioFrame` PTS/duration・`AudioBuffer` 非改変・flush/シーク受け口 | `mise exec -- cargo test -p ruviform-media-io` + API レビュー（`AudioFrame` の `MediaTime` PTS/`Option<MediaDuration>`・`Decoder::flush`・`Demuxer` 時刻指定シーク API）+ `git status crates/ruviform-audio-core` が無改変 |
| AC8 | 検証スイート緑・`unsafe` 不使用 | `mise run fmt-check`・`mise run check`・`mise run lint`（`-D warnings` 下 0 warnings）・`mise run test`・`mise exec -- prek run --all-files` + `grep -rn "unsafe" crates/ruviform-media-io crates/ruviform-io-symphonia`（コード中 0 件） |
| AC9 | 全公開 API `**RT contract:** non-RT only`・全公開 enum `#[non_exhaustive]` | doc レビュー（両クレート全公開 API のラベル）+ `grep -rn "pub enum" crates/ruviform-media-io crates/ruviform-io-symphonia` で enum 監査 + `mise run lint` の `exhaustive_enums` ゲート通過 |
| AC10 | ドキュメント記述が実装状態と一致 | レビュー: `README.md`/`AGENTS.md`/ルート `Cargo.toml`/ベースライン L265 相当行（`AudioBuffer` → `AudioFrame`）が 2 クレート追加・依存配線・非 RT 専用の記載と一致 |
| AC11 | codec 記述型の 3 要件 | `mise exec -- cargo test -p ruviform-media-io` の単体テスト: codec 識別（Ruviform 管理 ID/名）・opaque codec private data 保持・`AudioFormat`/`VideoFormat` 写像を合成データで検証（PCM PoC で通らない経路のカバー） |

### 検証スイート（step ごとの検証入口）

| ID | 対象 | 検証コマンド / 確認 |
| --- | --- | --- |
| V-1 | Phase 1（media-io） | `mise exec -- cargo test -p ruviform-media-io`・`mise exec -- cargo clippy -p ruviform-media-io --all-targets -- -D warnings`・`mise exec -- cargo check -p ruviform-media-io`・`mise exec -- cargo tree -p ruviform-media-io`（AC2/AC4/AC7/AC11） |
| V-2 | Phase 2（io-symphonia） | `mise exec -- cargo test -p ruviform-io-symphonia`（end-to-end 含む）・`mise exec -- cargo clippy -p ruviform-io-symphonia --all-targets -- -D warnings`・公開面 grep（AC1/AC5/AC6） |
| V-3 | ワークスペース全体 | `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test`・`mise exec -- prek run --all-files`（AC8） |
| V-4 | RT レーン分離 | `mise run test-rt` — 新規非 RT テストが `rt_` 部分文字列フィルタに誤選択されないこと（命名規約の監査） |
| V-5 | bench/no-op 維持 | `mise run bench` — ベンチ未実装のため現状維持の入口確認 |
| V-6 | publish・依存構造 | `mise exec -- cargo metadata --format-version 1 --no-deps` — 全パッケージ `publish` が `[]`・新規 2 クレートの依存辺が設計どおり（media-io→domain 3、io-symphonia→media-io+domain） |

振る舞いレベルの保証（demux/decode の正確性・PTS 写像・能力交渉の両機構・エラー正規化分類・`AudioFrame` 形状契約）は [test spec](../test-specs/media-io-abstraction-and-backend-poc.md) に委譲し、ここでは step ごとの検証入口のみを管理する。

## Rollback Strategy

| 対象 | 戻し方 | 影響 | 備考 |
| --- | --- | --- | --- |
| 本変更全体 | `git revert` で対象 PR の commit を revert（squash マージなら merge commit `-m 1`） | 新規 2 クレート・ルート `members`・依存配線・ドキュメント記述が一括で巻き戻る | 追加のみの変更のため完全ロールバック可能。既存 6 クレートの API は非改変 |
| 部分的戻し | `crates/ruviform-io-symphonia` 単体削除 + `members` から除去 | media-io 抽象境界のみ残る（PoC 撤回のみ） | io-symphonia が media-io に依存する方向のため、逆順（media-io 単体削除）は不可 — io-symphonia の巻き戻しが先 |
| 部分的戻し | `crates/ruviform-media-io` 削除 + `members`・io-symphonia 側の巻き戻し | Issue #20 範囲全体の撤去 | io-symphonia の path 依存が壊れるため両方を戻す必要あり |

## Open Questions / Gaps

- **デコーダ出力の表現形**: enum 分岐（`enum Frame { Audio(AudioFrame), Video(VideoFrame), … }`）vs ストリーム種別ごとの型付き受け口は spec Deferred どおり実装時に確定する。`StreamKind::Data` ストリームの packet 扱い（decode 対象外としてどう表現するか）も併せて確定。抽象肥大化と表現不足のいずれにも過剰適合しない最小契約を選ぶ
- **`Packet` の細部**: フラグ集合（キーフレーム以外に何を持つか）・DTS の `Option` 性・`StreamId` 以外のストリーム参照の要否は実装時に確定。所有バイト列（`Vec<u8>` 等）は spec Deferred で決定済み（借用/zero-copy は後続設計・Issue #26 系と併せ）
- **codec ID/名の管理方法**: Ruviform 管理の codec 識別子をどう名付け・登録するか（enum 風集合 + 拡張名の併用等）は codec 記述型の設計で確定
- **`Muxer`/`Encoder` の契約実証なし**: 実装者ゼロの未検証契約（spec Deferred）。第一 encode 対応バックエンド Issue で実証し、`publish = false`・0.x 下で改訂可能
- **ADR Verification 第 3 項（複数バックエンド間の同一境界・透過切替）**: 第一バックエンドのみでは検証不能（spec Deferred）。第 2 バックエンド実装 Issue で検証
- **gapless 情報（encoder delay/padding trim）の運搬先**: 本 spec では未定（codec 記述型・`AudioFrame`・能力交渉のいずれに載せるかは後続論点）
- **spec Deferred Design Concerns の据え置き**: 個別 codec/container 採否・Mux/Encode バックエンド・動画 decode・FFmpeg・シーク/A-V 同期/VFR の実行時振る舞い・フレームプール/zero-copy 具体方式・RT 経路 I/O は全て後続 Issue（本 plan の Open Questions に含まれる残課題を除き変更不要）

## Self-Review

- [x] spec の Requirements・受け入れ基準 AC1–AC11 が Phase 1–3 と V-1〜V-6、3 タスクに対応している
- [x] spec Deferred Design Concerns の全論点が Scope / Open Questions に反映され、据え置き項目が実装対象に混入していない
- [x] media-io ADR の Verification 第 1・2・4・5 項が AC1–AC4 の検証方法に写像されている（第 3 項は spec Deferred どおり据え置き）
- [x] design の決定事項（4 系統分離・単一 media-io 集約・外部型非露出・分離クレート + opt-in・非 RT 専用・シーク/フラッシュの責務限定）と RT ADR の API 単位ラベル規則・enum ADR・licensing ADR の依存調査規約が実装・検証に反映されている
- [x] TBD / TODO / 「後で実装」のプレースホルダは残っていない（据え置き項目は spec Deferred と Open Questions / Gaps に記録）
- [x] ファイルパス・relation・status・用語が spec/design/タスク命名と整合している（`changes` と File Map が一致）

### AC 対応表

| AC | 対応 Phase | 対応タスク | 検証 ID |
| --- | --- | --- | --- |
| AC1 | 1-5/2-5 | task1・task2 | V-1・V-2 |
| AC2 | 1-1 | task1 | V-1 |
| AC3 | 1-7 | task1 | V-1 |
| AC4 | 1-1/2-1 | task1・task2 | V-1・V-6 |
| AC5 | 2-6 | task2 | V-2 |
| AC6 | 1-6/2-4 | task1・task2 | V-2 |
| AC7 | 1-5/1-7 | task1 | V-1 |
| AC8 | 1-8/2-7/3-4 | task1・task2・task3 | V-3 |
| AC9 | 1-8/2-5 | task1・task2 | V-3 |
| AC10 | 3-1/3-2 | task3 | V-3・レビュー |
| AC11 | 1-4 | task1 | V-1 |

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation（task1 → task2 は API 契約依存で直列。task2 の symphonia 実装は task1 の trait 確定後に独立作業可能。task3 は全実装完了後の直列フェーズ）
- Independent work streams: なし（task1 → task2 → task3 の直列依存。task1 内部の error/packet/codec/frame/capability/trait 各モジュールは同一クレート内のため同一作業者が扱うのが安全）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提（`relations.depends-on`）: [implement-issue-18-data-model.md](implement-issue-18-data-model.md)（`PLAN-034J5K603qXDNmT82wI9v7`・completed）で確定したデータモデル API が本変更の入出力型。media-core/audio-core/video-core は非改変が前提（AC7）
- 明示的な仮定: `ruviform-media-io` の公開 API は Ruviform 固有型 + std 型のみで構成（symphonia 等の外部型・feature 名を露出しない）。`Packet`/`AudioFrame` 等のデータ型は `Send` でスレッド境界をまたぐ（ベースライン L301）。`ruviform-media-io` は domain 型を再エクスポートしない
- 未確定点（実装時に確定する設計余地）: デコーダ出力の表現形（enum 分岐 vs 型付き受け口）・`Data` ストリームの packet 扱い・`Packet` フラグ集合・codec ID/名の管理方式・`Muxer`/`Encoder` 契約の詳細 — レビュアーは過剰汎用化・表現不足の両方向から判定すること
- レビュー観点の重点: 型純度（外部型の `pub` 露出・再エクスポートの非存在）・能力交渉の両機構（照会 + `Error` 分類）・EOS/終端表現（`Err` でなく `Option`）・`rt_` 命名・`#[non_exhaustive]` 網羅・`**RT contract:**` ラベル網羅・ベースライン L265 同期
- 上流参照: spec `SPEC-034JcaMXubILS7j1FX5qob`、design `DESIGN-034IPynWe2V30FLubzQenb`、media-io ADR `ADR-034INZsD434t5cQ7gBp7XN`、RT ADR `ADR-034IO00JjhxeTXUbCZtcDX`、licensing ADR `ADR-034IO02s3n3KHsxnHckpRn`、enum ADR、Issue #20・親 Issue #1
