---
id: "TSPEC-icPRYUQjj1ahQHHkGcK8vZ"
type: "test-spec"
status: "approved"
title: "Issue #285 IO-03 Compression 契約試験棚卸し テスト仕様"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "docs/specs/issue-285-io-03-compression-contract-audit.md"
    - "https://github.com/scarecrow173/ruviform/issues/285"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-285-io-03-compression-contract-audit.md"
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
    - "docs/test-specs/issue-285-io-03-compression-serde-boundary.md"
    - "docs/test-specs/issue-300-io-compression-gzip-deflate.md"
    - "docs/test-specs/issue-301-io-compression-zstd.md"
  refines:
    - "docs/test-specs/issue-285-io-03-compression-serde-boundary.md"
  refined-by: []
  derives-from:
    - "docs/designs/issue-285-io-03-compression-contract-audit.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-285-io-03-compression-contract-audit.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #285 IO-03 Compression 契約試験棚卸し テスト仕様

## Purpose

Issue #285 受入条件の「完全入力/途中 EOF/破損/Checksum/過大
展開/Cancel/Writer Error の契約と試験」「ENC-01〜03・
DEC-01〜03・SEC-01〜02」のうち、本 slice が追加した headless
契約試験が保証すべき契約を定める。per-format の wire 形式・
framing 内部は #300/#301 の test-spec が正本であり、本 spec は
横断契約（encoder/decoder 状態機械・failure-path・budget・
所有権）のみを対象とする。実装方法は定めない。

## Feature

(a) Encoder の構築・write・flush・finish・Drop・失敗 latch が
設計正本の状態機械どおりに外部観測できる。(b) Decoder の
verified-end・`NotExhausted`・source I/O 失敗正規化・連結単位
境界・累積 budget が設計正本どおりに外部観測できる。
(c) いずれの試験も feature gate を維持し `default = []` を
壊さない。

## Rules

### Encoder（ENC 系）

- R1（header/構築時 I/O 失敗）: `Encoder::new` 内での固定
  header 書込（gzip）が失敗した場合、構築は `Io` kind で失敗し、
  header は output budget を超えて emit されない（部分 header
  を publish しない）。底流 writer は呼出側に残る。
- R2（flush ≠ finish）: `flush()` は pending byte を下流へ
  伝搬するが完了証明ではなく、途中 flush を挟んだ
  write→flush→write→finish が有効な member/frame を生成し
  decode 可能である。`Finished` 後の `flush()` は no-op で
  `Ok` を返す。
- R3（finish 末尾の底流 flush 失敗）: `finish()` が内部で呼ぶ
  底流 `flush()` の失敗は encoder の `Io` 失敗として latch
  し、以後の `finish()`/`write()` は `InvalidState` を返す。
- R4（terminal 失敗後の Drop）: `Failed` 状態の encoder を
  drop しても trailer/final block/frame epilogue 等の追加
  byte を底流へ書かない。sink に残るのは失敗前に受理された
  prefix のみであり、その部分出力は decoder の検証済み完了を
  通らない。
- R5（失敗後の底流再利用）: encoder が terminal `Failed` に
  latch した後、encoder を drop すれば底流 writer は呼出側で
  無傷に再利用できる。部分出力 byte は呼出側所有のまま残り、
  publish-or-discard は呼出側の責務である。
- R6（連続 encoder の連結）: 同一 writer 上で `finish()` 済み
  encoder の後に新しい encoder を構築すると、有効な連結
  member（gzip）/frame（zstd）stream が生成され、decoder は
  `WholeInput` で累積 units を receipt に計上する。
- R7（Finished 後の write 拒否と finish 冪等）: `finish()`
  成功後の `write()` は `InvalidState` を返し、`finish()` の
  再呼出は同一 receipt を返す（冪等）。

### Decoder（DEC/SEC/BUF 系）

- R8（verified-end finish の冪等）: 検証済み完了後の
  `finish()` は receipt を返し、繰返し呼んでも同一 receipt
  を返す。
- R9（SingleUnit 境界の独立検証）: `DecodeScope::SingleUnit`
  で 1 単位を decode・verified-end した後、底流 `BufRead` に
  残った次単位 byte を新しい decoder が独立に decode・
  verified-end できる（member/frame/stream を跨いで
  receipt が混ざらない）。
- R10（source I/O 失敗の正規化と latch）: decoder の底流
  `BufRead` が mid-stream で I/O エラーを返した場合、error
  は `Io` kind に正規化され、decoder は terminal `Failed`
  に latch して以後の `read`/`finish` を `InvalidState`
  で拒否する。
- R11（高圧縮率 payload の decoded budget）: 実際に高圧縮率
  である payload（~1 MiB zeros → ~1/100 以下の compressed
  byte）に対し、`max_decoded_bytes` が payload ちょうどでは
  成功・1 byte 不足では `LimitExceeded` で、cap までの byte
  は配送される。
- R12（`NotExhausted` の非破壊性）: 未 drain の出力を残した
  `finish()` は `NotExhausted` を返すが decoder 状態を壊さず、
  その後の drain で verified-end に到達できる。

## 既存試験との棲み分け

- wire 形式の正当性（RFC 1951/1952/8878 framing・checksum・
  skippable frame・window）: `tests/{gzip,deflate,zstd}_format.rs`
  と #300/#301 test-spec が正本。本 spec は形式内部を重複
  検証しない。
- backend interop（参照実装との相互運用）・公開 API 表面監査:
  `tests/api_surface.rs`・`tests/composition.rs` と既存
  test-spec が正本。
- 合成 E2E（TAR→compression→StagedFile・fault・fuzz）:
  `ruviform-io-e2e`（#306）と `docs/test-specs/issue-306-*`
  が正本。本 spec は単体 crate の横断契約のみ。

## NotRun / Unsupported（本 spec の対象外）

- 性能・ベンチマーク・長時間 Fuzz campaign — headless 契約
  試験の範囲外。後続: #300/#301 設計の性能節・#288 性能 gate。
- 両 Consumer（AudioLens #139・Synvelis #82/#84）実 repo での
  Round-trip/保存結果回帰 — consumer-verify（#219/#294）と
  #288 gate の責務。
- macOS/Linux 実機 `zstd` vendored build・CI OS matrix —
  headless Windows のみ。後続: CI 既存 OS lane。
- `Cancelled` kind の上位 Job 連携・実キャンセル配線 —
  上位 Job/Consumer 実装時（設計正本の Deferred と同一）。
