---
id: "TSPEC-ZpVwcAYhu5UaIR5HfPUNAb"
type: "test-spec"
status: "approved"
title: "Issue #285 IO-03 Compression — Serde境界・Format/Version契約・Consumer利用面・証拠項目対応 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/285"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/specs/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/plans/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/tasks/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/test-specs/issue-285-io-03-compression-serde-boundary.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
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
    - "docs/designs/issue-285-io-03-compression-serde-boundary.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-285-io-03-compression-serde-boundary.md"
    - "docs/designs/issue-285-io-03-compression-serde-boundary.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #285 IO-03 Compression — Serde境界・Format/Version契約・Consumer利用面・証拠項目対応 設計

## Purpose

Issue #285 の設計正本（[design](../designs/issue-285-io-03-compression-serde-boundary.md)）
が「Serde 非依存境界・Format/Version 契約・Finish/状態機械・
Resource Budget・Error 契約・Feature/Backend 確定値・Consumer
利用面・証拠項目対応を、実在する per-format 正本への参照で
構成し、設計合意と実装/検証の完了を混同していない」ことを
検証する。対象 spec は
[spec](../specs/issue-285-io-03-compression-serde-boundary.md)、
実施記録は [task](../tasks/issue-285-io-03-compression-serde-boundary.md) /
[plan](../plans/issue-285-io-03-compression-serde-boundary.md) を参照。
コード変更を伴わない docs 成果物のため、検証は文書検査
（link/relation 解決・参照先の実在確認・内容の突合・lint）で
行う。本 test-spec は横断設計文書の正しさを対象とし、
per-format の実走テスト自体の合否・Consumer 実測は対象外
（実装/受入段階の要件）。

## Feature

IO-03 Compression 横断設計文書（Serde 非依存境界・
Format/Version 契約・Finish/状態機械・Consumer 利用面・
証拠項目対応）

## Rules

- **R1（Serde 境界の完全性）**: 設計文書は公開 API に許可
  される外部型を `std::io::{Read, Write, BufRead}`・標準
  型・Ruviform 独自型に限定し、serde への依存・serde 型の
  不使用、backend（`flate2`/`zstd-safe`/`zstd-sys`/`zstd`）
  の型・Error・feature 名・version の非公開、
  `Error::source()` 経由のみの診断到達、自作 codec
  registry/format 自動判定を作らないことを明記する。
  検証根拠として `tests/api_surface.rs`・`issue-300` R8・
  `issue-301` AC6 への参照を持つ。
- **R2（Format/Version 契約）**: 3 形式（Raw Deflate
  RFC 1951・Gzip RFC 1952・Zstd Frame RFC 8878）の区別・
  Zlib wrapper（RFC 1950）対象外・Format 別
  `EncodeOptions`/`DecodeOptions`（level semantics を
  形式別に保持）・`DecodeScope::{WholeInput, SingleUnit}`・
  連結 member/frame の累積処理・`IntegritySummary` の
  checked/unchecked 分離・再現性の範囲（同一設定
  round-trip + gzip header 固定・backend version 間 byte
  一致は非保証）が記述される。
- **R3（参照の実在性）**: 設計文書・spec・plan・task が
  参照する Issue 番号（#282〜#306・#409・#219・#294・
  AL #139・SV #82/#84/#25）・PR 番号（#356/#359）・docs
  パス（per-format spec/design/test-spec・IO-01・Archive・
  #288・#306・#409）・crate 名（`ruviform-io-compression`・
  `ruviform-io-file`・`ruviform-io-archive`・
  `ruviform-io-e2e`）・test ファイル名（`tests/
  encoder_contract.rs` 等）・mise task 名
  （`test-io-compression`/`check-io-compression`/
  `lint-io-compression`）・CI lane 名（`compression-verify`）
  が作成時点で実在する。存在しない Issue/文書/command を
  根拠として記載しない。
