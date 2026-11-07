---
id: "TASK-rR5kzaPGjtdQepckCVSO1Z"
type: "task"
status: "done"
title: "Issue #400 T1: 借用 readback → demuxer bounded 接続の実装"
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
        path: "crates/ruviform-media-reader/src/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/reader.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/prefetch.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/wav_staged.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/tasks/README.md"
  implements:
    - "docs/plans/issue-400-borrowed-readback-decoder-bridge.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-400-borrowed-readback-decoder-bridge.md"
    - "docs/designs/issue-400-borrowed-readback-decoder-bridge.md"
    - "docs/test-specs/issue-400-borrowed-readback-decoder-bridge.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-306-io-composition-e2e.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-400-borrowed-readback-decoder-bridge.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 400
---

# Issue #400 T1: 借用 readback → demuxer bounded 接続の実装

## Work

`docs/designs/issue-400-borrowed-readback-decoder-bridge.md` の決定どおり
実装する。

(a) `crates/ruviform-media-io/src/demux.rs`: `impl<D: Demuxer + ?Sized>
Demuxer for &mut D` — `container`/`backend_info`/`streams`/`read_packet`/
`seek`/`seekable`/`make_decoder` を `(**self)` へ逐語 forward（`stream`
は default で正しく動く）。reborrow port として `'a` 束縛しか生まない
ことを doc に明記。

(b) `crates/ruviform-media-reader`: `SourceReader` → `SourceReader<'a>`
（`demuxer: Box<dyn Demuxer + 'a>`、`open`/`reopen` も `+ 'a`）。
`prefetch.rs` の所有 pipeline 側は `SourceReader<'static>` で固定、
tests は `<'static>`/`'_>` の機械的注釈。`lib.rs` doc を微修正。

(c) `crates/ruviform-io-symphonia/src/borrowed.rs` 新設:

- 公開: `with_borrowed_demuxer<R>(source: &mut (impl Read + Seek + ?Sized),
  f: impl FnOnce(&mut dyn Demuxer) -> Result<R> + Send) -> Result<R>`
  `where R: Send`。
- private 構造: `enum Req { Read { len: usize }, Seek(SeekFrom) }` /
  `enum Resp { Read(io::Result<Vec<u8>>), Seek(io::Result<u64>) }`、
  `struct ReadProxy { req: SyncSender<Req>, resp: Receiver<Resp> }`
  （`Read + Seek` impl で逐語 forward、`CHUNK = 64 * 1024`）、
  `fn serve<S: Read + Seek + ?Sized>(src: &mut S, req: Receiver<Req>,
  resp: SyncSender<Resp>)`（req disconnect まで回し、各 request は
  underlying へ 1 回だけ forward）。
- `std::thread::scope` で worker（`open_demuxer(proxy)` → `f`）を
  spawn し、呼び出し側で `serve` を回す。`worker.join()` で
  `Result<R>` を取り戻し、panic payload は `resume_unwind` で伝搬。

(d) `crates/ruviform-io-symphonia/src/lib.rs`: `mod borrowed;` +
`pub use borrowed::with_borrowed_demuxer;`。公開 API doc は doctest
で bounded 接続・`!Send` 借用受け入れ・escape 不能を示す。

(e) `crates/ruviform-io-symphonia/tests/borrowed.rs` 新設:
test-spec R-1..R-11 のうち実行可能な oracle を実装
（`BoundedSource`/`FailingSource` 等の test-only 小道具は同ファイル内
 private で持つ。公開 mock は追加しない）。

(f) `crates/ruviform-media-reader/tests/borrowed_source.rs` 新設:
`with_borrowed_demuxer` の `f` 内で `SourceReader::open(Box::new(d),
stream, DecodeMode::Canonical)` → `read_planar` が既存どおり動く回帰
（SV consumer 形状。`wav_bytes` fixture は `tests/reader.rs` と同じ
形式の最小 PCM WAV）。

(g) `crates/ruviform-io-e2e/tests/wav_staged.rs`: `independent_decode`
を `with_borrowed_demuxer(&mut readback, |demuxer| …)` 経由へ差し替え、
`read_to_end` 全量 `Vec` copy を除去（`Cursor` 版 oracle は
post-publish reopen 側に残してよいが、sealed readback 側は新 API）。

(h) `crates/ruviform-io-symphonia/README.md`: `with_borrowed_demuxer`
の 1 セクション追加。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-400-borrowed-readback-decoder-bridge.md`
- `docs/specs/issue-400-borrowed-readback-decoder-bridge.md`（AC-1..8）
- `docs/designs/issue-400-borrowed-readback-decoder-bridge.md`
- `docs/test-specs/issue-400-borrowed-readback-decoder-bridge.md`

## Dependencies

- なし（`ruviform-io-symphonia` 内で完結。`ruviform-media-io` の既存
  `Demuxer`/`Decoder`/`Error` API のみ利用）。

## Affected Files

- `crates/ruviform-io-symphonia/src/{borrowed,lib}.rs`
- `crates/ruviform-io-symphonia/tests/borrowed.rs`（新規）
- `crates/ruviform-io-symphonia/README.md`
- `crates/ruviform-media-io/src/demux.rs`
- `crates/ruviform-media-reader/src/{reader,prefetch}.rs`
- `crates/ruviform-media-reader/tests/{borrowed_source,reader,prefetch}.rs`
- `crates/ruviform-io-e2e/tests/wav_staged.rs`
- docs 索引（`docs/*/README.md` は `doc_maintenance apply` で再生成）

## Verification

- `mise run fmt-check` / `check` / `lint`
- `mise exec -- cargo test -p ruviform-io-symphonia`
- `mise exec -- cargo test -p ruviform-media-reader`
- `mise exec -- cargo test -p ruviform-media-io`
- `mise run test-io-e2e` / `mise run test`

実績（2026-11-07）: fmt-check/check/lint 全 pass。borrowed 回帰
11/11・borrowed_source 3/3・wav_staged 3/3・media-reader/media-io/
io-symphonia 全 test pass・compile_fail doctest 確認。`mise run test`
は既存の flaky な plugin-host プロセス境界 test
（`wedged_child_write_converges_and_teardown_is_bounded`）が
1 回時間超過したのみで単独再実行は pass — RF400 変更と無関係。
