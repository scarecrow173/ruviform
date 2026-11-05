# ruviform-io-structured

Ruviform の構造化ドキュメント I/O 基盤 — IO-02（Issue #284）設計の
実装（IO-IMP-03 / Issue #297、IO-IMP-04 / Issue #298、IO-IMP-05 /
Issue #299）。Serde 型付きの JSON/YAML/TOML `from_reader<T>` /
`to_writer<T>`、CSV の順次 record streaming API、XML の借用 event
streaming API を提供し、単一文書の Strict EOF・入力 byte budget・
正規化 `DocumentError`・format 固有制約の正直な文書化を担う。

## 責務

- Format 別 `json` / `yaml` / `toml` / `csv` / `xml` module の借用
  `Read`/`Write`/`BufRead` ベース API。公開する外部型は Serde の
  `Serialize` / `DeserializeOwned` のみ — backend（serde_json・
  serde-saphyr・toml・csv・quick-xml）の型・エラー・feature 名は
  公開 API に一切出さず、診断は `Error::source()` の `dyn Error`
  経由のみ。
- Strict EOF: JSON は文書後の非空白を `TrailingData` で拒否、YAML は
  複数文書を `TrailingData` で拒否。TOML は有限文書全体を parse する
  ため「文書後の余剰」と「文法違反」を区別できず `Syntax` とする
  （文書化済みの正直な分類）。
- `InputLimits` — `max_bytes` の hard cap は bounded reader が
  一箇所で実装し、超過は `LimitExceeded`（`limit()` に
  超過した上限名と値を保持）。`metadata().len()` 頼みの前置検査は
  使わない。byte 上限は CPU・deserializer の heap 消費を bound し
  ない — その旨を API 文書に明記する。
- `DocumentError` / `DocumentErrorKind` / `DocumentFormat` /
  `DocumentPosition` / `DocumentLimit` — format 横断の正規化
  エラー基盤。`Display` は backend レンダリング文字列や入力
  snippet を埋め込まない。
- `file` feature — `ruviform-io-file::StagedFile` 上の
  serialize → `seal()` → `publish(PublishOptions)` 合成。
  `DocumentFileReadError`（`Open`/`Decode`）・
  `DocumentFileWriteError`（`Create`/`Encode`/`Publish`）が失敗軸を
  分離し、`Publish` は `PublishFailure` 全体（effect・phase・
  durability 証跡・recovery info・recovery handle）を保存する。
  Encode 失敗時は destination を一切 publish しない（staged temp は
  Drop による best-effort cleanup）。

## 責務外（Non-goals）

- 製品側 Schema の所有・意味・移行 — AudioLens #139 / Synvelis #82
  等の consumer が保持する。本クレートは型無しの構造 I/O プリミティブ
  のみを提供する。
- 既存 Manifest/Project の byte-canonicalization — 本クレートは
  canonical 形態を一切決めない。byte 互換が必要なら consumer 側の
  golden test で担保する。
- CPU 時間・ヒープ消費の bound — byte budget は入力サイズの上限
  であり、解析コストの保証ではない。

## Feature matrix

| feature | 内容 | backend（private） |
| --- | --- | --- |
| `json` | JSON `from_reader`/`to_writer`・strict EOF・depth は backend 固定 | `serde_json =1.0.154` |
| `yaml` | YAML `from_reader`/`to_writer`・depth/alias budget | `serde-saphyr =2.0.0` |
| `toml` | TOML `from_reader`/`to_writer`・有限 buffer parse | `toml =1.1.8+spec-1.1.0` |
| `csv` | CSV streaming `CsvReader`/`CsvWriter`・再利用 `CsvRecord`・`read_typed`・dialect/BOM/limit 制御 | `csv =1.4.0`（`csv-core =0.1.13`） |
| `xml` | XML event streaming `XmlEventReader`・再利用 `XmlEventBuffer`・借用 `XmlEventRef`+`to_owned`・namespace 解決・DTD/entity 拒否・limit 制御 | `quick-xml =0.42.0` |
| `file` | `read_file`/`write_file`（`StagedFile` publish 合成） | `ruviform-io-file`（path） |

