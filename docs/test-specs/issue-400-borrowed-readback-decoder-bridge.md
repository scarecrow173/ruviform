---
id: "TSPEC-UiEoMhS5QMTr3jBiurbTtC"
type: "test-spec"
status: "approved"
title: "Issue #400 借用 readback → demuxer bounded 接続の test 仕様"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/400"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-symphonia/tests/borrowed.rs"
      - type: "file"
        path: "crates/ruviform-media-reader/tests/borrowed_source.rs"
    modified:
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
        path: "crates/ruviform-media-reader/tests/rt_fill.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/wav_staged.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/test-specs/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-400-borrowed-readback-decoder-bridge.md"
    - "docs/designs/issue-400-borrowed-readback-decoder-bridge.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/media-io-abstraction-and-backend-poc.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-400-borrowed-readback-decoder-bridge.md"
    - "docs/designs/issue-400-borrowed-readback-decoder-bridge.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 400
---

# Issue #400 借用 readback → demuxer bounded 接続の test 仕様

`with_borrowed_demuxer`（`ruviform-io-symphonia`）の回帰 test 群。
配置: `crates/ruviform-io-symphonia/tests/borrowed.rs`（unit/integration）
＋ `crates/ruviform-io-e2e/tests/wav_staged.rs` の既存独立 decode を
新 API 経由へ差し替え。

## Rules

- R-1（bounded 転送）: 巨大論理サイズの source（実メモリを持たない
  synthetic `Read + Seek`）から decode しても、in-flight encoded bytes
  は API 上限（request/response 各 1 件 × CHUNK）を超えない。実装上は
  読み出し回数・最大単発 `read` 要求長を観測して検証する。
- R-2（逐語 seek）: proxy 経由の `Seek` は underlying `seek` へ逐語
  forward され、戻り値は真の position を返す。proxy 側の position
  cache は存在しない（external seek との乖離を起こさない）。
- R-3（逐語 read）: short read / EOF（`Ok(0)`）/ `Interrupted` /
  任意 `io::ErrorKind` の一時 error は underlying の応答どおり伝搬する。
  API 側で retry・合成・黙殺しない。
- R-4（scope 完了保証）: `f` が `Ok` / `Err` / panic のいずれで終わっても
  呼び出しは戻る（serve が無限待ちしない）。panic は caller へ伝搬
  する（error へ潰さない）。unwrap で catch した場合に panic payload
  が元の panic と一致すること。
- R-5（escape 不能）: `f` に渡すのは `&mut dyn Demuxer`。compile 上、
  scope 外に所有値を持ち出せない（型検査で保証。test では型制約を
  `Send` 境界・`&mut dyn` 受け取りとして静的に検証）。
- R-6（早期 decoder error）: `f` が最初の `read_packet`/`decode` で
  `Err` を返しても、残り channel/scope は正常に閉じ、呼び出しはその
  `Err` をそのまま返す。serve は source の生 `Read`/`Seek` 側で
  無駄な read を待ち続けない。
- R-7（invalid source）: container として不成立な bytes を渡すと
  `open_demuxer` 由来の `Err`（`InvalidData` / `UnexpectedEos` / `Io`
  いずれか）が返り、test は panic/timeout しない。
- R-8（正常 WAV 独立 decode）: 既存 `wav_staged.rs` の oracle を維持
  する — `WavMuxer` → `StagedFile` → `seal` → `with_borrowed_demuxer`
  で sealed bytes を symphonia 独立 decode → byte-exact に原 PCM と
  一致 → `publish`。`publish` は readback 借用終了後にのみ可能。
- R-9（sealed 排他の維持）: `with_borrowed_demuxer` 呼び出し中は
  `SealedFile` への `&mut` 借用が生きており、型レベルで `publish`/
  `abort` は不可能。呼び出し終了後は `sealed` の消費操作が可能。
- R-10（`?Sized` 受け入れ）: `source` が `&mut impl Read + Seek +
  ?Sized` を受け、unsized な source 型でも呼べること。実用上の
  入力は `&mut ReadbackFile` と `&mut readback.reader()`。
  （`dyn ReadSeek` 自体は `Read`/`Seek` impl を持たないため
  直接は渡せない — 独立 review 確認済み。test では local trait
  object に明示 impl を持つ stub か `&mut ReadbackFile` を使う。）
