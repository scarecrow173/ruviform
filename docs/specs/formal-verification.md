---
id: "SPEC-rf806frmlvrf0slice1"
type: "spec"
status: "proposed"
title: "Issue #806 Formal Verification D 層 — proptest + Kani 検証基盤（slice 1: wire codec / slice 2: archive parser / slice 3: structured documents）"
created: "2026-11-20"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/806"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/tests/formal_ipc.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/formal_transport.rs"
      - type: "file"
        path: "crates/ruviform-ui/tests/formal_ipc.proptest-regressions"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/formal_archive.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/tests/formal_structured.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/tests/formal_structured.proptest-regressions"
      - type: "doc"
        path: "docs/specs/formal-verification.md"
      - type: "dir"
        path: "evidence/rf806/"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-archive/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-structured/src/input.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".agents/skills/verify-ruviform/SKILL.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  supersedes: []
  superseded-by: []
  related:
    - ".agents/skills/verify-ruviform/SKILL.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/806"
  derived-by: []
---

# Formal Verification — wire codec + archive + structured slice（rvf.ipc.v1 / rvf.uitest.v1 / tar / zip / json / yaml / toml / csv / xml）

Issue `scarecrow173/ruviform#806` の D 層検証（property-based
testing → bounded model checking → 将来の TLA+/Lean 昇格）実装
spec。slice 1 で unsafe を含まない wire codec に proptest と
Kani の 2 層を常設し、slice 2 で同じ型を `ruviform-io-archive`
の TAR/ZIP reader と `ArchiveReadLimits` に、slice 3 で
`ruviform-io-structured` の JSON/YAML/TOML/CSV/XML reader と
共有 `InputLimits`/`BoundedRead` byte budget に適用した。

## 対象と検証する契約

### slice 1 — wire codec

| 対象 | ファイル | 契約 |
| --- | --- | --- |
| `parse_intent` | `crates/ruviform-ui/src/ipc.rs` | 任意入力で panic しない・`Err` は安定コード集合（`MESSAGE_TOO_LARGE` / `MALFORMED_MESSAGE`）に留まる・`Ok` なら envelope gate（`v=1`・`kind="intent"`・`valid_op`）を満たす・`MAX_INTENT_BYTES` 超過は `MESSAGE_TOO_LARGE` |
| `valid_op` | 同上 | 受理は「非空の `.` 区切り・各セグメント非空・`[a-z0-9._-]`・128 文字以下」のみ。全入力に対し total（panic なし）かつ sound（受理は仕様形状を満たす） |
| `Event::to_json` | 同上 | 常に parse 可能な `v=1` envelope を返し、variant kind と全フィールドを保持する |
| `read_frame` | `crates/ruviform-ui-test/src/transport.rs` | `Frame` / `Eof` / `TooLarge` / `Err(UnexpectedEof)` / `Err(InvalidData)` の tri-class が宣言長と残バイト数で一意に決まる。`TooLarge` は宣言本文を drain してストリーム整列を保つ |
| `write_frame` | 同上 | `MAX_INTENT_BYTES` 超過を `InvalidInput` で拒否。in-cap 本文は `u32le` 長 + UTF-8 で書き出し、`read_frame` で同一文字列が復元される |

### slice 2 — archive reader（`ruviform-io-archive`）