`default = []`。format feature と `file` は独立に組合せ可能
（`file` 単体でも共通エラー型群は利用可。format module 側の
`read_file`/`write_file` は各 format feature + `file` の両方が
有効なときのみ存在する）。

### YAML の制約（正直な保証範囲）

serde-saphyr は実 budget 機構を持つため、本クレートの YAML 制限は
実保証として働く:

- `max_bytes` — adapter の bounded reader（authoritative）+ backend
  reader budget の二重適用。
- `max_depth`（既定 64）・`max_aliases`（既定 50_000）—
  `YamlReadOptions` で調整可能。alias replay（per-anchor 展開・
  replay stack・総 replay event）と anchor/event/node/doc/scalar
  byte の各 budget は backend の有限既定値を維持し、超過は
  `LimitExceeded`（limit 名を付記できるものは `limit()` で報告）。
- `!!binary`・merge key・unsupported tag 等 — tag 未対応は
  `Unsupported` で報告し、silent に捨てない。
- 保証しないもの: CPU 時間・deserializer heap・`from_reader`
  経由の非有界外部 resolver（include/property 機構は既定で
  無効のまま公開しない）。

### TOML の制約

有限 buffer parse（`max_bytes` で bound 済み）+ 2 相分類:
DOM parse（`Syntax`）→ `try_into::<T>`（`DataModel`）。UTF-8 必須 —
非 UTF-8 は `InvalidEncoding`。streaming としては提供しない。

### JSON の制約

depth は serde_json の固定内部上限に従い、超過は `Syntax` —
`InputLimits` 上の調整可能制限としては露出しない（虚偽の
`LimitExceeded` 分類を行わない）。

### CSV の制約（正直な保証範囲）

JSON/YAML/TOML と異なり CSV は有限文書ではなく record 列であるため、
`csv` module は buffer 全読みの `from_reader`/`to_writer` に加えて
`CsvReader` / `CsvRecord` / `CsvWriter` の順次 streaming API を持つ:

- `CsvReader<R>` は借用 `&mut R` の上で 8 KiB chunk + 最大 4 byte の
  BOM probe だけを先読みする（lookahead は bound 済み）。
  `read_record(&mut CsvRecord)` は caller 提供バッファを再利用し、
  `CsvRecord` の data/ends 両方の成長は `max_record_bytes` /
  `max_field_bytes` / `max_fields` に先立って監査される —
  上限超過は `LimitExceeded`（`limit()` で上限名を報告）で、
  silent な truncation はしない。record 数は `max_records`、入力総量は
  共通 `InputLimits::max_bytes` で bound。
- `read_typed::<T>` は header 名（`has_headers`）または位置で
  Serde deserialize する。失敗は `DataModel`、field の非 UTF-8 は
  `InvalidEncoding` — どちらも record 全体の失敗であり部分値は返さない。
- `CsvDialect`（delimiter・quote・terminator・comment・double_quote・
  escape・quoting）は構築時に検証し、`delimiter == quote` のような
  矛盾設定は `Unsupported` で即座に拒否する（backend に渡せない制約を
  黙って無視しない）。
- Grammar は permissive: 未閉鎖 quote・quote 後の余剰文字は寛容に
  受理される（backend の契約であり、モジュール文書に明記）。
  非方形 record は `flexible == false` で `DataModel`。
- Encoding: UTF-8 BOM は `CsvBomPolicy` で strip（既定）/ preserve を
  選択 — preserve は field データとして残る。UTF-16 系 BOM は
  `InvalidEncoding` で拒否。record 内の非 UTF-8 field は raw bytes
  として読め、`field_str`/`read_typed` の UTF-8 要求点で
  `InvalidEncoding` となる。
- Formula injection（`=`-leading field 等）はデータとしてそのまま
  読み書きする — 本クレートは spreadsheet 向け sanitizer ではない。
  注入対策が必要な consumer は自前で field を前処理する。

### XML の制約（正直な保証範囲）

