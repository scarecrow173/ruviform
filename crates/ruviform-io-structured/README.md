# ruviform-io-structured

Ruviform の構造化ドキュメント I/O 基盤 — IO-02（Issue #284）設計の
実装（IO-IMP-03 / Issue #297）。Serde 型付きの JSON/YAML/TOML
`from_reader<T>` / `to_writer<T>` を提供し、単一文書の Strict EOF・
入力 byte budget・正規化 `DocumentError`・format 固有制約の正直な
文書化を担う。後続の IO-IMP-04 CSV（Issue #298）・IO-IMP-05 XML
（Issue #299）は本クレートの scaffold 上に format module を追加する。

## 責務

- Format 別 `json` / `yaml` / `toml` module の借用 `Read`/`Write`
  ベース API。公開する外部型は Serde の `Serialize` /
  `DeserializeOwned` のみ — backend（serde_json・serde-saphyr・toml）
  の型・エラー・feature 名は公開 API に一切出さず、診断は
  `Error::source()` の `dyn Error` 経由のみ。
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

## 層・許可依存

- 層: 構造化ドキュメント I/O 基盤。`ruviform-io-file`（`file`
  feature 時・opt-in）への下向き依存のみで、media-io / domain /
  engine 層への依存は持たない。io-file 側から本クレートへの依存は
  禁止（循環を作らない）。
- 許可依存: `serde`（公開 API・trait 境界のみ）、optional な
  `serde_json` / `serde-saphyr` / `toml`（backend・private）、
  optional な `ruviform-io-file`（path）。GUI・audio・GPU への
  強制依存なし。
- `default = []`・MSRV 1.89・`publish = false` 継承。

## RT 契約

- 全 API 非 RT — I/O にブロックし alloc も行う。
  RT オーディオコールバックからは絶対に呼ばない。

## 実装 Issue

- Issue #284: IO-02 設計（serde trait 公開・strict EOF・
  error 正規化・backend 非公開の原則）
- Issue #297: 本クレートの新設・公開契約・format adapter・
  file feature（IO-IMP-03）
- Issue #298/#299: 後続の CSV / XML format module（本 scaffold 上）
- Issue #288: IO-06 共通 gate（統合・consumer 受入の上流 gate）