| 対象 | ファイル | 契約 |
| --- | --- | --- |
| TAR streaming reader | `crates/ruviform-io-archive/src/tar/`（`read.rs` / `header.rs`） | 任意 byte 列で panic せず、`Err` の `ArchiveErrorKind` は安定集合（`Io`/`InvalidInput`/`InvalidState`/`Truncated`/`Malformed`/`Inconsistent`/`ChecksumMismatch`/`SizeMismatch`/`LimitExceeded`/`Overlap`/`Unsupported`）に留まる。typeflag は surfaced entry / folded extension / `Unsupported` のいずれか一つに一意分類される |
| ZIP reader | `crates/ruviform-io-archive/src/zip/` | 任意 byte 列で panic せず `Err` は同じ安定 kind 集合。central-directory/local-header の不整合は `Inconsistent`、CRC 不一致は `ChecksumMismatch`、宣言/実測サイズ不整合は `SizeMismatch`、EOCD の multi-disk・ZIP64 必須は `Unsupported`（entry 数・metadata gate より先） |
| `ArchiveReadLimits` | `crates/ruviform-io-archive/src/limits.rs` | `max_entries` / `max_metadata_bytes` / `max_path_bytes` / `max_path_depth` / `max_entry_uncompressed_bytes` / `max_total_uncompressed_bytes` の各 gate が allocation/parse 前および streaming/decompress 中に `LimitExceeded` を返す |
| completion / receipt | `entry.rs` / `receipt.rs` | `finish` / `discard_remaining` / `discard_pending` が authoriative な完了点。receipt は走査した entry 数と payload byte 数の不変条件を満たす。TAR の pending entry は `discard_pending` で回復し reader が継続可能 |
| roundtrip / codec | `tests/common/` fixture + writer | writer 生成アーカイブ（TAR・ZIP stored/deflate）は reader が同一 name/size/payload で復元する。TAR は payload 完全性を持たない（構造・宣言サイズまで）・ZIP は CRC32 で payload 完全性を検証する |

### slice 3 — structured document readers（`ruviform-io-structured`）

| 対象 | ファイル | 契約 |
| --- | --- | --- |
| `InputLimits` / `BoundedRead` | `src/limits.rs` / `src/input.rs` | `max_bytes` は inclusive の hard budget: 丁度 `max_bytes` byte の入力は受理され得るが、超過 1 byte 目で `BoundedRead` の 1-byte probe が `BudgetExceeded` を返し `was_limit_exceeded` が立つ。`Ok(0)` は真の EOF のみ — budget 外の byte を EOF に見せかけない。trip/EOF は共に sticky |
| 共有 byte budget（全形式） | 各 reader + `BoundedRead` | `limit < len` なら `LimitExceeded`（`limit.name == "max_bytes"`・`max == limit`）。形式ごとに「tail byte が budget 内で先に非 limit error を出す」優先順位は異なる — JSON は strict-EOF check が tail を逐 byte 読むため「budget 内の最初の非空白 byte は `TrailingData`」が正確な境界 |
| `DocumentErrorKind` 安定集合 | `src/error.rs` + 各 reader | 任意入力で panic せず、`Err` の kind が形式別の文書化集合内に留まる — json {`Io`,`Syntax`,`DataModel`,`TrailingData`,`LimitExceeded`}、yaml {`Io`,`InvalidEncoding`,`Syntax`,`DataModel`,`TrailingData`,`LimitExceeded`,`Unsupported`}、toml {`Io`,`InvalidEncoding`,`Syntax`,`DataModel`,`LimitExceeded`}（trailing byte は `Syntax` — `TrailingData` なし）、csv {`Io`,`InvalidEncoding`,`DataModel`,`LimitExceeded`,`Unsupported`}（緩い文法 — `Syntax`/`TrailingData` なし）、xml {`Io`,`InvalidEncoding`,`Syntax`,`LimitExceeded`,`Unsupported`}（event model — `DataModel`/`TrailingData` なし） |
| strict EOF / trailing | `src/json.rs` / `src/yaml.rs` / `src/toml.rs` | JSON は文書後の非空白 byte が `TrailingData`（JSON の空白集合は space/tab/LF/CR のみ — form feed は `TrailingData`）。YAML は 2 文書目が `TrailingData`。TOML は trailing continuation byte が `Syntax` |
| JSON/YAML/TOML roundtrip | 各 `to_writer`/`from_reader` | 生成した型付き値は serialize→parse で同値に復元される |
| YAML depth/alias gate | `src/yaml.rs` | `max_depth` は入れ子深度 `d` が `d > m` で `LimitExceeded`（`limit.name == "max_depth"`）・alias 爆弾は `max_aliases` で同様に gate |
| CSV 独立 limit | `src/csv.rs` | `max_record_bytes` は buffer capacity gate（`data.len() >= max` が `OutputFull` で発火 — terminator を伴う record は `r >= m` で trip）、`max_field_bytes` は `s > m`、`max_fields` は `f > m`、`max_records` は `min(k,m)` 個の後 `LimitExceeded`（header は data record に数えない）。全て `DocumentError::limit` に option 名が載る |
| CSV streaming / cursor | `src/csv.rs` | `CsvReader` は record を入力順に 0-based data-record index で返し `record_index` が一致する。`read_typed` は失敗時も丁度 1 record を消費し cursor が進む（`DataModel` 後も後続 record が読める）。`flexible=false` で ragged は `DataModel`・`true` で受理。`field_str` は非 UTF-8 を `InvalidEncoding`。`delimiter == quote` dialect は `Unsupported` |
| CSV BOM | `src/csv.rs` | UTF-8 BOM は `StripUtf8`（default）で除去・`Preserve` で先頭 field byte として保持。UTF-16 系 BOM は構築時 `InvalidEncoding` |
| XML event reader | `src/xml.rs` | `read_event` は EOF で `Ok(None)` を安定に繰り返し、error 後は poison されて再 read も同 kind で失敗する。`max_depth`/`max_attributes`/`max_event_bytes`/`max_events` は各 named limit で `LimitExceeded`。secure default で DTD/doctype・非 UTF-8 宣言・非事前定義 general entity は `Unsupported`・数値 char reference は `Reference` event |

