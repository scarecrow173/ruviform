---
id: "TSPEC-EtHC4HLHJhXDwgG6UbrccI"
type: "test-spec"
status: "approved"
title: "Issue #303 ruviform-io-archive TAR 検証・Pending・Writer テスト仕様"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "docs/specs/issue-303-io-archive-tar.md"
    - "https://github.com/scarecrow173/ruviform/issues/303"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-archive/tests/tar_reader.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/tar_writer.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/common/tar.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-archive/tests/common/mod.rs"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-303-io-archive-tar.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-303-io-archive-tar.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #303 ruviform-io-archive TAR 検証・Pending・Writer テスト仕様

## Purpose

`ruviform-io-archive` の TAR 契約 — Streaming Reader の構造/Budget
検証・`Ready/EntryOpen/Pending/Failed` 状態機械・USTAR/GNU/PAX 実効
値・終端検証・Writer の宣言 size/finish 分離 — を byte レベルの
fixture（正常系 + 手作り否定系）で pin する。Issue #303 Acceptance
（途中 Drop→Pending・discard_pending 復帰・Header/宣言 size/
Short read/Padding/終端/PAX/GNU 実効名・未対応 Sparse/Special・
Writer 宣言長と実書込・tar.gz 二層 finish）が正本。Secure
Extraction の FS 安全性は #304 の Gate であり本 spec では検証しない。

## Feature

TAR Archive の逐次 Entry 解析・payload 消費・明示 discard・Archive
生成において、構造欠陥・宣言値と実測の不一致・Budget 超過・未対応
機能・状態機械違反が、設計どおりの `ArchiveErrorKind` / 状態遷移に
分類され、`finish` 成功のみが完了証拠となる。

## Rules

- R1（Header 構造分類）: header block 途中 EOF・拡張 payload 途中
  EOF・終端 marker 無しの EOF → `Truncated`。magic 不一致・octal/
  base-256 でない数値 field・PAX record 文法不正（len 不整合・
  `=`/改行欠落）・非 zero padding・終端後の非 zero 後続 byte →
  `Malformed`。chksum 実測 vs 宣言不一致 → `ChecksumMismatch`。
- R2（実効値解決）: name+prefix 合成（USTAR）・GNU `L`/`K`・PAX `x`
  の `path`/`linkpath`/`size` が設計の優先順位（PAX > GNU > USTAR）
  で実効値となり raw bytes を失わず `ArchiveName`/link target に
  反映される。非 UTF-8 name も raw 保持。
- R3（Unsupported 分類）: `'S'` sparse・`'3'`/`'4'`/`'6'` special・
  `'V'`/`'M'`/未知 typeflag → `Unsupported`（黙って処理しない）。
- R4（Entry 数/metadata/path Budget）: surfaced Entry 数が
  `max_entries` 超過 → `LimitExceeded`。header + 拡張 payload の
  累積が `max_metadata_bytes` 超過 → `LimitExceeded`（拡張 payload
  の宣言 size は Vec 確保前に照合）。実効 name/link target が
  `max_path_bytes` 超過 → `LimitExceeded`。
- R5（size Budget）: 宣言 size > `max_entry_uncompressed_bytes` →
  open 時 `LimitExceeded`。read/discard 累積が
  `max_total_uncompressed_bytes` を超過 → `LimitExceeded` で途中
  遮断（超過 byte を下流へ出さない）。
- R6（Pending 状態機械）: Entry Reader 途中 Drop（Pending 化）→
  `next_entry`・`finish` は `InvalidState`（非破壊の契約違反）。
  `discard_pending` は残 payload + padding を budget 内 drain して
  `Ready` 復帰。drain 中の io/構造失敗 → `Failed`（以後の
  `next_entry`/`finish`/`discard_pending` は `InvalidState`）。
  `Ready` での `discard_pending` → `InvalidState`。
- R7（finish/discard 分離）: `TarEntryReader::finish` は payload 全
  消費時のみ成功（Receipt `EntryDisposition::ReadToEnd`）。
  `discard_remaining` は残りを drain して `Discarded` の Receipt を
  返す。TAR の `checksum` は常に `EntryChecksum::NotPresent`。
- R8（Archive finish）: `finish` は残 Entry を discard 検証（構造・
  budget・padding を drain 中に確認し unverified 計上）→ 終端
  marker → EOF まで全 zero を確認して `(inner, ArchiveReadReceipt)`
  を返す。`entries_total`/`verified`/`unverified`/`payload_bytes` が
  正確。lone zero block + EOF は受理。