- **R4（Finish/状態機械）**: Encoder の
  `Writing -> Finishing -> Finished | Failed`・Decoder の
  per-format `Phase` + `failed` latch（`Phase::Done` =
  VerifiedEnd）・`finish()` の冪等性（保存 receipt
  再返却）・Drop の無書込・`Finished`/`Failed` での
  `InvalidState`（Panic しない）・`Decoder::finish()` が
  状態照会のみで未読出力を破棄しない（`NotExhausted`）・
  `read(&mut [])` の `Ok(0)` を完了証拠にしないことが、
  実装（`src/stream.rs`・`src/{gzip,deflate,zstd}.rs`）と
  一致して記述される。
- **R5（Resource Budget）**: `EncodeLimits`
  （input/output byte・上限超過の物理書込を行わない）・
  `DecodeLimits`（compressed/decoded/units の連結累積・
  ちょうど許容・+1 は `LimitExceeded`・`Body` 相の
  1-byte scratch probe）・zstd `max_window_log`（既定 27・
  clamp 10..=41・header parse で backend 到達前拒否）の
  実装済み値が記述され、CPU 時間・backend 内部 heap・
  consumer heap を hard limit と主張しない非保証が明記
  される。
- **R6（Error 契約）**: `CompressionErrorKind` の 12
  variant（Io/InvalidInput/InvalidFormat/TruncatedInput/
  CorruptData/ChecksumMismatch/TrailingData/LimitExceeded/
  Unsupported/Cancelled/InvalidState/NotExhausted）と
  `io::ErrorKind` 対応・`impl Read`/`impl Write` での
  `io::Error` 包摂時も `source()` で kind を復元できる
  ことが `src/error.rs` と一致して記述される。
- **R7（Feature/Backend 確定値）**: feature matrix
  （`default=[]`・`gzip`/`deflate`/`zstd`・`flate2`
  共有）と backend 選定表（flate2 1.1.10 rust_backend・
  zstd-safe 8.2.0・dev-dep `zstd` 0.14.0・不採用候補の
  理由）が `crates/ruviform-io-compression/Cargo.toml`・
  #300/#301 設計の「Backend 選定と隠蔽」節と一致し、全
  backend の MSRV が workspace MSRV 1.89 以下である。
- **R8（Consumer 利用面）**: AudioLens（AL #139・Dataset
  圧縮候補）と Synvelis（SV #82/#84・Backup 圧縮
  tar.gz/zip・SV #25）が、schema/version/canonical
  bytes を Ruviform に移さず `std::io` 合成で利用できる
  面として記述され、checksum が整合性検出であり真正性
  保証でないこと・Ruviform 単一 Publish ≠ Project
  Transaction/HEAD CAS の責務分界が含まれる。
- **R9（証拠項目対応）**: Issue #285 の ENC-01〜06・
  DEC-01〜03・SEC-01/02/04/05・INT-01/02・ARC-01・
  COMP-01/03・BUF-01/02 と Design Acceptance Gate 6 項目
  の全てが、実在する per-format test-spec の rule/節・
  #288 matrix 行（C-N1/C-N2/C-X1〜C-X4・E-N1/E-X1/E-X2）・
  実装 Issue/PR へ対応付けられている。
- **R10（設計/実装分離）**: 「実装段階の検証（Consumer
  実測・OS 別実機・性能比較）と本 slice の文書合意を混同
  しない」旨が明記され、未実施項目は `not run`/`NotRun`/
  `Unsupported` と記録する方針が示される。
- **R11（変更範囲）**: 差分が `docs/` の新規
  `issue-285-*` 文書と生成 index のみであり、`crates/`・
  `Cargo.toml`・`Cargo.lock`・`mise.toml`・`.github/` に
  変更がない。

## Examples

- Given Issue #285 v0.2-A の Encoder 状態機械、When
  設計文書の Finish 節を検査する、Then `Writing ->
  Finishing -> Finished | Failed`・finish 冪等・Drop 無
  書込・`InvalidState` 拒否が `src/stream.rs`・
  `tests/encoder_contract.rs` への参照を伴って記述される
  （R4）。
