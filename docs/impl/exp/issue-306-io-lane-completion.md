---
id: "EXP-issue-306-io-lane-completion"
title: "Issue #306 G01/G05 lane 実装時の API・lint・計測設計の検討経緯"
created: "2026-11-14"
updated: "2026-11-14"
---

# Issue #306 G01/G05 lane 実装時の検討経緯(2026-11-14)

- 計測契約: `docs/specs/issue-306-io-lane-completion.md` +
  `docs/designs/issue-306-io-lane-completion.md`
- 正本証拠: `evidence/rf306/large-bench-report.md`

## 1. Archive/Compression encoder API の実シグネチャ確認

### 問い

TAR→gzip/zstd→`StagedFile` 合成 bench を書くにあたり、encoder と
tar writer の正確な `new`/`finish` シグネチャが不明だった。

### 調査

`ruviform-io-compression` / `ruviform-io-archive` の source と
既存 `tests/archive_compression_staged.rs` を確認した結果:

- `gzip::Encoder::new(&mut W, EncodeOptions)` — writer を **借用**
  して encoder を作る。`finish(&mut self) -> EncodeReceipt` で
  receipt のみ返し、underlying writer は返さない。
- `zstd::Encoder` も同型。
- `TarArchiveWriter::new(W)` — writer を **値で** 受け取り、
  `finish() -> Result<W, ArchiveError>` で writer を返す。
- entry は `start_entry(name, declared_size, &TarEntryOptions)` →
  `TarEntryWriter::finish()` → 次 entry、最後に archive `finish()`。

### 結論

encoder は `&mut StagedFile` を borrow する形で合成し、encoder
`finish` 後に同じ `StagedFile` へ追記・seal できる構成にした。
tar writer は値受けのため encoder の `&mut W` を `W` として渡す
（`W = &mut Encoder<...>` の二重 borrow 構成）で、receipt を
捨てて encoder を drop すれば `StagedFile` が戻る設計を採用。

## 2. Criterion `iter_batched` / `BatchSize` の意味論確認

### 問い

fixture 準備（8MiB payload 生成・StagedFile create）を計測窓の
外に置く方法を確定する必要があった。

### 調査

vendored criterion 0.8.2 source を確認:

- `BatchSize::PerIteration` は「1 iteration = setup 1 回 +
  routine 1 回」で、setup の生成・破棄は計時対象外。
- `iter_batched(setup, routine, BatchSize::PerIteration)` の
  `I: Sized`（所有値を routine へ渡す）、
  `iter_batched_ref` は `&mut I` を渡す亜種。

### 結論

`staged_seal`/`staged_publish`/`staged_readback` は
`iter_batched`（所有 `StagedFile`/`SealedFile` を渡す）、
`staged_write`/tar 合成 cell は `iter_batched_ref`（`&mut
StagedFile` へ書き込む routine）を採用した。

## 3. `fill_pattern` の `chunks_exact` lint 対応

### 問い

固定長 8byte word pattern 生成で `clippy::chunks_exact_to_as_chunks`
（pedantic）が出た。

### 結論

`buf.as_chunks_mut::<8>()` で `(&mut [[u8; 8]], &mut [u8])` を
得て word/tail を分離する形に書き換え、lint を `#[allow]` なしで
解消した。tail byte は絶対 offset の `mix64(pos)` 下位 byte で
従来と同じ意味論を維持した。

## 4. peak heap 計測の `unsafe` 許可範囲

### 問い

counting `GlobalAlloc` は `unsafe impl` が必須だが、
`workspace.lints.rust.unsafe_code = "warn"` + `-D warnings` で
実質 deny。クレートの `#![forbid(unsafe_code)]` は `src/lib.rs`
のみ適用で bench target は対象外だが、lint ゲートは効く。

### 結論

`#[expect(unsafe_code)]` + SAFETY コメントで `CountingAlloc` の
`unsafe impl GlobalAlloc` のみに局所許可を付けた。計測は
`WINDOW_OPEN` atomic で区間限定し、peak は計測 window 内の
`current − base` の最大値として記録する。bench 間で状態が漏れない
よう各 `measure_peak` 呼出で open→close を対にした。

## 5. G01 の NotRun 分類設計

### 問い

容量不足を Pass/Fail に偽装しないため、どの失敗を `NotRun` に
分類するか。

### 結論

scenario 関数を `Result<(), NotRun>` にし、`io::ErrorKind::
StorageFull` と `FileErrorKind::Io`/`Unsupported` の OS 起因
失敗を `NotRun` へ写像する。`InvalidInput` や assertion 失敗は
通常の panic（Fail）として残し、契約違反と環境不足を分離した。
preflight の `set_len` probe 失敗も同じ `NotRun` 経路に流し、
要求量・temp path・観測値を stdout に残す。
