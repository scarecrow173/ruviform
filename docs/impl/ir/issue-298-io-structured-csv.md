---
id: "IMPL-NEfDGbqjDEqRUWZxTgR3"
type: "impl"
status: "completed"
title: "Issue 298 ruviform-io-structured — CSV format module（IO-IMP-04）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/298"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-structured/src/csv.rs"
      - type: "file"
        path: "docs/specs/issue-298-io-structured-csv.md"
      - type: "file"
        path: "docs/designs/issue-298-io-structured-csv.md"
      - type: "file"
        path: "docs/plans/issue-298-io-structured-csv.md"
      - type: "file"
        path: "docs/test-specs/issue-298-io-structured-csv.md"
      - type: "file"
        path: "docs/tasks/issue-298-t1-csv-module.md"
      - type: "file"
        path: "docs/tasks/issue-298-t2-file-docs.md"
      - type: "file"
        path: "docs/impl/ir/issue-298-io-structured-csv.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-structured/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-structured/README.md"
      - type: "file"
        path: "crates/ruviform-io-structured/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/input.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/file.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/json.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/yaml.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/toml.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/tests/structured.rs"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 298 ruviform-io-structured — CSV format module（IO-IMP-04）

## Summary

IO-IMP-03（Issue #297）で新設した `ruviform-io-structured` scaffold へ
`csv` feature/module を追加した。`csv-core` の pull-based parser を
caller 所有 buffer 駆動で回す `CsvReader` / `CsvRecord` / `CsvWriter`
streaming API と、buffer 全読みの `from_reader`/`to_writer`
convenience を提供し、scaffold 契約（借用 `Read`/`Write`・
`InputLimits` byte budget・正規化 `DocumentError`・backend 非公開）を
そのまま再利用した。CSV 固有の制約（field/record byte・field 数・
record 数・dialect 妥当性・BOM policy・formula injection 非対応）は
全て API 文書と README に正直に記録した。

## Context

Issue #298 は #297 で整備した共通契約の上に 2 番目の format module を
載せるタスク。JSON/YAML/TOML は「1 入力 = 1 文書」の有限 parse だが、
CSV は record 列であり scraffold の `from_reader`/`to_writer` だけでは
streaming・buffer 再利用・positional tracking・型付き record
deserialize の Acceptance を満たせない。design では「共通 free
function は維持しつつ、record streaming は format 固有 API として
`csv` module 内に閉じる」方針を採った。

## Implementation

### Changes

- `Cargo.toml` — `csv = ["dep:csv", "dep:csv-core"]` feature。
  `csv =1.4.0`（record container・Serde glue・writer）と
  `csv-core =0.1.13`（caller 所有 buffer の pull parser）を
  `default-features=false`・`=` pin で導入。いずれも
  Unlicense/MIT。
- `src/error.rs` — `DocumentFormat::Csv` 追加。`DocumentPosition` に
  `record: Option<u64>`（0-based record index）を追加し `Display`
  に `record` を配線。
- `src/input.rs` — `BoundedRead` に `Debug` derive（CSV reader 構造体の
  Debug 経由で診断可能に）。既存 `read_to_string` の
  `DocumentPosition` literal に `record: None` 追補。
- `src/{json,yaml,toml}.rs` — `DocumentPosition` literal に
  `record: None` 追補のみ。format 挙動は不変。
- `src/lib.rs` — `pub mod csv`（feature gated）と module 表に
  csv 行。
- `src/file.rs` — `open_document`/`write_document` の cfg ゲートに
  `csv` を追加（csv+file で staged publish 合成が有効化）。