- Given `flate2::Compress` が公開 API の戻り値として
  現れない設計、When Serde 境界節を検査する、Then
  「serde 依存ゼロ・backend 型/Error/feature/version は
  非公開・診断は `source()` の `dyn Error` のみ・自作
  codec/registry なし」が明記され `issue-300` R8・
  `issue-301` AC6・`api_surface.rs` への参照を持つ（R1）。
- Given consumer-verify の Synvelis lane が `not run`、
  When Consumer 受入を判定する、Then 「両 Consumer 受入
  済み」とは記録されず `not run` のまま残る — 本書は
  Consumer 独立判定と設計/実装分離を明記している（R8/R10）。
- Given Issue #285 の ENC-06（Backend Drop 副作用）項目、
  When 証拠項目対応表を検査する、Then `issue-300` R2・
  `issue-301` AC5・#300/#301 設計「Backend 選定と隠蔽」・
  `drop_writes_*` 試験への参照を持つ行が存在する（R9）。
- Given 本 PR の diff、When `git status`/`git diff --stat`
  を確認する、Then 変更は `docs/` の `issue-285-*` 新規
  文書と `docs/*/README.md` index のみ（R11）。

## Guarantees

- 横断契約の記述は per-format 正本（#300/#301 の設計・
  test-spec）への参照で構成され、詳細の複写による drift
  を避ける構造。
- Serde 境界・budget・error 分類の検証は既存 test-spec・
  `tests/api_surface.rs`・crate README の契約記述に pin
  されており、本書はその索引を提供する。
- Backend 選定の版・MSRV・ライセンスは `Cargo.toml` の
  dependency record と per-format 設計の記録に一致し、
  独自の未検証な主張を追加しない。
- 証拠項目対応表の各行は実在する正本への参照のみを持ち、
  存在しない Issue/文書/command を根拠にしない。
- Consumer の schema/version/canonical bytes 所有と、
  checksum=整合性検出（真正性でない）・Ruviform 単一
  Publish ≠ Project Transaction/HEAD CAS の責務分界が
  #288 と同一原則で記述される。
- DAG 記述は実態と一致する: `ruviform-io-compression` の
  `ruviform-*` 依存ゼロ、`ruviform-io-archive` からの
  参照は dev-dependency のみ、合成は `std::io` のみ。

## Non-goals

- per-format の実走テスト合否（`issue-300`/`issue-301`
  test-spec と `mise run test-io-compression` が正本）。
- Consumer repo での Golden/byte 互換/性能の実測
  （#219/#294 consumer-verify・AL/SV Issue が Owner）。
- macOS/Linux 実機・Network/Cloud FS での `zstd` feature
  （vendored C build）経路の実測。
- Zlib wrapper・preset dictionary・所有型 API・async/
  no_std・skippable frame 書込・Archive 内 codec 統合・
  schema/version/migration の Ruviform 側実装。
- `crates/`・`Cargo.toml`・`mise.toml`・`.github/` の変更。

## Risk

- 本 test-spec が検証するのは横断設計文書の正しさで
  あり、IO-03 の Consumer 受入（COMP-01/03）は #288 gate
  と consumer-verify の実走段階で別途検証される。設計
  文書が完璧でも Consumer 実測が `not run` なら Issue
  #285 の受入は未完である — この分離を崩すと
  「設計合意＝受入完了」の誤認が起きる。
- 参照先 Issue の状態（OPEN/CLOSED）は変動する。本書は
  2026-11-09 時点の snapshot（AL #139 CLOSED・SV #82
  CLOSED・SV #84 OPEN・SV #25 CLOSED）を記録しており、
  将来の状態変化で接続表の「役割」記述が古くなる可能性
  がある — snapshot 日付の明記で緩和する。
- `CompressionError` の `Display` は `source` の
  `Display` を連結する実装であり、IO-02 の `DocumentError`
  （Display に backend 文字列を含めない）とは診断面の
  保証が異なる。本書は実装どおり「kind が安定語彙・
  source が診断」と記述し、backend 文字列の秘匿を主張
  しない — この差異を「同一契約」と誤記しないことを
  R6 で担保する。