## Lane 構成

| Lane | 実行 | 環境要件 |
| --- | --- | --- |
| `mise run test-formal` | `cargo test --locked -p ruviform-ui --test formal_ipc` + `cargo test --locked -p ruviform-ui-test --test formal_transport` + `cargo test --locked -p ruviform-io-archive --test formal_archive` + `cargo test --locked -p ruviform-io-structured --all-features --test formal_structured` | なし（pure Rust・MSRV 内・CI と同一入口） |
| `mise run test-formal-kani` | `cargo kani --tests -p ruviform-ui -p ruviform-ui-test -p ruviform-io-archive` と `cargo kani --tests --all-features --no-unwinding-checks -p ruviform-io-structured` の 2 invocation（両方 pass でのみ `pass` — `default = []` の crate は別途 `--all-features` が必要なため分割。`--no-unwinding-checks` の根拠は Kani bound の節を参照） | ネイティブ `cargo kani`、または `cargo` + `cargo kani` を持つ WSL2 ディストロ |

`test-formal` は常時実行可能な第一層。`test-formal-kani` は
環境検出つきの第二層で、次の順に解決する。

1. ネイティブ `cargo kani` があればそれを使う。
2. なければ `wsl.exe -l -q` でディストロを列挙し、`cargo` と
   `cargo kani` の両方が PATH にあるものを先着で採用する。
3. カレントの Windows パスを `/mnt/<drive>/...` に写像し、WSL 側は
   `CARGO_TARGET_DIR=$HOME/.cache/ruviform-kani/target` の WSL
   ネイティブ領域で実行する（`/mnt` 越しのビルド IO を避ける）。
4. どちらも見つからなければ `not_run`（`environment_blocked`）を
   報告して exit 0 — 環境欠如は fail でも pass でもない。
   真の検証失敗（CBMC が反例を返した場合）のみ `fail` + exit 1。

## Kani の bound と仮定

証明は全入力ではなく bounded symbolic input に対する exhaustive
検証（CBMC の性質上、未証明領域は残る — 正直な bound 記録が必須）。

