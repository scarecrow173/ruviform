---
id: "PLAN-7KK3Fpy2WfrNghFE2UJHF8"
type: "plan"
status: "completed"
title: "Issue #400 実施計画 — 借用 readback → demuxer bounded 接続"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/400"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/borrowed.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/tests/borrowed.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/borrowed_source.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-symphonia/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-symphonia/README.md"
      - type: "file"
        path: "crates/ruviform-media-io/src/demux.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/src/reader.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/src/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/reader.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/rt_fill.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/wav_staged.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/plans/README.md"
  implements:
    - "docs/specs/issue-400-borrowed-readback-decoder-bridge.md"
  implemented-by:
    - "docs/tasks/issue-400-t1-borrowed-readback-decoder-bridge.md"
  depends-on:
    - "docs/specs/issue-400-borrowed-readback-decoder-bridge.md"
    - "docs/designs/issue-400-borrowed-readback-decoder-bridge.md"
    - "docs/test-specs/issue-400-borrowed-readback-decoder-bridge.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-306-io-composition-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-400-borrowed-readback-decoder-bridge.md"
    - "docs/designs/issue-400-borrowed-readback-decoder-bridge.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-400-borrowed-readback-decoder-bridge.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/391"
  defers: []
  deferred-by: []
---

# Issue #400 実施計画 — 借用 readback → demuxer bounded 接続 Implementation Plan

## Goal

`SealedFile::open_readback` が貸す借用・`!Send` な `Read + Seek` を
`open_demuxer` の `impl MediaSource + 'static` 要求へ、追加メモリ
O(1) のまま接続する `with_borrowed_demuxer` を `ruviform-io-symphonia`
へ追加し、既存 E2E の encoded 全量 `Vec` copy を除去する。

## Scope

単一実施ストリーム。1 task（`docs/tasks/issue-400-t1-borrowed-readback-decoder-bridge.md`）。

## Upstream Documents

- `docs/specs/issue-400-borrowed-readback-decoder-bridge.md`
- `docs/test-specs/issue-400-borrowed-readback-decoder-bridge.md`

## Design Inputs

- `docs/designs/issue-400-borrowed-readback-decoder-bridge.md`

## File Map

- `crates/ruviform-io-symphonia/src/borrowed.rs` — `with_borrowed_demuxer`
  本体 + 内部 `ReadProxy` / `serve` ループ（channel 配管は private）。
- `crates/ruviform-io-symphonia/src/lib.rs` — `mod borrowed;` +
  `pub use borrowed::with_borrowed_demuxer;` + doctest。
- `crates/ruviform-io-symphonia/tests/borrowed.rs` — test-spec の
  Rules/Examples 実装。
- `crates/ruviform-media-io/src/demux.rs` — `impl<D: Demuxer + ?Sized>
  Demuxer for &mut D`（reborrow port）追加。
- `crates/ruviform-media-reader/src/reader.rs` — `SourceReader` →
  `SourceReader<'a>`（`demuxer: Box<dyn Demuxer + 'a>`、`open`/`reopen`
  の `Box<dyn Demuxer + 'a>` 化）。
- `crates/ruviform-media-reader/src/lib.rs` — doc の `SourceReader`
  記述を `'a` 対応へ微修正。
- `crates/ruviform-media-reader/src/prefetch.rs` — `SourceReader` 型位置を
  `SourceReader<'static>` / `&SourceReader<'_>` へ機械的注釈。
- `crates/ruviform-media-reader/tests/{reader,prefetch,rt_fill}.rs` —
  `-> SourceReader`/`&mut SourceReader` を `<'static>`/`<'_>` へ注釈。
- `crates/ruviform-media-reader/tests/borrowed_source.rs` — 新設:
  `with_borrowed_demuxer` + `SourceReader::open(Box::new(d))` で
  canonical `read_planar` が使える回帰（SV の consumer 形状）。
- `crates/ruviform-io-e2e/tests/wav_staged.rs` — `independent_decode` を
  `with_borrowed_demuxer` 経由へ差し替え（`Vec<u8>` 全量 copy を解消）。
- `crates/ruviform-io-symphonia/README.md` — borrowed entry の
  責務 1 段落 + doctest と同内容の例。