- R-11（`f` の `Send` 境界）: `f` が `!Send` な値を捕捉しない形でのみ
  呼び出し可能であること（compile-time 契約。test 内では Send な
  closure のみを使用）。
- R-12（`SourceReader` 借用対応）: `f` 内で
  `SourceReader::open(Box::new(d), stream, DecodeMode::Canonical)` が
  構築でき、既存の `read_planar`/`next_frame` canonical decode が
  所有版と同一結果を返すこと（SV consumer 形状の回帰。
  `media-reader/tests/borrowed_source.rs` に配置）。
- R-13（reborrow port）: `impl Demuxer for &mut D` が `Box<dyn Demuxer
  + 'a>` を `'a` 束縛のまま生み、`seekable`/`make_decoder` を含む全
  メソッドが underling へ正直に forward されること（default で嘘を
  つかないこと）。

## Examples

- E-1: `BoundedSource` — `position: u64` / `len: u64` を内部保持し、
  `read` は `min(remaining, buf.len())` をゼロ埋めで返す。`len` を
  例えば `64 * 1024 * 1024` に設定し、`with_borrowed_demuxer` で WAV
  として解釈できない（先頭が RIFF でない）ことを oracle に、呼び出しが
  完結して `Err` を返すことを確認。同時に source 側に観測フック
  （呼ばれた `read` の `buf.len()` 最大値・呼出回数）を仕掛け、
  R-1 の bounded 検証に使う。巨大論理サイズの割に実 alloc は
  `CHUNK`（64 KiB）以下。
- E-2: `FailingSource` — `Cursor<Vec<u8>>` 相当の `Read + Seek` を
  wrap し、N 回目の `read`/`seek` で指定した `io::ErrorKind`
  （例: `PermissionDenied` や custom な一時 error）を 1 回返す。
  `f` 内で `read_packet` が該当位置まで進んだ時点で `Err` を観測し、
  API の戻り値が `Err(ErrorKind::Io)` であることを確認（R-3/R-6）。
- E-3: panic 伝搬 — `f` 内で `panic!("sentinel")` し、
  `catch_unwind(AssertUnwindSafe(...))` で捕捉した payload が
  `"sentinel"` を含むこと（R-4）。
- E-4: `InvalidSource` — 先頭が `RIFF`/`OggS`/`fLaC` 以外の 64 バイト
  を渡し `Err` が返ること（R-7）。
- E-5: `?Sized`/`!Send` 受け入れ — `Rc` を含む test-only source
  （`Read + Seek` impl 付き、`Send` 不可）を `&mut` で渡して
  呼べること（R-10 + `Send` 不要求の実証）。`&mut ReadbackFile`
  の直接受け入れは E2E 側で検証する。

## Oracles

- O-1: `with_borrowed_demuxer` の戻り値は `f` の `Result` と一致する
  （`Ok(r)`/`Err(e)`/`panic` の三枝）。
- O-2: `f` 内の `Demuxer` は `read_packet`/`seek`/`streams`/
  `make_decoder`/`seekable` が既存 trait どおり動く（実 WAV fixture
  で `read_packet` → `make_decoder` → `receive_frame` →
  `AudioFrame` の平面化 PCM が得られること）。
- O-3: E2E では `read_to_end` による `Vec` 全量 copy が残っていない
  こと（コード上の構造 oracle + 動作 oracle は byte-exact PCM 一致）。
- O-4: serve 終了 — `f` が早期 `Err` や panic で抜けた場合も
  `with_borrowed_demuxer` 呼び出しが有限時間で返ること（hang しない。
  test は panic catch / timeout なしで普通に assert する）。

## Non-goals

- decoded PCM の保持量は本 test-spec の oracle 対象外（Consumer 側の
  設計事項。追加 encoded bytes の bounded 性のみ検証する）。
- `SourceReader` を scope 内で合成する経路は存在しない（design の
  記録済み制約として非対象）。
- マルチスレッド同時 `with_borrowed_demuxer` 呼び出し（std::thread
  は並行可能だが同一 `source` 借用は排他で競合しないため、単一
  呼び出しの内部整合のみを検証する）。