| Harness | Bound | 未証明領域 | 補完 |
| --- | --- | --- | --- |
| `parse_intent_error_contract` | envelope field（`v:u8`・`kind` 5 択・`id`/`op` 各 6 byte valid UTF-8・`params` 7 択 fragment）を固定 JSON skeleton 上で symbolic に全網羅。`#[kani::unwind(80)]` | 任意 malformed byte 列・1 MiB cap 経路（bound 内では構造的に到達不可） | malformed 入力は proptest `parse_intent_arbitrary_error_set`、cap 経路は `parse_intent_oversize_is_too_large`（1 MiB 実測）で担保 |
| `valid_op_soundness` | op = 8 byte 任意列のうち valid UTF-8。`#[kani::unwind(16)]` | 8 文字超の入力・`MAX_OP_LEN=128` をまたぐ受理/拒否 | proptest `valid_op_total_and_sound` / `valid_op_accepts_generated` で担保 |
| `event_to_json_wellformed` | id = 6 byte 任意列のうち valid UTF-8・`result_ok`・`params=Null`。`#[kani::unwind(80)]` | 他 variant・大きな params | proptest `event_to_json_versioned_envelope` が全 variant・任意 `Value` で担保 |
| `read_frame_contract` | stream = ≤32 byte 任意 byte 列。`#[kani::unwind(48)]` | 32 byte 超の stream・1 MiB 超 `declared` の drain 実走（symbolic u32 で `TooLarge` 判定自体は到達可） | proptest `read_frame_arbitrary_stream` / `too_large_drains_and_stays_aligned` で担保 |
| `write_read_roundtrip` | body = 16 byte 任意列のうち valid UTF-8。`#[kani::unwind(40)]` | in-cap だが 16 byte 超または無効 UTF-8 の body（`&str` 引数のため構造上到達不可） | proptest `write_read_roundtrip`（in-cap 任意・exact-cap 境界含む）で担保 |

### slice 2 archive harness（`tests/formal_archive.rs` の `#[cfg(kani)] mod proofs`）

slice 1 の教訓（任意 symbolic byte 列 × parser は CBMC unwind が非実用的）
を踏まえ、archive harness は「concrete な最小 valid skeleton 上の
symbolic gate field」のみを対象にする。任意 malformed stream は
proptest の concrete coverage に委譲する。

| Harness | Bound | 未証明領域 | 補完 |
| --- | --- | --- | --- |
| `tar_typeflag_classification` | 512 byte 固定 TAR header block（name は concrete、typeflag byte 156 のみ symbolic・全 256 値）。checksum は closed-form（`TAR_HEADER_BASE` + digit sum）で構築 | 複数 header・extension payload 中身・GNU/PAX 長名・数値 field 全体 | proptest `tar_arbitrary_stream_error_set` / `tar_pax_gnu_extensions` / roundtrip 系で担保 |
| `tar_declared_size_gate` | 同上 skeleton で declared size field を symbolic 化（bounded u64）| `max_entry_uncompressed_bytes` 境界±1 以外の組合せ・total gate との複合 | proptest `tar_declared_size_limits` / `tar_total_uncompressed_limit` で担保 |
| `zip_index_gates` | concrete ZIP skeleton（1 local header + 1 central record + EOCD）で EOCD count field を symbolic u16 | 複数 entry・ZIP64・deflate payload・local/central 不整合の全組合せ | proptest `zip_index_limit_gates` / `zip_arbitrary_stream_error_set` / `zip_writer_roundtrip` で担保 |

### slice 3 structured harness（`tests/formal_structured.rs` の `#[cfg(kani)] mod proofs` + `src/input.rs` の in-src proof）

slice 2 と同じ「concrete skeleton + symbolic gate field」型のみ。
任意 stream は proptest に委譲する。`BoundedRead` は `pub(crate)`
なので byte-budget の証明だけ `src/input.rs` 内の
`#[cfg(kani)] mod proofs` に置く（`cargo kani --tests` は lib target
の harness も test mode でカバーする）。