- `mise.toml` — 変更なし（crate 単位は `mise exec -- cargo test -p`
  既存入口を使用）。

## Implementation Sequence

- [ ] media-io: `impl<D: Demuxer + ?Sized> Demuxer for &mut D`（7
  メソッド forward、`stream` は default 利用）+ doc。
- [ ] media-reader: `SourceReader<'a>` 化（open/reopen の
  `Box<dyn Demuxer + 'a>` 化）+ prefetch/tests の `<'static>`/`<'_>`
  注釈。既存 test が無変更で通ることを確認。
- [ ] `borrowed.rs` 実装（`Req`/`Resp` enum、proxy、serve、scope join、
  panic 伝搬）。公開 API doc に RT/bound/escape 制約を明記。
- [ ] `lib.rs` へ re-export + doctest。
- [ ] `tests/borrowed.rs` に回帰 test（test-spec R-1..R-11 から
  実装可能な oracle を抽出）。
- [ ] `media-reader/tests/borrowed_source.rs` 新設: `f` 内
  `SourceReader::open(Box::new(d))` の canonical `read_planar` 回帰。
- [ ] `wav_staged.rs` の独立 decode を `with_borrowed_demuxer` 経由へ
  置き換え（`&mut readback` を直接渡す）。
- [ ] `mise run fmt` → `mise run check` → `mise run lint` →
  `mise exec -- cargo test -p ruviform-io-symphonia` →
  `mise exec -- cargo test -p ruviform-media-reader` →
  `mise run test-io-e2e` → `mise run test`。

## Dependencies and Risks

- `SymphoniaDemuxer` を `&mut dyn Demuxer` へ渡すため、consumer は
  具象型の inherent `make_decoder` ではなく trait の
  `make_decoder(&self, StreamId, DecodeMode) -> Box<dyn Decoder>` を
  使う（`Decoder` object は `Send`、source/proxy を保持しない）。
- `f` の `Send` 境界により `!Send` な外部参照（例: SV の `expect_pcm`
  callback）を `f` 内で直接呼べない。`f` 内は decode/pump のみに留め、
  `Send` でない後処理は scope 外に置く。
- `R` は `Send` 必須。`Vec<Vec<f32>>`・`Vec<Packet>`・byte 集約等は
  `Send` で問題ない。
- 巨大な `Read` buffer 要求は CHUNK 分割により短い逐語 read 列に
  変換されるため、`MediaSourceStream` 側の read-ahead 挙動は維持される
  （各短 read は `Ok(short)` を許容する contract 済み）。
- `SourceReader<'a>` は公開型の lifetime 追加 — 型位置で bare
  `SourceReader` を書く consumer は `<'static>`/`'_>` の機械的注釈が
  必要（SV 約 4 箇所。互換破壊として PR に明記し、式位置の利用は
  `'a='static` 推論で無変更）。`reopen` は `Box<dyn Demuxer + 'a>` を
  受け `'static` box も `'a` へ coerce されるため既存利用は無変更。
- `impl<D: Demuxer + ?Sized> Demuxer for &mut D` は公開 trait impl の
  追加 — `&mut D` が `Demuxer` を満たすことで `Box<dyn Demuxer + 'a>`
  が任意の reborrow から作れる。`'a` 束縛のみ生み、`'static` 所有や
  escape を新たに可能にしない（`&mut dyn` handout の escape 不能性を
  損なわない）。

## Task Breakdown

- `docs/tasks/issue-400-t1-borrowed-readback-decoder-bridge.md` —
  上記 Implementation Sequence 全体。

## Verification

- [ ] `mise run fmt-check` / `check` / `lint`
- [ ] `mise exec -- cargo test -p ruviform-io-symphonia`
- [ ] `mise exec -- cargo test -p ruviform-media-reader`
- [ ] `mise exec -- cargo test -p ruviform-media-io`
- [ ] `mise run test-io-e2e`
- [ ] `mise run test`（workspace）
- [ ] doctest 実行が `cargo test -p ruviform-io-symphonia --doc` に
  含まれること

## Open Questions / Gaps

- なし（`SourceReader` 対応は `<'a>` 一般化で確定）。