`xml` module は DOM を作らない pull 型 event streaming で、
`XmlEventReader<R: BufRead>` が caller 所有の `XmlEventBuffer` へ
借用 event（`XmlEventRef<'_>`）を供給する。`to_owned()` で buffer
から切り離した所有 event を得る。`Start`/`End`/`Empty`/`Text`/
`CData`/`Comment`/`Declaration`/`ProcessingInstruction`/`Reference`
の区別を保持し、EOF は `Ok(None)`（EOF 後の繰返し呼出しも
`Ok(None)`）で、エラー後の reader は poison され後続 read も失敗を
返し続ける。

- Namespace: `NsReader` 相当の resolver が event ごとに即時解決する。
  default namespace は prefix 無し element 名にのみ適用し、prefix 無し
  attribute 名には適用しない（XML Namespaces 勧告どおり）。`xml` /
  `xmlns` の reserved binding を保持し、`xmlns` 宣言は raw attribute
  stream にも残る。`End` 名は scope pop 前の自 scope で解決し、未宣言
  prefix は `Syntax`。
- Secure defaults: いかなる `<!DOCTYPE>`（internal subset / external
  ID とも）も `Unsupported` で拒否し、DTD の取得・展開は行わない。
  定義済み 5 entity と数値文字参照のみ `Reference` event として明示
  報告し、未定義 general entity は `Unsupported` で拒否 — entity
  展開は一切しない。宣言された非 UTF-8 encoding は `Unsupported`、
  入力の非 UTF-8 byte 列は `InvalidEncoding`。
- Limits: `InputLimits::max_bytes` の共有 byte budget に加え
  `max_depth`（既定 512）・`max_attributes`（既定 512）・
  `max_event_bytes`（既定 16 MiB・単一 event の source byte 幅）・
  `max_events`（既定 1_000_000）・`max_namespace_bindings`（既定は
  backend の 128）を `XmlReadOptions` で制御し、超過は
  `LimitExceeded`（`limit()` に上限名）。`max_event_bytes` は event
  公開時の検査であり、backend がその byte 数を内部 buffer 済みの
  可能性は正直に文書化する（事前確保 cap ではない）。
- Encoding / text: 入力は UTF-8 のみ受理。`Text` payload は宣言
  version（1.0/1.1）の行末正規化を backend から受け、`CData`/
  `Comment` は raw のまま借用公開する。
- Borrow 契約: `Text`/`CData`/`Comment` payload は buffer を直接借用
  し、element/attribute 名・属性値・namespace URI・宣言/PI 文字列は
  backend 都合で所有コピーとなる（`max_event_bytes` で bound）。
  event を保持したまま `position()`/`depth()` は呼べる。
- Writer / serde typed deserialize / schema validation（XSD・
  RelaxNG・XInclude）は非対象。XML 1.0/1.1 grammar 上妥当でも複数
  root・top-level text のような document 構造検査は本 module の
  責務外（backend 忠実な streaming 契約）。

## 層・許可依存

- 層: 構造化ドキュメント I/O 基盤。`ruviform-io-file`（`file`
  feature 時・opt-in）への下向き依存のみで、media-io / domain /
  engine 層への依存は持たない。io-file 側から本クレートへの依存は
  禁止（循環を作らない）。
- 許可依存: `serde`（公開 API・trait 境界のみ）、optional な
  `serde_json` / `serde-saphyr` / `toml` / `csv` / `quick-xml`
  （backend・private）、optional な `ruviform-io-file`（path）。
  GUI・audio・GPU への強制依存なし。
- `default = []`・MSRV 1.89・`publish = false` 継承。

## RT 契約

- 全 API 非 RT — I/O にブロックし alloc も行う。
  RT オーディオコールバックからは絶対に呼ばない。

## 実装 Issue

- Issue #284: IO-02 設計（serde trait 公開・strict EOF・
  error 正規化・backend 非公開の原則）
- Issue #297: 本クレートの新設・公開契約・format adapter・
  file feature（IO-IMP-03）
- Issue #298: CSV format module（IO-IMP-04・本 scaffold 上）
- Issue #299: XML format module（IO-IMP-05・本 scaffold 上）
- Issue #288: IO-06 共通 gate（統合・consumer 受入の上流 gate）