| Harness | Bound | 未証明領域 | 補完 |
| --- | --- | --- | --- |
| `bounded_read_budget_gate`（in-src） | concrete 6 byte input + symbolic `max_bytes:u64`（全値）を 2 byte/read の `read` loop で drive。`#[kani::unwind(16)]` | 任意長・任意 backend reader の組合せ・probe 対象 byte の値（`Read` 契約上 0/1 のみ効く） | proptest 各形式の `*_max_bytes_gate`（concrete skeleton + tail/offset）が reader 実経路で担保 |
| `csv_max_records_gate` | concrete skeleton `h\nr1\nr2\nr3\n` + symbolic `max_records:u64`。`#[kani::unwind(128)]` + `--no-unwinding-checks`（後述） | 任意 record 内容・他 limit との複合・4 record 超 | proptest `csv_max_records_gate` が任意 `k`/`m`・非 header 経路も担保 |
| `csv_max_field_bytes_gate` | concrete skeleton `a,bb\n` + symbolic `max_field_bytes:usize`。`#[kani::unwind(128)]` + `--no-unwinding-checks`（後述） | 任意 field 内容・複数 field 超過パターン・buffer 成長境界（`max_record_bytes` は concrete） | proptest `csv_max_field_bytes_gate` / `csv_max_record_bytes_gate` が `s`/`r`/`m` 全組合せで担保 |

仮定:

- `kani::assume(len <= STREAM_LEN)` で stream 長を bound 内に制限
  （それ以外は unconditional — 入力を絞る assume は契約を弱める
  ので置かない）。
- `#[kani::unwind(N)]` は library loop（UTF-8 検証・`Chars`・
  `Read`/`copy` 内部）の unwind 上限。bound 未指定では CBMC が
  symbolic 状態上の std loop を無制限 unwind して非実用的になる
  ことが実測で判明したため、入力サイズ＋余裕に設定する。証明は
  あくまでこの loop bound 内の主張であり、bound 超過は
  unwinding assertion として検出される。
- symbolic `&str` は `core::str::from_utf8(&bytes)` を `Ok` のときのみ
  使用する — allocation-free の固定長 byte 列から得るため
  `String` 生成の RawVec unwind コストを回避できる。`Ok` 以外を
  early return するのは入力型の制約であって契約弱化の `assume`
  ではない（`&str` 引数に invalid UTF-8 は型上到達不能）。
- CSV harness 固有: `CsvReader::new` が呼ぶ `csv_core` の
  `build_dfa` は `for c in 0..256` が dominant loop（完全証明には
  unwind >= 257 が必要）。skeleton の byte は全て < 128 であり、
  c >= 128 の DFA row は concrete 入力から到達不能なので、
  io-structured invocation は `#[kani::unwind(128)]` +
  `--no-unwinding-checks` で当該 loop を意図的に truncate する
  （全 256 row の unwind は式をほぼ 2 倍に膨らませ、参照 WSL2
  環境を枯渇させることを実測）。truncate 領域が到達不能である
  保証は harness の assert 自体が検査する — table が byte < 128
  で現実と乖離していれば concrete 期待値（field 内容・error
  kind）の assert が落ちる。それ以外の全 loop（NFA_STATES・
  `for _ in 0..4`・reader 内部）は bound 内で完全に網羅される。
- serde_json の float 復号精度: `float_roundtrip` feature は無効の
  ため、最短表現 decimal が parse 時に 1 ulp ずれる場合がある。
  proptest 側は `json_value_eq`（整数は厳密・float は 1 ulp 以内）
  で「wire 上の意味的同一性」を検証する。これは codec 契約の
  外側にある serde_json 既知の精度限界であり、契約違反ではない。

## Kani 環境のプロビジョニング

`test-formal-kani` が `not_run`（`environment_blocked`）を返す環境
では、次のいずれかを用意する。

- Linux / macOS: `rustup` + `cargo install --locked kani-verifier` +
  `cargo kani setup`（CBMC 等の solver は `kani-verifier` の
  setup が取得する）。
- Windows: WSL2 ディストロ内で同上。lane は `wsl.exe -l -q` で
  列挙した各ディストロに `cargo` + `cargo kani` があるかを先着
  検出し、カレント Windows パスを `/mnt/<drive>/` へ写像して
  実行する。ビルド成果物は WSL ネイティブ側
  （`CARGO_TARGET_DIR=$HOME/.cache/ruviform-kani/target`）に置き、
  `/mnt` 越しの遅い IO と Windows 側 target の混線を避ける。

solver 取得はネットワーク依存のため、CI で Kani を必須化する場合は
イメージ焼き込み等の別途検討が必要（本 slice の scope 外 — lane は
manual/opt-in）。

