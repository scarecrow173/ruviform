---
id: "TASK-034Jgi3OFauA1SWRPdvnnb"
type: "task"
status: "todo"
title: "ruviform-io-symphonia バックエンド PoC 実装"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/20"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-symphonia/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/demuxer.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/decoder.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/capability.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/tests/wav_decode_poc.rs"
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
    - "implement-media-io-abstraction.md"
  blocks:
    - "sync-docs-and-verify-issue-20.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/licensing-and-distribution-policy.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
    - "../specs/implement-media-io-abstraction-and-first-backend-poc.md"
    - "https://github.com/pdeljanov/Symphonia"
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

# ruviform-io-symphonia バックエンド PoC 実装

## Work

plan `PLAN-034JfKRCFCu8oJr5iCe6AV` の Phase 2。タスク 1 で確定した `ruviform-media-io` の trait・型を実装する第一バックエンド PoC `ruviform-io-symphonia` を、分離クレート + opt-in 依存として新設する（media-io ADR）。demux + decode のみを提供し、`Muxer` / `Encoder` 非提供を能力交渉「非対応」表現の実例とする。**PoC の目的は抽象境界の実証であり、個別 codec/container の正式採否は spec Out of Scope（後続 Issue）**。

- [ ] `crates/ruviform-io-symphonia/Cargo.toml` 新設: workspace 継承 + `[lints] workspace = true` + `ruviform-media-io` への `path` 依存（+ 必要に応じ `ruviform-media-core` / `ruviform-audio-core` / `ruviform-video-core` への直接 `path` 依存 — ベースライン L203 許容。media-io は domain 型を再エクスポートしない）+ `symphonia` 依存。**default features のみ**（royalty-free codec/format のみ）とし、`all` 等の特許対象 codec を含む feature は有効化しない。依存追加理由（discovery 評価「純 Rust decode 最有力・Lean=yes・`forbid(unsafe)`・trait 分離が抽象境界の好モデル」・MPL-2.0・MSRV 1.85 整合）と MPL-2.0（弱コピーレフト・ファイル単位）の調査結果を Cargo.toml コメントまたは PR 記録へ残す（暫定規約の依存追加規約・licensing ADR の per-dep 調査義務）。symphonia は最新 0.6.x を対象（0.5→0.6 API 変更・MSRV 1.85）。ルート `Cargo.toml` の `members` へ追加
- [ ] `src/demuxer.rs`: `Demuxer` 実装（symphonia `FormatReader` 系の wrap）。ストリーム列挙（`StreamDescriptor` + codec 記述型への写像）・パケット読み出し（`Packet` 変換: `StreamId`・PTS/DTS・フラグ・所有バイト列）・時刻指定シーク要求受け口。in-memory バイト供給用 `MediaSource` 経路を確立する（`MediaSource` trait は公開。既存 impl が合わなければ backend 内部実装として byte-backed `MediaSource` を持つ — 公開露出しない内部詳細）
- [ ] `src/decoder.rs`: `Decoder` 実装（symphonia 音声デコーダ wrap → 出力を planar f32 正準 `AudioBuffer` へ変換 → `AudioFrame` の PTS/duration 写像。symphonia timebase → `MediaTime` / `TimeBase` 変換）。flush はデコーダ内部状態をリセットする
- [ ] `src/capability.rs`（規模により `lib.rs` への併合は実装者裁量）: 能力情報（`Muxer` / `Encoder` を非対応と報告）
- [ ] 能力交渉の両機構: 照会 API が `Muxer` / `Encoder` 非対応を報告し、**かつ** `Muxer` / `Encoder` の取得要求が正規化 `Error` の非対応分類を返す（panic・`unimplemented!()`・暗黙的失敗による表現は禁止）
- [ ] エラー正規化: symphonia エラーを media-io `Error` 分類へ写像し、詳細は型消去済み `source` 診断へ封じる。公開 API に symphonia 型・エラー・feature 名を露出しない（参照実装としての型純度 — ベースライン L214 の「バックエンド内部 API は安定性契約対象外」とは別に型純度を維持）
- [ ] `tests/wav_decode_poc.rs`: **in-memory で WAV/音声バイト列を合成**（RIFF ヘッダ合成ヘルパーをテスト内に用意。**LFS フィクスチャ不使用**）→ demux → decode → `AudioFrame` の end-to-end（フレーム数・PTS 進行・サンプル値を検証）
- [ ] `src/lib.rs`: crate doc（symphonia による demux + decode のみの PoC・参照実装として型純度維持）+ 構築入口・能力照会の公開面。全公開 API に `**RT contract:** non-RT only` ラベル・全公開 enum に enum レベル `#[non_exhaustive]`・全公開 API に doctest/単体テスト
- [ ] テスト名に `rt_` 部分文字列を含めない（`assert_` / `start_` / `convert_` 等も誤選択されるため禁止 — `cargo test --workspace rt_` 部分文字列フィルタ対策）
- [ ] Phase 2 検証（下記 Verification 節）を実施し結果を記録する

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/issue-20-i-o-poc.md](../plans/issue-20-i-o-poc.md) — `PLAN-034JfKRCFCu8oJr5iCe6AV`（approved）。Phase 2（step 2-1〜2-7）・File Map・検証 V-2・Risk Register（symphonia 0.6 API churn・`MediaSource` 経路）の正本
- [../specs/implement-media-io-abstraction-and-first-backend-poc.md](../specs/implement-media-io-abstraction-and-first-backend-poc.md) — `SPEC-034JcaMXubILS7j1FX5qob`（approved）。Requirements（symphonia PoC・検証方法）・AC1/AC4/AC5/AC6 の正本
- [implement-media-io-abstraction.md](implement-media-io-abstraction.md) — `TASK-034JghH9i5p72LmOoWIPGe`。実装対象の抽象 API（`Demuxer` / `Decoder` / `Packet` / `AudioFrame` / `Error` / 能力情報型）を提供する前提タスク
- [../adr/media-io-backend-abstraction.md](../adr/media-io-backend-abstraction.md) — 分離クレート + opt-in・外部型非露出・実行時文字列選択なしの根拠
- [../adr/licensing-and-distribution-policy.md](../adr/licensing-and-distribution-policy.md) — symphonia MPL-2.0（弱コピーレフト・ファイル単位）・feature 制限・依存ごとのライセンス調査記録の正本
- [Symphonia](https://github.com/pdeljanov/Symphonia) — 第一バックエンド PoC の対象（discovery 評価の起点）

## Dependencies

- 前提タスク: [implement-media-io-abstraction.md](implement-media-io-abstraction.md)（`TASK-034JghH9i5p72LmOoWIPGe`）が完了し、抽象 API が確定していること — 本タスクはその trait/型を実装するため着手不可分
- 後続をブロック: [sync-docs-and-verify-issue-20.md](sync-docs-and-verify-issue-20.md)（総合検証対象・AC5/AC6 の検証対象を提供）
- 外部依存追加: `symphonia`（default features のみ・`ruviform-io-symphonia` の `[dependencies]` に限定。`ruviform-media-io` の `[dependencies]` にバックエンド依存を記述しない — AC4）。`unsafe` 不使用（局所許可なしで成立）

## Affected Files

- `crates/ruviform-io-symphonia/Cargo.toml` — 新規（workspace 継承 + media-io/domain への path 依存 + symphonia default features のみ）
- `crates/ruviform-io-symphonia/src/lib.rs` — 新規（crate doc + 構築入口・能力照会公開面）
- `crates/ruviform-io-symphonia/src/demuxer.rs` — 新規（`Demuxer` 実装 + in-memory `MediaSource` 経路）
- `crates/ruviform-io-symphonia/src/decoder.rs` — 新規（`Decoder` 実装 + flush）
- `crates/ruviform-io-symphonia/src/capability.rs` — 新規（能力情報。規模により `lib.rs` 併合可）
- `crates/ruviform-io-symphonia/tests/wav_decode_poc.rs` — 新規（in-memory WAV end-to-end PoC テスト）
- `Cargo.toml` — `members` へ `crates/ruviform-io-symphonia` 追加
- `Cargo.lock` — path/symphonia 依存記録（`cargo build` / `cargo generate-lockfile` で自動更新）

## Done When

- [ ] in-memory 合成 WAV の demux → decode → `AudioFrame` 変換 end-to-end が `cargo test -p ruviform-io-symphonia` で成功する（フレーム数・PTS 進行・サンプル値を検証。LFS 不使用 — AC5）
- [ ] 能力交渉の両機構が動作する: 照会 API が `Muxer` / `Encoder` 非対応を報告し、取得要求が正規化 `Error` の非対応分類を返す。panic / `unimplemented!()` でない（AC6）
- [ ] 公開 API に symphonia 型・エラー・feature 名が露出せず、エラー詳細は型消去 `source` 診断へ封じられている（AC1・型純度）
- [ ] symphonia 依存が `ruviform-io-symphonia` の `[dependencies]` に限定され default features のみであり、依存追加理由・MPL-2.0 調査が記録されている（AC4）
- [ ] 全公開 API に `**RT contract:** non-RT only`・全公開 enum に `#[non_exhaustive]`（AC9 うち本タスク範囲）
- [ ] 検証結果が記録され、実施記録 `docs/impl/ir/implement-io-symphonia-backend-poc.md` へ残せる状態（IR 作成・集約はタスク 3 で実施）

## Verification

- [ ] `mise exec -- cargo test -p ruviform-io-symphonia` — end-to-end を含む全テスト合格（AC5/AC6）
- [ ] `mise exec -- cargo clippy -p ruviform-io-symphonia --all-targets -- -D warnings` — 0 warnings
- [ ] `grep -rn "symphonia" crates/ruviform-media-io/src crates/ruviform-io-symphonia/src/lib.rs` — media-io 側 0 件・io-symphonia 公開面への露出なし（AC1）
- [ ] `grep -rn "pub use" crates/ruviform-io-symphonia/src` — 外部型の再エクスポートなし（AC1・型純度）
- [ ] `grep -rn "unimplemented!\|unreachable!\|panic!" crates/ruviform-io-symphonia/src` — 公開経路の panic 表現なし（AC6）
- [ ] `mise exec -- cargo tree -p ruviform-media-io` — symphonia 辺が media-io に存在しない（AC4）