- `src/csv.rs` — 本 Issue の中核。
  - `CsvDialect`（delimiter/quote/terminator/comment/double_quote/
    escape/quoting）— 構築時に backend が受け付けない矛盾設定
    （`delimiter == quote`・control byte 衝突等）を `Unsupported`
    で拒否。
  - `CsvReadOptions`（dialect・has_headers・flexible・trim・bom・
    limits・max_record_bytes/max_field_bytes/max_fields/max_records）、
    `CsvWriteOptions`（dialect・quote_style・has_headers・flexible）。
  - `CsvRecord` — caller 所有の再利用 buffer（data + ends index）。
    `field_bytes`/`field_str`/`iter`/`as_bytes`/`clear`。
  - `CsvReader<'a, R>` — 借用 `&mut R` + `BoundedRead` byte budget
    + 8 KiB inbuf + 最大 4 byte BOM probe の bound 済み lookahead。
    `read_record` は全 buffer 成長を事前監査（超過は `LimitExceeded`
    + limit 名）。`headers()` は lazy parse・`record_index` は
    header を除く data record 番号、`position()` は
    byte/line/record を返す。`read_typed::<T>` は完全 record 取得後に
    `csv::StringRecord` 経由で deserialize — 失敗は record 全体の
    `DataModel`/（非 UTF-8 field 由来は）`InvalidEncoding` で部分値を
    返さない。
  - `CsvWriter<'a, W>` — `write_record`（`CsvRecord` または field
    iterator）・`write_typed::<T: Serialize>`・消費型 `finish()`
    （内部 flush を明示）。`flexible=false` で非方形を `DataModel`
    拒否。
  - `from_reader`/`to_writer` — 全 record を `Vec<Vec<String>>`
    等の `T` に集約する convenience（budget は共通 `InputLimits`）。
  - `read_file`/`write_file`（`feature="file"`）— 共有
    `open_document`/`write_document` helper 経由。
  - `map_csv_error` — backend エラーを kind/format/position/source
    に正規化。`ErrorKind::Deserialize` 内の
    `DeserializeErrorKind::InvalidUtf8` は `InvalidEncoding`、その他
    deserialize 失敗は `DataModel`、unequal lengths は `DataModel`、
    io は `Io`、seek/serialize 等は `Unsupported`/`DataModel` へ。
- `tests/structured.rs` — CSV 契約試験 41 件 + file 合成試験。
  typed round-trip（struct/tuple/enum/Unicode）・streaming 逐次 read・
  record buffer 再利用・byte/line/record position・借用 lifetime・
  lookahead 境界・chunk 境界・budget 丁度/超過・oversized field/
  record/field 数/record 数・quoted newline・CRLF/任意 terminator・
  BOM strip/preserve・UTF-16 BOM 拒否・非 UTF-8 field・malformed
  quote（寛容受理の pin）・空行/空 field・ragged `DataModel`・
  flexible・複数 dialect（TSV/semicolon/comment/custom quote/
  escape）・header 有無・typed 非 UTF-8・typed 型不一致・I/O 途中
  失敗・writer finish・quote style・非方形 write・file Open/Decode/
  Encode/Publish・決定論 fuzz 的入力回し。
- `crates/ruviform-io-structured/README.md` — feature matrix・
  責務・「CSV の制約（正直な保証範囲）」・許可依存・Issue 一覧更新。
- `crates/README.md` — 索引行を CSV 実装済みに更新。
- `Cargo.lock` — cargo 解決で csv/csv-core/itoa/ryu/memchr/serde_core
  を追記（手編集なし）。

### Decisions During Implementation

- backend 二層構成: parsing は `csv-core`（caller 所有 buffer で
  上限超過前の割当抑止が可能）、record container・Serde
  deserialize・writer は `csv`。`csv` の `Reader` 内部 buffer は
  成長上限を外から制御できないため、budget/limit の authoritative
  実装は本 adapter の `csv-core` 駆動側に置いた。
- `Preserve` BOM policy: csv-core は最初の `read_record` で 3 byte
  以上あれば無条件に UTF-8 BOM を strip する（`has_read` ラッチ）。
  `Preserve` では最初の parser 投入を 2 byte に cap する `bom_guard`
  で strip 発火を回避し、BOM を field データとして残す。
- `refill` 条件を `!exhausted || prefix 未消化` に修正 — probe が
  短い入力で先に EOF へ到達した場合、prefix 残留 byte が parser に
  届かず末尾 record を drop する実害 bug を解消。
- header は lazy 取得・`record_index` から除外 — `headers()` を
  呼ばなくても data record は 0-based で数えられる。
- formula injection は変換しない — sanitizer ではない旨を API 文書・
  README・design で明文化。
- `max_records` は「yield 済み record 数」で監査（headers は数えない）。
- `DocumentPosition` は `#[non_exhaustive]` のため外部構築不可 —
  `record` 追加は後方互換。
- `CsvRecord` は `Vec<u8>` + `Vec<usize>` の flat 構造 — field 毎の
  個別 alloc を避けつつ `iter`/`field_*` で借用 view を提供。
  `clear` で capacity を保持して再利用。

## Related Experiments

### Adopted

- `csv-core` 直接駆動 — `csv::Reader` は内部 `Vec<u8>` を自動成長
  させ、field/record byte 上限を「割当前」に強制する経路がない。
  `ReadRecordResult` 区切りの手動駆動で全成長を監査可能にした。
- `bom_guard`（初回入力 ≤2 byte）— csv-core の内部 BOM strip を
  回避する最小の containment。workaround は `refill` 内の 1 条件に
  隔離され `Preserve` 時のみ有効。