## Mutation-fail 証跡（「fail できること」の証明）

プロパティ suite は緑であることだけでなく、壊れた実装を検出できる
ことが証拠として必要。手順:

1. 対象関数の契約を 1 箇所だけ一時的に破る（例: `valid_op` の
   セグメント非空チェックを外す、`parse_intent` の `v` gate を
   緩める、`read_frame` の drain を省略する）。
2. `mise run test-formal`（または対象 `--test` 単独）を実行し、
   該当 property が shrink つきで `fail` することを記録する。
3. 変異を完全に revert し、緑に戻ったことを再実行で確認する。

変異・観測した fail・revert の 3 点セットを `evidence/rf806/` に
記録する。変異の持ち越し（revert 漏れ）は禁止 — verification.md に
revert 後の再検証結果を残す。slice 2 では例として TAR reader の
limit gate または ZIP reader の CRC/size 整合チェックを 1 箇所だけ
破り、`formal_archive` の該当 property が shrink つき fail する
ことを記録する。slice 3 では `BoundedRead` の probe 比較
（`remaining == 0`）を 1 箇所だけ破り、`formal_structured` の
max_bytes gate property が shrink つき fail することを記録する
（観測上の注意: probe 比較を早めに効かせる変異は、read 粒度が
細かい backend（serde_json の逐 byte trailing check）でのみ発火
する — 大量 fill する backend は `remaining` を 1 跨いで 0 まで
一読みで落とすため変異がマスクされる。ledger にこの観察を残す）。

## 判定語彙と fail-closed

`pass` / `fail` / `not_run` / `unsupported` / `blocked` — verify-
ruviform skill の語彙に従う。環境欠如は `not_run` +
`environment_blocked` で exit 0、証拠のない lane を `pass` と記録
しない。ローカル検証は変更範囲の検証であり、merge 後の対象 SHA
に対する必須 CI 結果が統合検証の証拠（本記録はそれを代替しない）。

## 昇格経路（本 slice 外）

Kani（bounded model checking）への次の昇格候補:

| 候補 target | 対象 crate / 契約 | Kani が向く理由 |
| --- | --- | --- |
| parser family | `ruviform-ui-theme`（`rvf-dtcg-profile.v1` の parse/validate → typed `ResolvedTheme`）、`ruviform-io-wav`（RIFF header） | wire codec と同型 — bounded symbolic byte 列に対し「panic なし・エラー分類が安定集合・受理なら構造契約を満たす」を尽きられる。`ruviform-io-archive`（tar/zip header・`limits` の上限 gate）は slice 2 で、`ruviform-io-structured`（JSON/YAML/TOML/CSV/XML reader・`InputLimits`/`BoundedRead` byte budget・`DocumentErrorKind` 安定集合・per-format named limit gate）は slice 3 で本形を適用済み |
| unsafe 局在コード | `ruviform-math`（SIMD dispatch）、`ruviform-audio-device` / `ruviform-midi-device` / `ruviform-observability` の `#[expect(unsafe_code)]` 局所箇所 | unsafe ブロックの前後条件・pointer/len 不変条件を symbolic に検証 — 通常 test では到達不能な入力経路をカバーできる（harness は unsafe 自体ではなく公開境界の契約を検証する設計にする） |
| DSP helper 算術 | `ruviform-dsp`（biquad/fft/fir/resample/window の係数・index・累積計算）、`ruviform-math` の scalar helper | 「オーバーフロー/除算ゼロ/index 範囲外が起きない」「精度 bound」等の算術不変条件は bounded symbolic 入力で尽きやすく、全サイズ証明が必要になった段階で Lean へ再昇格できる |

- ordering / state machine 系（session・dock・close 協議）は
  TLA+ 候補（`scarecrow173/ruviform#389` 系）。
- 全サイズの数学的不変条件（DSP・算術）は Lean 4 候補。
- 本 slice で確立した「coverage 記録 + sync commit + 1 コマンド
  再実行 + mutation-fail 証跡」の型を、そのまま昇格先にも適用する。
