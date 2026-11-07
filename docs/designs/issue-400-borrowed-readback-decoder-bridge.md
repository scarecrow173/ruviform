---
id: "DESIGN-utKDN0iwqlQEVtqkiEUHJi"
type: "design"
status: "approved"
title: "Issue #400 借用 sealed readback → 独立 decoder bounded 接続の設計"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "docs/specs/issue-400-borrowed-readback-decoder-bridge.md"
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
        path: "docs/designs/README.md"
  implements:
    - "docs/specs/issue-400-borrowed-readback-decoder-bridge.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-400-borrowed-readback-decoder-bridge.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/391"
    - "https://github.com/scarecrow173/synvelis/pull/181"
  defers: []
  deferred-by: []
metadata:
  issue: 400
  spec: "SPEC-0kIy1SFw7ffSK1AAi4BXHu"
---

# Issue #400 借用 sealed readback → 独立 decoder bounded 接続の設計

## 背景と制約

- `SealedFile::open_readback(&mut self)` が返す `ReadbackFile<'a>` は
  `&'a mut dyn ReadSeek` の薄い借用で `Read + Seek`・`!Send`・非 clone・
  handle/path なし（`ruviform-io-file::staged`）。
- `ruviform_io_symphonia::open_demuxer(source: impl MediaSource + 'static)`
  は `Read + Seek + Send + 'static` を要求し、`SourceAdapter`（
  `Box<dyn MediaSource>`）→ `MediaSourceStream` で所有する。
- `MediaSource` port（`ruviform-media-io`）は `Read + Seek + Send`。
  backend（`io-symphonia`）が `'static` を要求するのは `MediaSourceStream`
  の所有構造に由来する。
- `SourceReader::open(demuxer: Box<dyn Demuxer>, ...)` は所有 demuxer を
  要求するため、scope 制約下の `&mut dyn Demuxer` をそのまま入れられない。

## 決定（候補比較は Appendix）

公開関数を `ruviform-io-symphonia` に 1 つだけ追加する。シグネチャ:

```rust
pub fn with_borrowed_demuxer<R>(
    source: &mut (impl std::io::Read + std::io::Seek + ?Sized),
    f: impl FnOnce(&mut dyn ruviform_media_io::Demuxer)
        -> ruviform_media_io::Result<R> + Send,
) -> ruviform_media_io::Result<R>
where
    R: Send,
```

### 内部構造

- `std::thread::scope` で scoped worker を 1 本立てる。
- 呼び出し側スレッドが `serve` ループを回し、`&mut *source` を直接保持
  する（`!Send` な借用は thread をまたがない）。
- worker 側の `ReadProxy { req: SyncSender<Req>, resp: Receiver<Resp> }`
  は `Send + 'static`（中身は channel endpoint のみ）。
- `sync_channel(1)` を request/response の両方向に使う。in-flight は
  request 1 件 + response 1 件まで。
- worker は `open_demuxer(proxy)` を呼び、`f(&mut demuxer)` を実行して
  `worker.join()` の戻り値を親へ返す。

### プロトコル: 逐語 forward

- `Req::Read { len }` → serve が `src.read(&mut vec![0; len])` を 1 回
  実行し、得た `io::Result<Vec<u8>>`（実読バイトで truncate）を
  `Resp::Read` で返す。`len` は `min(buf.len(), CHUNK)`、CHUNK=64 KiB。
- `Req::Seek(SeekFrom)` → serve が `src.seek(pos)` をそのまま実行し
  `io::Result<u64>` を返す。
- proxy は自分で position cache や len probe を持たない。`is_seekable` /
  `byte_len` は `SourceAdapter` 側の `stream_position`→`seek(End(0))`→
  `seek(Start(saved))` round-trip が proxy 越しにそのまま動く。

### 終了・panic 契約

- serve ループは request channel が disconnect（proxy 側 sender 全滅）
  した時点で終了する。`f` が `Ok`/`Err`/panic いずれで抜けても proxy
  は worker 内で drop され request channel が切れる。終了は `f` の
  正直さではなく worker 終了にのみ依存する（`mem::forget` 等でも
  pointee は worker 内で drop）。
