---
id: "SPEC-0kIy1SFw7ffSK1AAi4BXHu"
type: "spec"
status: "implemented"
title: "Issue #400 借用 sealed readback を独立 decoder へ bounded に接続する共通API"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/400"
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/synvelis/pull/181"
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
        path: "crates/ruviform-media-reader/tests/borrowed_source.rs"
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
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
  implements: []
  implemented-by:
    - "docs/designs/issue-400-borrowed-readback-decoder-bridge.md"
    - "docs/plans/issue-400-borrowed-readback-decoder-bridge.md"
  depends-on:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-306-io-composition-e2e.md"
    - "docs/specs/issue-197-media-reader-prefetch-rt-consumer.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-400-borrowed-readback-decoder-bridge.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/287"
    - "https://github.com/scarecrow173/ruviform/issues/391"
  defers: []
  deferred-by: []
metadata:
  issue: 400
---

# Issue #400 借用 sealed readback を独立 decoder へ bounded に接続する共通API

## What Should Be Built

`SealedFile::open_readback` が貸す借用・`!Send` な `Read + Seek`
(`ReadbackFile<'a>`) を、`open_demuxer` が要求する所有・`'static` な
`MediaSource` へ、追加メモリを定数に抑えたまま接続する最小の共通APIを
`ruviform-io-symphonia` に提供する。

背景: Synvelis PR181 の StagedFile 移行で、sealed WAV の独立 decode 検証
（pre-publish readback）が借用 reader を `read_to_end → Vec<u8> → Cursor`
で owned source に変換していた。これは encoded bytes 全量を同時保持し、
ピークメモリを出力サイズ分退行させる（SV 側修正候補
`synvelis@5fa1ef9` は scoped thread + bounded channel proxy でアプリ側に
解消済み）。この「借用 byte stream → 所有 source 要求の decoder」の
接続はアプリではなく共通基盤の責務として提供する（Issue #400）。

## Why

- `MediaSource` port は `Read + Seek + Send`、backend の `MediaSourceStream`
  が所有するため `impl MediaSource + 'static` が必要。借用・`!Send` な
  readback は満たせない。
- 全量 copy 回避策（生 OS handle・`try_clone`・unsafe lifetime 延長・
  temp path 再 open・独自 WAV parser）は型状態設計の意図的制約または
  明示禁止事項に触れるため使えない。
- アプリごとに channel/scope 配管を複製させると、deadlock 等の所有境界
  バグが各 Consumer に分散する。共通基盤に 1 つの正しい実装を置く。

## Acceptance Criteria

- AC-1: 借用 `&mut (impl Read + Seek + ?Sized)`（`Send` 不要求）を
  `&mut dyn Demuxer` の scoped closure port へ接続する公開関数が
  `ruviform-io-symphonia` に存在し、Consumer は channel・thread・
  proxy を一切操作しない。
- AC-2: `Read`/`Seek`/`len`/EOF/`Interrupted`/短読み/一時 error/
  position overflow が underlying source の挙動どおり正直に伝搬する
  （逐語 forward; proxy は状態を偽装しない）。
- AC-3: in-flight の encoded bytes は定数上限（request あたり ≤64 KiB、
  request/response 各 in-flight 高々 1 件）。入力サイズに対する追加
  メモリは O(1)。
- AC-4: proxy/demuxer の scope 外 escape を API 型上で不可能にする
  （`&mut dyn` port）。`f` の早期 return・error・panic・drop いずれでも
  serve は終了し scope は join する。panic は呼び出し側へ伝搬する。
- AC-5: `ReadbackFile` の借用・`!Send`・read-only の不変条件を維持し、
  sealed の publish/abort が borrow 終了まで排他であることを崩さない。
  raw handle・`try_clone`・path 再 open・新 WAV parser・全量 RAM copy・
  新外部依存・unsafe は使わない。
- AC-6: 既存の `open_demuxer`/`SourceReader`/canonical decode 経路と
  fixtures は維持する。`SourceReader` は `Box<dyn Demuxer + 'a>` を
  受ける `SourceReader<'a>` へ一般化し、`f` 内で
  `SourceReader::open(Box::new(d), …)` による既存 canonical decode
  （`read_planar`/`next_frame`/`seek_source_frame`）がそのまま使える。
  `SourceReader` を型位置で名指す consumer への影響は機械的注釈
  （`<'static>`/`'_>`）に限定され、PR に記録する。
- AC-7: bounded backing source/大論理サイズ・早期 decoder error・invalid
  source・fault・panic・Read/Seek 整合の最小回帰 test が追加される。
  正常 WAV の独立 decode 成功を含む。
- AC-8: 正規 mise 入口（`fmt`/`check`/`lint`/`test`/`test-io-e2e` と、
  crate 単位は確立済みの `mise exec -- cargo test -p` 入口）で実測 pass。

## Constraints

- 編集範囲は `ruviform-media-io` / `ruviform-io-symphonia` /
  `ruviform-io-file` の責務境界と、直接関連する tests / docs / mise のみ。
  新しい汎用 IO framework、便利な公開配管 API、公開 mock、外部依存は
  追加しない。
- `main` 互換を維持する。旧 API 撤去（RF391 候補 `d68b14d`）は本
  変更に混ぜない。
- `std::thread::scope` + request/response proxy は候補の一つであり、
  無条件にコピーしない（設計側で逐語 forward との比較を記録する）。
- 実行した検証のみを pass と報告し、未 OS / Native / 実メモリ計測を
  成功と主張しない。

## Out of Scope

- decoded PCM 全量保持の改善（SV 側既存の別制限。本件は追加 encoded
  全量 buffer の解消のみ）。
- `with_borrowed_reader` のような reader 級 scoped entry ——
  `SourceReader<'a>` 一般化により `f` 内構成で足りるため不要
  （YAGNI）。
- 他 backend（`ruviform-io-avi` 等）への同種 scoped entry。必要になった
  時点で plumbing の一般化を判断する（YAGNI）。
- RF391 の旧 `StagedOutput`/`PublishOptions` 撤去。