- `csv::StringRecord` 経由 typed deserialize — header 名
  deserialize・pos deserialize・ByteRecord 非対応の制約を backend
  に委譲し、本 adapter は record → StringRecord 変換のみ担当。

### Rejected

- `csv::Reader` を streaming 本体に採用 — field/record byte 上限を
  割当前に強制できず Acceptance（bounded allocation）を満たせない。
- strict quote 検査の独自実装 — backend grammar が permissive で
  あり、自前の strictifier は二重責務。permissive である事実を文書化
  して採用せず。
- `read_typed` の streaming partial deserialize — backend は完全
  record 前提。部分値の黙示 truncation と矛盾するため採用せず。

## Validation

実施済み（Windows 11 x64 / mise 管理 toolchain rustc 1.99.0）:

- `mise run fmt` / `mise run fmt-check` — 全ソース正規化・差分なし。
- `mise run check` — `cargo check --workspace --all-targets` 全緑。
- `mise run lint` — `cargo clippy --workspace --all-targets --
  -D warnings` 全緑（`struct_excessive_bools` は `HeaderState`
  enum へ fold、`cognitive_complexity` は helper 分割で解消 —
  `#[allow]` は不使用）。
- `mise run test` — `cargo test --workspace` 全緑。
- `cargo test -p ruviform-io-structured --all-features` — 77
  契約試験（CSV 41 + 既存 36）+ 11 doctest（CSV 5 件含む）全緑。
- feature matrix（補助証拠・canonical は CI 側 default features）:
  `cargo test -p ruviform-io-structured --no-default-features
  --features csv` 36 CSV 試験全緑、`csv,file`・`csv,json,yaml,toml,
  file` 各組合せで全緑、`--no-default-features`・`file` 単体で
  警告 0 コンパイル、clippy 各組合せ `-D warnings` 全緑。
- MSRV: `mise exec rust@1.89.0 -- cargo test -p
  ruviform-io-structured --all-features --locked` 41 試験 + 5
  doctest 全緑、`mise exec rust@1.89.0 -- cargo check --workspace
  --locked` 全緑。
- 依存 DAG: `cargo tree -p ruviform-io-structured --features csv`
  で csv → csv-core → memchr / itoa / ryu / serde_core、serde —
  既存層外への新規依存なし。
- license: csv・csv-core・memchr は Unlicense(/MIT)、itoa・
  serde_core・serde は MIT OR Apache-2.0、ryu は Apache-2.0 OR
  BSL-1.0 — 配布条件に抵触する copyleft なし。
- `mise run bench-smoke` — ruviform-math/rt/viz bench `--test`
  smoke 全緑（baseline 劣化なし）。
- 非 RT 証跡: RT 層 crate からの `ruviform-io-structured` 依存は
  DAG 上存在せず、csv module も `std::io` ベースで RT 契約に
  触れない（crate README RT 契約節そのまま）。

未実施・補足:

- `cargo test` の feature matrix は workspace `test` task
  （default features のみ）の対象外のため補助証拠として実施。
  `ci/impact/map.mjs` の feature matrix 対象変更は CI 側検証が
  default features のみのため不要と判断。
- macOS/Linux 実 OS・実 FS 検証は CI lane に委譲。`csv+file`
  staged publish は Windows 実 FS で試験済み。
- 巨大入力の長時間 soak・真の peak-memory 定量計測は未実施 —
  buffer 成長監査は unit 試験で pin し、定量 profile は本 Issue の
  Acceptance 外。

## Risks

- csv-core の BOM strip は backend 内部挙動であり、将来版で仕様が
  変われば `bom_guard` の前提が揺れる。`=` pin で固定し bump 時は
  BOM 両 policy の契約試験で検知する。
- permissive grammar（未閉鎖 quote 受理等）は backend 契約 —
  「strict CSV」を期待する consumer には module 文書の記載を
  参照させる必要がある。
- `read_typed` は `csv::StringRecord` 経由のため非 UTF-8 field を
  含む record は deserialize 前に `InvalidEncoding` — raw bytes を
  typed read したい需要には `read_record` + `field_bytes` を使う
  設計であり、制約として文書化済み。

## Follow-ups

- IO-IMP-05 XML（Issue #299）— 本 scaffold・今回確立した
  `DocumentPosition.record`・feature wiring パターンを再利用可能。
- Issue #288（IO-06 共通 gate）・#219（cross-repo）での consumer
  受入統合検証。
- consumer 側で spreadsheet 用途の formula-injection 対策が必要に
  なった場合は consumer 側責務として field 前処理を実装（本
  クレートへの sanitizer 追加は設計上の非目標）。