- worker panic → `worker.join()` が `Err` → 呼び出し側で
  `resume_unwind` により panic を伝搬（error に潰さない。
  `ErrorKind` には panic を正直に表す kind が存在せず、捏造しない）。
- serve 側（source の read/seek が panic） → unwind で channel endpoint
  が drop され、worker の send/recv が失敗して worker も終了 →
  `thread::scope` が join → panic は呼び出し側へ伝搬。二重 panic
  abort は起きない。
- **channel disconnect は `io::Error`（例 `BrokenPipe`）へ写像し、
  `Ok(0)`/clean EOF にはしない**（serve 死亡を正常 EOF として報告
  すると decode が中途半端な成功に見え、真の失敗を隠す）。

### escape 防止

- `f` に渡すのは `&mut dyn Demuxer`（HRTB で scoped lifetime）。
  `Box<dyn Demuxer>` 化も `mem::replace`/`mem::swap` による concrete
  値の差し替えも不可能（`mem::*` は `Sized` を要求し `dyn` は
  unsized）。**`&mut SymphoniaDemuxer` や `&mut Box<dyn Demuxer>`
  では `f` が `open_demuxer(Cursor)` 等で偽 source を作り
  `mem::replace` で proxy 保持 object を盗み出せる（serve が永続
  待ち → 実質永久 deadlock）ため、`&mut dyn` が唯一安全な handout**
  （独立 review 確認済み）。
- `R` は `for<'a>` の外側で束縛されるため `'a` を含められず、
  `f` から `&mut` 由来の値を return で漏らせない。`R: Send` は
  join 境界で必要だが `'static` ではない（caller 側 `'env` 借用を
  含んでよい）。
- scope-bound は **borrow のみ**。`make_decoder` が返す
  `Box<dyn Decoder>`（`Send`・source/proxy を保持しない）、
  `Packet`/`Frame`/`StreamInfo` 等の所有値は合法に `R` へ出せる。
- `f` は `Send` 要求（worker へ move）。serve を worker 側に回す
  配置は `&mut S: Send` を要求するため `!Send` 借用では成立せず、
  現配置が唯一の構成。
- **維持すべき不変条件（将来の trait 拡張への制約）**: escape 防止は
  `Demuxer` trait が downcast port（`as_any`/`as_any_mut`/`Any`
  supertrait）や `Box<dyn Demuxer>`/concrete を返す factory を
  持たないことに依存する。これらが追加されると `&mut dyn` の
  opacity が破れて deadlock 可能になるため、rustdoc と trait 側の
  コメントで明記する。
- proxy の `req` sender は clone しない（唯一の sender が demuxer
  内に留まることが serve 終了保証）。

## 非選択案

1. **`&mut SymphoniaDemuxer`（concrete）port** — `mem::replace` で
   `open_demuxer(Cursor::new(vec![]))` 等の偽 source を差し込めば、
   proxy 保持 object を `f` の外へ escape させ得る。`&mut dyn` で
   object-safety 境界により型上排除する。
2. **reader 級 `with_reader`** — `SourceReader::open(Box<dyn Demuxer>)`
   が所有を要求するため、`&mut dyn Demuxer` では合成不可。reader 側で
   独自に同配管を持つと責務分散・重複になる（別 Issue 候補）。
3. **proxy を `impl MediaSource` で直接公開** — Consumer が proxy を
   保持・複製し得るため scope 外 escape を防げない。
4. **proxy 側 position cache + 独自 Len request**（SV 候補の方式） —
   真の source position/len と proxy cache が乖離する経路（外部 seek
   等）で正直でなくなる。逐語 forward に統一。

## `SourceReader` の借用 demuxer 対応（canonical decode 維持）

`SourceReader::open(Box<dyn Demuxer>)` が `'static` 所有を要求するため、
`f` 内で canonical decode をそのまま使えない問題は、consumer に pump
再実装させるのでなく、media-io / media-reader 側の既存 port を
`Box<dyn Demuxer + 'a>` へ一般化して解決する（coordinator 指摘に
対応。`Demuxer`/`Decoder` pump の複製をアプリへ要求しない）。