- R9（宣言 size 偽装の表面化）: 宣言と実データのずれは次 header
  位置を狂わせ、後続解析で `Malformed`/`ChecksumMismatch`/
  `Unsupported` 系として検出されること（「読み切った」を無破損と
  混同しない契約の pin）。
- R10（Writer cap/finish 分離）: `TarEntryWriter` は宣言 size で
  write を cap する（超過 write は拒否）。実書込 < 宣言での
  `finish` → Err + archive poison（以後 `start_entry`/`finish` は
  `InvalidState`）。Entry を finish せず Drop → poison。
  `TarArchiveWriter::finish` は 2 zero block を書いて inner を返す
  唯一の完了証拠。
- R11（Writer 出力検証）: name ≤100 → USTAR、`/` 分割可能 →
  prefix、`x` 必須の長名 → PAX `x` 先行 + header 切詰め、size が
  octal 上限超過 → PAX `size=`（header 欄は octal 最大）。出力は
  自前 Reader で round-trip 可能。
- R12（tar.gz 二層）: gzip decode 層を介した TAR 終端検証（TAR
  `finish` 成功）と、compression 側 trailer 破損が TAR 層の成功を
  汚さない/別契約であることを pin（decode 層の失敗は decoder 側
  finish/read で表面化）。
- R13（共通型契約）: `ArchiveEntryInfo.link_name` は TAR `1`/`2`
  で `Some`・他 kind/ZIP で `None`。`EntryReadReceipt.disposition`
  は ZIP で `ReadToEnd`・TAR discard で `Discarded`。公開 enum は
  `#[non_exhaustive]`。

## Examples

- USTAR 3 Entry（file + dir + file）を writer で生成 → reader の
  `next_entry` で順に read → `finish()` → archive `finish()` で
  `entries_verified==3`・`payload_bytes` 正確。
- Entry payload を半分だけ読み drop → `next_entry`/`finish` が
  `InvalidState` → `discard_pending` で復帰 → 次 Entry を read。
- chksum field を 1 byte 変えた fixture → `next_entry` が
  `ChecksumMismatch`、以後 `Failed`。
- name `longdir/…/file`（>100 byte・`/` 分割可）→ USTAR prefix 合成
  で同一実効名に復元。`/` 分割不能な 200 byte 名 → PAX `x` の
  `path` が実効名になる（GNU `L` fixture でも同じ結果）。
- `max_entries=2` で 3 Entry archive → 3 個目の `next_entry` が
  `LimitExceeded`。
- `S`/`3` typeflag header → `Unsupported`。
- writer `start_entry("a", 10)` に 7 byte しか書かず `finish` →
  Err + 以後 `start_entry` が `InvalidState`。
- `tar.gz`（flate2 decoder 経由）で TAR `finish` 成功 + trailer を
  破損した fixture で decoder 側の終端検証が独立に失敗。

## Guarantees

- `next_entry` Ok + Entry `finish` Ok = その Entry の宣言終端まで
  呼出し側が消費し padding が整合したこと（payload の無破損証明で
  はない — `NotPresent` が明示する）。
- `TarArchiveReader::finish` Ok = 全 Entry の構造（header/padding/
  budget）と終端 marker・全 zero 末尾が検証済みであること。
- Budget は「宣言値の事前拒否」と「実測の途中遮断」の両層で機能し
  超過 byte は下流 `Read` 呼出しへ渡さない。
- `Pending`/`Failed` からの復帰経路は `discard_pending` のみ（
  `Failed` は復帰不能）、契約違反呼出しは `InvalidState` で非破壊。

## Non-goals

- 実 FS への展開・Path Traversal 防止・symlink/hardlink の安全処理・
  TOCTOU（#304）。
- sparse/special の受理試験（`Unsupported` 拒否のみ pin）。
- GNU `L`/`K`・base-256 の Writer 出力試験（読み側受理のみ pin）。
- 他ツール（GNU tar/BSD tar）との相互運用性試験 — round-trip は
  自前 Reader/Writer 間のみ（fixture 自体が byte 正確なため）。
- PAX `g` の実効値適用試験（構造検証のみ・不適用を pin）。

## Risk

- Pending/discard の drain が宣言 size・padding・budget のどれかを
  数え損ねると、以後の header 位置がずれて誤検証を通すリスク。
- PAX record の `len` 解釈がずれると、文法検証をすり抜けた巨大
  alloc や誤った実効名を通すリスク。
- Writer の宣言不足を poison せず続行させると、宣言と実態がずれた
  TAR が下流 publish へ流れるリスク。