1. **media-io — reborrow port**:
   `impl<D: Demuxer + ?Sized> Demuxer for &mut D`（std の
   `impl<R: Read + ?Sized> Read for &mut R` と同じ先例）。全メソッドを
   `**self` へ逐語 forward。`Box::new(&mut *d)` が
   `Box<dyn Demuxer + 'a>` を得る道になる — 生成物は常に `'a` 束縛で
   `'static` 所有を捏造しない。`stream` は default impl が `streams()`
   経由で正しく forward するため省略可。`seekable`/`make_decoder` は
   default（`NotDeclared`/`Unsupported`）で嘘をつくため必ず forward する。
2. **media-reader — `SourceReader<'a>`**:
   `pub struct SourceReader<'a> { demuxer: Box<dyn Demuxer + 'a>, … }`。
   `open(demuxer: Box<dyn Demuxer + 'a>, …)` / `reopen(…)` のみ
   署名変化。`SourceReader::open(Box::new(owned_demuxer), …)` は
   `'a = 'static` で従来どおり推論されるため式位置の利用は無変更。
   型位置（struct field / fn 戻り値 / `&SourceReader`）で bare
   `SourceReader` を書いている箇所のみ `<'static>` / `<'_>` を
   書き足す（prefetch.rs 3 箇所 + tests 3 ファイル + SV 側 ~4 箇所、
   全て機械的）。`PrefetchProducer`/`spawn`/`manual` は所有 pipeline
   のため `SourceReader<'static>` で固定。
3. **consumer 側の呼び出し形（SV の verify 相当）**:

   ```rust
   with_borrowed_demuxer(&mut readback, |d| {
       let mut reader = SourceReader::open(
           Box::new(d), stream_id, DecodeMode::Canonical)?;
       // 既存の read_planar / next_frame pump をそのまま使用
       Ok(planes)
   })
   ```

   `Box::new(d)`（`d: &'b mut dyn Demuxer`）は `Box<dyn Demuxer + 'b>`
   となり `SourceReader<'b>` を生成 — `'b` 束縛で `f` 外へ出られない
   （`R` が `'b` を名指せないため return 不可。`mem::replace` で
   `'static` reader に差し替えても盗み出した値は `'b` 束縛のまま）。
   `Box::new(&mut *d)` で reborrow すれば `d` を後続でも使える。

## 制約（over-promise しない）

- `f` は **worker thread で実行される**（worker TLS を見る。caller
  の TLS や `!Send` 値は捕捉不可）。`f` が呼び出し側スレッドとの
  同期に依存して block する（caller が drain する channel へ送って
  待つ等）は真の deadlock になる。
- 呼び出し側スレッドは `f` の実行中ずっと serve ループで専有される。
- RT ではない（両側で channel I/O・source I/O・無制限 demux work が
  起き得る）。decode は caller 側の `!Send` 制約下で行う。
- in-flight encoded bytes は `CHUNK`（64 KiB）×（request 1 + response
  1）≈ 128 KiB が上限。decode 済み PCM の保持量は Consumer 次第
  （`Decoder` の内部 queue は別途 unbounded と既存記録済み）。
- `source` の seek position は呼び出し後未規定（probe が end/restore
  し sequential read が進める）。

## 影響範囲

- `ruviform-io-symphonia`: `src/borrowed.rs` 新設、`src/lib.rs` に
  `pub use`、README に 1 セクション、doctest 付き公開 API doc。
- `ruviform-media-io`: `demux.rs` に `impl<D: Demuxer + ?Sized> Demuxer
  for &mut D` 追加（reborrow port）。
- `ruviform-media-reader`: `SourceReader` → `SourceReader<'a>`（
  `reader.rs`/`lib.rs` doc/`prefetch.rs`/`tests` の `<'static>`/`<'_>`
  注釈）。公開 API の型変更であり、型位置で `SourceReader` を名指す
  consumer（SV ~4 箇所）は機械的な注釈対応が必要 — PR に影響を記録。
- `ruviform-io-symphonia/tests/borrowed.rs` +
  `ruviform-media-reader/tests/borrowed_source.rs`: 回帰 test 群。
- `ruviform-io-e2e/tests/wav_staged.rs`: 既存 E2E の独立 decode 部分を
  新 API 経由へ差し替え（`Vec<u8>` 全量 read を排除）。
- `mise.toml`: 変更なし。crate 単位の検証は確立済みの
  `mise exec -- cargo test -p` 入口を使う（`docs/tasks` 既存
  convention）。
