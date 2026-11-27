---
id: "SPEC-rf806frmlvrf0slice1"
type: "spec"
status: "proposed"
title: "Issue #806 Formal Verification D 層 — proptest + Kani 検証基盤（slice 1: wire codec / slice 2: archive parser / slice 3: structured documents / slice 4: PCM WAV backend）"
created: "2026-11-20"
updated: "2026-11-26"
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
      - type: "file"
        path: "crates/ruviform-io-wav/tests/formal_wav.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/tests/formal_wav.proptest-regressions"
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
        path: "crates/ruviform-io-wav/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-wav/src/convert.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/src/wav.rs"
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

# Formal Verification — wire codec + archive + structured + WAV slice（rvf.ipc.v1 / rvf.uitest.v1 / tar / zip / json / yaml / toml / csv / xml / riff-wave）

> **整合性 note（2026-11-25）**: 本文書は Issue
> `scarecrow173/ruviform#806` で導入された個別 verification
> lane・Bound・証跡の inventory/spec である。現行の形式的検証方法論の
> 正本は `POLICIES.md` §6「形式的検証の原則・検証粒度・検証境界」
> であり、本書の `D 層` や `proptest → Kani → TLA+/Lean` という表現は
> 固定昇格路ではなく、rf#806 当時の導入経緯または個別 lane の説明
> として読む。新規検証では Property・リスク・必要保証から手法と境界を
> 選ぶ。
>
> `ruviform-io-structured` に対する `--no-unwinding-checks` は
> rf#1015 slice 3 で撤廃済みであり、旧結果は legacy-exception result
> として扱う。WAV の `cfg(kani)` 診断 payload 置換は未移行の
> legacy exception であり、`scarecrow173/ruviform#1015` の再評価対象である。
> これらを通常の Proof Pass の根拠や新規にコピーしてよい pattern
> として扱わない。

Issue `scarecrow173/ruviform#806` の実装 spec。slice 1 で unsafe を
含まない wire codec に proptest と Kani を個別 Property に対応する
complementary lane として常設し、slice 2 で同じ型を
`ruviform-io-archive` の TAR/ZIP reader と `ArchiveReadLimits` に、
slice 3 で `ruviform-io-structured` の JSON/YAML/TOML/CSV/XML reader
と共有 `InputLimits`/`BoundedRead` byte budget に、slice 4 で
`ruviform-io-wav` の RIFF/WAVE serialization・canonical planar
`f32`→wire sample 変換・muxer finalize/`WavReceipt` 契約に適用した。

## rf#1015 — Property ledger、legacy exception、wire/archive/structured redesign

rf#806 の harness inventory は後続の再設計で Property group 単位へ再編
された。再編後の group、risk、必要保証、仮定、未証明領域、verdict、予算、
選択再実行の台帳は `evidence/rf1015/property-ledger.md`、slice 1/2 の作業証跡は
`evidence/rf1015/verification.md`・`evidence/rf1015/verification-slice2.md`・
`evidence/rf1015/verification-slice3.md`・`evidence/rf1015/verification-slice4-archive.md`・
`evidence/rf1015/verification-slice5-archive-payload.md`・
`evidence/rf1015/verification-slice6-archive-typeflag.md`・
`evidence/rf1015/verification-slice7-frame-codec.md`・
`evidence/rf1015/verification-slice8-ipc-envelope.md`・
`evidence/rf1015/verification-slice9-zip-entry-size.md`・
`evidence/rf1015/verification-slice10-zip-output-cap.md`・
`evidence/rf1015/verification-slice11-zip-measured-total.md`・
`evidence/rf1015/verification-slice12-tar-measured-total.md`・
`evidence/rf1015/verification-slice13-tar-entry-progress.md`・`evidence/rf1015/runs.jsonl` であり、
以降の本書は rf#806 当時の Bound・unproven-area・complement table・
resource-caused `not_run` の inventory として残す。Property group と
legacy exception の判断では ledger を優先する。本書の表現と矛盾を見つけた
場合は、どちらが正しいかを確認して修正し、同期状態を記録する。

slice 1 で確定した設計方向は次のとおり。structured の D-1 は slice 3 で、
Archive 側代表 Property `ARC-IDX-LIMIT` は slice 4 で、`ARC-PAYLOAD-LIMIT`
の TAR declared size gate は slice 5 で、`ARC-TAR-STRUCT` の typeflag
classification は slice 6 で、`ARC-PAYLOAD-LIMIT` の ZIP declared entry-size gateは slice 9 で、
ZIP entry output hard-cap / cap-overflow classification は slice 10 で、ZIP measured total budget accumulation は slice 11 で、TAR measured total read-window / accumulation は slice 12 で、TAR entry progress / read-demand composition は slice 13 で実施済みだが、WAV・他の wire/archive Property・
read_frame・SampleFormat の方向は後続実装 slice の採用方針である。

1. structured documents では crate 全体への `--no-unwinding-checks` を
   撤廃する。最終形は CSV limit decision を `csv_core` の DFA/reader
   shell から分離した pure decision core で証明する。移行中の proof 単位
   限定は許容するが、crate 単位例外の存続は採らない。
2. WAV の `cfg(kani)` 診断置換は撤廃し、kind 判定の pure core と診断付与
   shell を分離する lazy/pure diagnostic boundary を採用する。診断文面は
   wire 契約に含めず、未証明領域として明示する。
3. verdict を返せない wire/archive 8 harness は、現行形の予算拡大ではなく
   pure decision core 抽出と Property 単位分割で再設計する。複合 parser
   shell の証明が依然として支配的な場合は、その部分だけ proptest への
   downgrade または引退を ledger へ記録する。
4. `read_frame` EOF は製品 rustdoc を正とする。length prefix の 1〜3 byte
   部分 EOF は clean EOF ではなく `UnexpectedEof` とする契約差分を後続
   製品 slice へ分離する。この slice では実装・テスト oracle を変更しない。
5. `SampleFormat` の手動 `%10` 列挙は、audio-core 側の public canonical
   variant inventory（`SampleFormat::ALL` 相当）と exhaustive probe に
   置き換え、variant 追加時に compile error で検出する方向を採用する。
   公開 API 追加の承認と命名は後続 slice で確定する。

slice 2 では、代表 important Property のうち `W-IPC-OP` だけを実装した。
`valid_op(&str)` は crate-private な `valid_op_bytes(&[u8])` へ処理を委ねる
薄い wrapper になり、grammar 判定は allocation・UTF-8/`char` 走査・I/O を
持たない byte predicate に閉じる。proof は製品実装と同じ `src/ipc.rs` の
`#[cfg(kani)] mod proofs` に置く。これは proof placement のための cfg であり、
製品挙動を `cfg(kani)` で置き換える例外ではない。

rf#1015 slice 4 では、Archive 側代表 Property のうち `ARC-IDX-LIMIT` を
実装した。`ZipArchiveReader::new` は EOCD の declared central-directory
count/size と `ArchiveReadLimits` だけを受け取る crate-private な
`zip_index_gate_decision` を central directory buffering の前に呼ぶ。core は
allocation・I/O・`Read`/`Seek`・EOCD/CD parser・Vec buffering・診断構築を
持たない。proof は製品実装と同じ `src/zip/read.rs` の `#[cfg(kani)] mod proofs`
に置き、parser ではなく production decision core の symbolic exactness を検証する。
parser が core を呼ぶ接続条件、複数 entry、ZIP64、deflate payload、local/central
不整合、実 CD parse、overlap audit は Kani 未証明領域として残り、既存 proptest が
接続 coverage を担う。

rf#1015 slice 5 では、Archive 側代表 Property のうち `ARC-PAYLOAD-LIMIT` の
TAR declared size gate を実装した。`TarArchiveReader` は PAX size overrideを
foldした effective declared sizeと `max_entry_uncompressed_bytes` だけを受け取る
crate-private な `tar_declared_size_gate_decision` を payload byteを読む前に呼ぶ。
core は allocation・I/O・`Read`・TAR header parser・PAX/GNU extension folding・
Vec buffering・診断構築を持たない。proof は製品実装と同じ
`src/tar/read.rs` の `#[cfg(kani)] mod proofs` に置き、production decision core の
symbolic exactness を検証する。parser が core を呼ぶ接続条件、TAR header parse、
PAX/GNU override、任意 malformed stream、payload streaming、measured total
uncompressed budget、ZIP entry size gate、decompression は Kani 未証明領域として残り、
既存 proptest が接続 coverage を担う。

rf#1015 slice 6 では、Archive 側代表 Property のうち `ARC-TAR-STRUCT` の
TAR typeflag classification を実装した。`TarArchiveReader` は header typeflag
byte だけを受け取る crate-private な `tar_typeflag_class` を entry dispatchの
前に呼び、local/global PAX、GNU longname/longlink、surface kind、unsupported を
一意に分類する。core は allocation・I/O・TAR header parser・checksum・extension
payload・診断構築を持たない。proof は製品実装と同じ `src/tar/read.rs` の
`#[cfg(kani)] mod proofs` に置き、production decision core を independent oracleと
比較して全 256 byte値の exactnessを検証する。parser が core を呼ぶ接続条件、
PAX/GNU extension folding、複数 header、任意 malformed stream、数値 field 全体は
Kani 未証明領域として残り、connection regression と既存 proptest が接続 coverageを担う。

rf#1015 slice 7 では、Wire 側代表 Property のうち `W-FRM-CODEC` の
length-prefix/write-cap decision subboundary を実装した。`write_frame` と
`read_frame` は frame body length、`MAX_INTENT_BYTES`、4 byte の wire prefix
だけを受け取る crate-private な decision core から accept/reject、u32le
length prefix、declared length、in-cap/TooLarge 判定を導く。core は
allocation・I/O・`Read`/`Write`・Vec growth・UTF-8 validation・environment
access・診断構築を持たない。proof は製品実装と同じ `src/transport.rs` の
`#[cfg(kani)] mod proofs` に置き、symbolic `usize` body length、symbolic
`[u8; 4]` prefix、accepted length の encode→decode roundtripを検証する。
body byte transport、UTF-8 validation、EOF/truncation 分類、oversize drain、
Vec/Read/Write shell、実 OS pipe/TCP、shell-core接続は Kani 未証明領域として残り、
既存 proptest と deterministic boundary regression が接続 coverageを担う。

rf#1015 slice 8 では、Wire 側代表 Property のうち `W-IPC-ENVELOPE` の
body size cap と post-parse envelope field gate の decision subboundaryを実装した。
`parse_intent` は raw body byte length だけを受け取る crate-private な
`intent_body_size_decision` で JSON parse 前の cap accept/rejectを導き、serde_jsonで
parse済みの `v`、`kind`、`op` だけを受け取る crate-private な
`intent_envelope_gate_decision` から Accept / unsupported version / unsupported kind /
invalid op を現行優先順位で導く。core は allocation・I/O・serde_json・JSON parse・
`String`/`Value` 構築・Vec growth・UTF-8 validation・environment access・診断構築を
持たない。proof は製品実装と同じ `src/ipc.rs` の `#[cfg(kani)] mod proofs` に置き、
symbolic body length/version、len 0〜8に分割した kind/op byte window、fixed length 6の
kind content、concrete 127/128/129 byte op boundary、version→kind→op priorityを検証する。
serde_json parser correctness、任意 malformed input、JSON object typing、missing/extra/
duplicate fields、`id`/`params` payload identity、float 厳密性、ErrorBody message構築、
shell-core接続は Kani 未証明領域として残り、既存 proptestと deterministic regressionが
接続 coverageを担う。旧 shell 向け `parse_intent_error_contract` harnessは退役した。

rf#1015 slice 9 では、Archive 側代表 Property のうち `ARC-PAYLOAD-LIMIT` の
ZIP declared entry-size gate decision subboundary を実装した。`ZipArchiveReader::open_entry_inner`
は central directory record の declared uncompressed sizeと `max_entry_uncompressed_bytes`
だけを受け取る crate-private な `zip_declared_entry_size_gate_decision` を local header decodeの前に呼ぶ。
core は allocation・I/O・`Read`/`Seek`・EOCD/CD/local-header parser・Deflate/zlib・Vec buffering・CRC・
environment access・診断/error message構築を持たない。proof は製品実装と同じ
`src/zip/read.rs` の `#[cfg(kani)] mod proofs` に置き、symbolic `u64` declared sizeと
limitに対して over-limitのみ Reject、boundary equalityを含む in-capを Acceptとする exactnessを検証する。
production inputは classic ZIP recordの `u32` wideningだが、proof inputを `u64` 全体にしたのは
production domainを含む stronger proofであり、入力域を縮めていない。shell-core接続、ZIP EOCD/CD/
local-header parse、measured total budget、entry output hard cap、cap overflow classification、
decompression、CRC、local/central consistency、overlap、任意 malformed streamは Kani 未証明領域として残り、
deterministic regressionと既存 proptestが接続 coverageを担う。退役すべき旧 ZIP declared entry-size
shell harnessはない。

rf#1015 slice 10 では、Archive 側代表 Property のうち `ARC-PAYLOAD-LIMIT` の
ZIP entry output hard-cap / cap-overflow classification decision subboundary を実装した。
`ZipArchiveReader::open_entry_inner` は declared uncompressed sizeと `verify_all` から渡される
remaining total budget inputだけを受け取る crate-private な `zip_entry_output_cap_decision` から
hard capと overflow classificationを導き、`ZipEntryReader::probe` は同じ decision値を error kindへ写像する。
standalone openでは capが declared size、budget inputがある場合は capが `min(declared, remaining)`、
overflowは remainingが declared未満の場合だけ budget overflow、equalityまたは超過および budgetなしでは
declared-overproductionである。core は allocation・I/O・`Read`/`Seek`・EOCD/CD/local-header parser・
Deflate/zlib・Vec buffering・CRC・environment access・診断/error message構築を持たない。proof は製品実装と同じ
`src/zip/read.rs` の `#[cfg(kani)] mod proofs` に置き、symbolic `u64` declared size、symbolic remaining budget、
symbolic boolから構成した `None`/`Some` 全 domainに対して hard capと classificationの exactnessを検証する。
この encoding は input narrowingではなく full-domain encodingである。shell-core接続、ZIP EOCD/CD/local-header parse、
measured total budget accumulation、decompression、CRC、local/central consistency、overlap、任意 malformed streamは
Kani 未証明領域として残り、deterministic regressionと既存 proptestが接続 coverageを担う。退役すべき旧 ZIP output cap
shell harnessはない。

rf#1015 slice 11 では、Archive 側代表 Property のうち `ARC-PAYLOAD-LIMIT` の
ZIP measured total budget accumulation decision subboundary を実装した。`ZipArchiveReader::verify_all`
は `max_total_uncompressed_bytes` と accumulated totalだけから remaining total budgetを導く
`zip_measured_total_remaining`、および accumulated totalと measured entry bytesだけから次の accumulated
totalを導く `zip_measured_total_accumulate` を使う。core は allocation・I/O・`Read`/`Seek`・EOCD/CD/local-header parser・
Deflate/zlib・Vec buffering・CRC・environment access・診断/error message構築を持たない。proof は製品実装と同じ
`src/zip/read.rs` の `#[cfg(kani)] mod proofs` に置き、3 つの symbolic `u64` の全 domainに対して zero saturationと
`u64::MAX` saturationを含む exactnessを assumeなしで検証する。production invariant `accumulated_before <=
max_total_uncompressed_bytes` は assumeせず、全 domain proofは production reachable subsetを含む stronger proofである。
shell-core接続、ZIP EOCD/CD/local-header parse、decompression、CRC、local/central consistency、overlap、任意 malformed stream、
複数 entry composition、error priorityは Kani 未証明領域として残り、deterministic regressionと既存 proptestが接続 coverageを担う。
退役すべき旧 ZIP measured total shell harnessはない。

rf#1015 slice 12 では、Archive 側代表 Property のうち `ARC-PAYLOAD-LIMIT` の
TAR measured total read-window / accumulation decision subboundary を実装した。
`TarEntryReader::read` と `TarArchiveReader::drain_bytes` は、total budgetと
accumulated totalだけから remaining total budgetを導く `tar_measured_total_remaining`、
remaining budget・nonzero demand・caller bufferまたは drain scratch capacityだけから
1 回の windowと budget exhaustionを導く `tar_measured_total_window_decision`、および
accumulated totalと measured bytesだけから次の accumulated totalを導く
`tar_measured_total_accumulate` を共有する。core は allocation・I/O・`Read`・TAR header parser・
checksum・PAX/GNU extension folding・padding・pending state machine・environment access・
診断/error message構築を持たない。proof は製品実装と同じ `src/tar/read.rs` の
`#[cfg(kani)] mod proofs` に置き、5 つの symbolic `u64` の全 domainに対して zero saturation、
window minimum、budget exhaustion、zero-demand no-read、`u64::MAX` accumulation saturationを
assumeなしで検証する。production invariant `accumulated_before <= max_total_uncompressed_bytes`
は assumeせず、全 domain proofは production reachable subsetを含む stronger proofである。
`capacity == 0` は scalar exactnessでは window 0・non-exhaustionだが、production接続では
entry read shellの empty-buffer early returnまたは drain固定 nonzero capacityにより到達しない。
shell-core接続、TAR measured total streaming composition、複数 entry composition、pending state machine、
TAR header parse、PAX/GNU extension folding、任意 malformed stream、decompression、CRC、error priorityは
Kani 未証明領域として残り、deterministic regressionと既存 proptestが接続 coverageを担う。
退役すべき旧 TAR measured total shell harnessはない。

rf#1015 slice 13 では、`ARC-PAYLOAD-LIMIT` の TAR entry progress / read-demand composition decision subboundaryを実装した。`TarEntryReader::read`、`discard_remaining`、`Drop`、`finish` は declared/producedだけから remaining entry payloadを導く `tar_entry_remaining_payload`、entry残量・total budget残量・caller buffer capacityだけから 1 回の read window・budget exhaustion・entry exhaustionを導く `tar_entry_read_window_decision`、producedと read bytesだけから次の producedを導く `tar_entry_accumulate_produced`、declared/producedだけから exhaustionを導く `tar_entry_payload_exhausted` を共有する。composition decisionは entry残量を demandとして slice 12の `tar_measured_total_window_decision` へ渡す合成であり、window計算を再実装しない。core は allocation・I/O・`Read`・TAR header parser・checksum・PAX/GNU extension folding・padding・pending state machine・environment access・診断/error message構築を持たない。proof は製品実装と同じ `src/tar/read.rs` の `#[cfg(kani)] mod proofs` に置き、5 つの symbolic `u64` の全 domainに対して remaining saturation、exhaustion equality、zero-demand no-read、budget exhaustion、window minimum、produced accumulation saturationを assumeなしで検証する。production invariant `produced <= declared` は assumeせず、全 domain proofは production reachable subsetを含む stronger proofである。`capacity == 0` は scalar exactnessでは window 0・non-exhaustionだが、production接続では entry read shellの empty-buffer early returnにより到達しない。shell-core接続、TAR streaming compositionの残部、複数 entry composition、pending/discard/finish state machine、TAR header parse、PAX/GNU extension folding、任意 malformed stream、TAR metadata/path/link gate、decompression、CRC、error priorityは Kani 未証明領域として残り、deterministic regressionと既存 proptestが接続 coverageを担う。退役すべき旧 TAR entry progress shell harnessはない。

選択再実行は `mise run test-formal-kani w-ipc-op` / `w-ipc-envelope` に加え、slice 3 の
`sd-csv-limits` / `sd-byte-budget`、slice 4 の `arc-idx-limit`、slice 5 の
`arc-payload-limit`、slice 6 の `arc-tar-struct`、slice 7 の `w-frm-codec`、slice 8 の `w-ipc-envelope`、
slice 9 の `arc-zip-entry-size`、slice 10 の `arc-zip-output-cap`、slice 11 の `arc-zip-measured-total`、slice 12 の `arc-tar-measured-total`、slice 13 の `arc-tar-entry-progress` が最小入口である。
selector なしの `test-formal-kani` は従来の aggregate 挙動を維持し、未知 selector は
Kani 実行前に exit 2 の usage error で止まる。Property group 全件を対象にした
generic runner はまだない。`W-IPC-OP`, `W-IPC-ENVELOPE`, `W-FRM-CODEC`, `ARC-IDX-LIMIT`, `ARC-PAYLOAD-LIMIT`, `ARC-TAR-STRUCT`,
`SD-CSV-LIMITS`, `SD-BYTE-BUDGET` の選択再実行は `synced`、他 group は引き続き `not_synced`
または `deferred` と記録する。実装済みの bound・実測コスト・mutation 証跡は
ledger §6 と slice 2/3/4/5/6/7/8/9/10/11/12/13 の evidence を正本とする。

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

### slice 4 — PCM WAV backend（`ruviform-io-wav`）

| 対象 | ファイル | 契約 |
| --- | --- | --- |
| `Wire::new` / `wav_capabilities` | `src/wav.rs` | 受理する `SampleFormat` はちょうど wire 集合 `{U8,S16,S24,S32,F32}` — それ以外は `Unsupported`。`channels`/`block_align` は `u16` 範囲、`byte_rate = rate * block_align` は `u32` 範囲 — 超過は `InvalidValue`。`extensible` は channel mask が定義済み bitmask に収まらない layout でのみ立つ |
| `fmt_chunk_body` / `header` | `src/wav.rs` | `WAVEFORMATEX`（16 byte）か `WAVEFORMATEXTENSIBLE`（40 byte・`cbSize=22`・valid bits・speaker mask・subtype GUID）かは `extensible` で一意。header は `RIFF`+size placeholder+`WAVE`+`fmt `+`data`+size placeholder — open 時の size field はゼロで `data` size offset が backpatch 用に返る |
| `write_sample`（canonical `f32`→wire） | `src/convert.rs` | `NaN` は scaling 前に silence（0）。`±Inf`・範囲外は `[-1,1]` に clamp して飽和。整数形式は全範囲 scaling + `f32::round`（half away from zero）: U8 `x*128+128`・S16 `x*32768`・S24 `x*8388608`・S32 `x*2147483648` — `-1.0` は負の full-scale、`+1.0` は正の full-scale −1 LSB。F32 は IEEE-754 LE を無変換で書く。dither なし |
| encoder / packet stream | `src/encoder.rs` / `src/lib.rs` | `encode` は planar `f32` frame を sample 数そのままの packet へ変換し、timestamp/duration は frame のものを保持する。wire 外 format・channel/rate 不一致は `InvalidData` で reject し encoder は再利用可能（非 sticky） |
| muxer finalize / `WavReceipt` | `src/muxer.rs` / `src/lib.rs` | `finalize` は RIFF/data size を backpatch して `WavReceipt{bytes_written, frames, duration}` を 1 度だけ返す — 2 度目の `finalize`/`write_packet` は `InvalidData`/`InvalidState`。write 済み packet の frame 数・byte 数が receipt の値と一致する |
| round-trip | `tests/` + `ruviform-io-symphonia` | muxer が書いた byte 列は Symphonia 側が RIFF/WAVE として decode し、format 宣言と frame 内容が復元される |

## Lane 構成

| Lane | 実行 | 環境要件 |
| --- | --- | --- |
| `mise run test-formal` | `cargo test --locked -p ruviform-ui --test formal_ipc` + `cargo test --locked -p ruviform-ui-test --test formal_transport` + `cargo test --locked -p ruviform-io-archive --test formal_archive` + `cargo test --locked -p ruviform-io-structured --all-features --test formal_structured` + `cargo test --locked -p ruviform-io-wav --test formal_wav` | なし（pure Rust・MSRV 内・CI と同一入口） |
| `mise run test-formal-kani` | selector なし。`cargo kani --tests -p ruviform-ui -p ruviform-ui-test -p ruviform-io-archive` と `cargo kani --tests --all-features -p ruviform-io-structured` と `cargo kani --tests -p ruviform-io-wav` の 3 invocation（全て pass でのみ `pass` — `default = []` の crate は別途 `--all-features` が必要なため分割） | ネイティブ `cargo kani`、または `cargo` + `cargo kani` を持つ WSL2 ディストロ |
| `mise run test-formal-kani w-ipc-op` | `cargo kani -j1 --output-format terse -p ruviform-ui --harness w_ipc_op` の 1 invocation。`src/ipc.rs` の `w_ipc_op_exactness_len0_16` と 3 件の `w_ipc_op_boundary_len*` だけを選び、Unwinding Assertions を無効化しない。未知 selector は Kani 実行前に exit 2 | 同上 |
| `mise run test-formal-kani w-ipc-envelope` | `cargo kani -j1 --output-format terse -p ruviform-ui --harness w_ipc_envelope` の 1 invocation。`src/ipc.rs` の 10 件の `w_ipc_envelope_*` decision-core proof だけを選び、Unwinding Assertions を無効化しない | 同上 |
| `mise run test-formal-kani w-frm-codec` | `cargo kani -j1 --output-format terse -p ruviform-ui-test --harness w_frm_codec` の 1 invocation。`src/transport.rs` の 3 件の frame length decision proof だけを選び、Unwinding Assertions を無効化しない | 同上 |
| `mise run test-formal-kani arc-idx-limit` | `cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_idx_limit` の 1 invocation。`src/zip/read.rs` の `arc_idx_limit_decision_exactness` だけを選び、Unwinding Assertions を無効化しない | 同上 |
| `mise run test-formal-kani arc-payload-limit` | `cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_payload_limit` の 1 invocation。`src/tar/read.rs` の `arc_payload_limit_declared_exactness` だけを選び、Unwinding Assertions を無効化しない | 同上 |
| `mise run test-formal-kani arc-zip-entry-size` | `cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_zip_entry_size` の 1 invocation。`src/zip/read.rs` の `arc_zip_entry_size_declared_gate_exactness` だけを選び、Unwinding Assertions を無効化しない | 同上 |
| `mise run test-formal-kani arc-zip-output-cap` | `cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_zip_output_cap` の 1 invocation。`src/zip/read.rs` の `arc_zip_output_cap_decision_exactness` だけを選び、Unwinding Assertions を無効化しない | 同上 |
| `mise run test-formal-kani arc-zip-measured-total` | `cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_zip_measured_total` の 1 invocation。`src/zip/read.rs` の `arc_zip_measured_total_decision_exactness` だけを選び、Unwinding Assertions を無効化しない | 同上 |
| `mise run test-formal-kani arc-tar-measured-total` | `cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_tar_measured_total` の 1 invocation。`src/tar/read.rs` の `arc_tar_measured_total_decision_exactness` だけを選び、Unwinding Assertions を無効化しない | 同上 |
| `mise run test-formal-kani arc-tar-entry-progress` | `cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_tar_entry_progress` の 1 invocation。`src/tar/read.rs` の `arc_tar_entry_progress_decision_exactness` だけを選び、Unwinding Assertions を無効化しない | 同上 |
| `mise run test-formal-kani arc-tar-struct` | `cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_tar_struct` の 1 invocation。`src/tar/read.rs` の `arc_tar_struct_typeflag_exactness` だけを選び、Unwinding Assertions を無効化しない | 同上 |
| `mise run test-formal-kani sd-csv-limits` | `cargo kani -j1 --output-format terse --all-features -p ruviform-io-structured --harness csv_limit_decision` の 1 invocation。CSV limit decision core の 3 proofだけを選び、Unwinding Assertions を無効化しない | 同上 |
| `mise run test-formal-kani sd-byte-budget` | `cargo kani -j1 --output-format terse --all-features -p ruviform-io-structured --harness bounded_read_budget_gate` の 1 invocation。Unwinding Assertions を無効化しない | 同上 |

`test-formal` は常時実行可能な pure Rust lane、`test-formal-kani` は
環境検出つきの bounded model checking lane であり、どちらも既存 Property
に対して選ばれた complementary lane である。`test-formal-kani` は次の順に
環境を解決する。

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
| `w_ipc_envelope_size_exactness`（in-src `src/ipc.rs`） | body length `usize` を全て symbolic。cap accept/reject exactness。`#[kani::unwind(2)]`。assumeなし | serde_json parse、body bytes、shell-core接続 | deterministic `envelope_gate_connection_regression` と proptest `parse_intent_oversize_is_too_large` |
| `w_ipc_envelope_version_exactness`（in-src `src/ipc.rs`） | version `u32` を全て symbolicし、valid kind/op固定で version gate exactness。`#[kani::unwind(10)]` | 他 fieldとの複合違反、JSON typing | priority proof、deterministic regression、proptest |
| `w_ipc_envelope_kind_*`（in-src `src/ipc.rs`） | length precheckは symbolic `usize`、fixed length 6 contentは symbolic `[u8; 6]`、full kind exactnessは symbolic len 0〜8。len 0〜8 は入力域分割の `assume`。`#[kani::unwind(10)]` | len 9以上の任意長 content、JSON string typing | length precheck proof、deterministic regression、proptest |
| `w_ipc_envelope_op_exactness_len0_8` / boundary proofs（in-src `src/ipc.rs`） | len 0〜8 は入力域分割の `assume`付き exactness。127/128/129は concrete boundary matrix。`#[kani::unwind(10)]` / 131 / 132 | 17〜126 byte interior、129 byte超任意長、grammar大入力変種 | `W-IPC-OP` proof、proptest `valid_op_*`、deterministic regression |
| `w_ipc_envelope_gate_priority`（in-src `src/ipc.rs`） | symbolic versionと len 0〜8 kind/opで version→kind→op priority exactness。`#[kani::unwind(12)]` | len 9以上の field、serde_json parse、ErrorBody message構築、shell-core接続 | deterministic regression、proptest |
| `w_ipc_op_exactness_len0_16`（in-src `src/ipc.rs`） | byte slice 長 0〜16 を symbolic に全網羅し、製品 `valid_op_bytes` を独立 reference predicate と比較。`len <= 16` は入力域分割の `assume`。`#[kani::unwind(20)]` | 17〜126 byte の interior、`MAX_OP_LEN` 境界、128 byte 超 | boundary matrix と proptest `valid_op_total_and_sound` / `valid_op_accepts_generated` で担保 |
| `w_ipc_op_boundary_len{127_valid,128_valid,129_over_cap}`（in-src `src/ipc.rs`） | concrete 長 127/128/129。127・128 は valid witness、129 は他条件を満たす入力で cap 拒否を検査。`#[kani::unwind(131)]`・`#[kani::unwind(132)]`・`#[kani::unwind(132)]` | 17〜126 byte の interior、129 byte を超える任意長、grammar 個別条件の大入力変種 | grammar 個別条件は長 0〜16 の exactness proof、大入力と arbitrary Unicode は proptest で担保 |
| `event_to_json_wellformed` | id = 6 byte 任意列のうち valid UTF-8・`result_ok`・`params=Null`。`#[kani::unwind(80)]` | 他 variant・大きな params | proptest `event_to_json_versioned_envelope` が全 variant・任意 `Value` で担保 |
| `read_frame_contract` | stream = ≤32 byte 任意 byte 列。`#[kani::unwind(48)]` | 32 byte 超の stream・1 MiB 超 `declared` の drain 実走（symbolic u32 で `TooLarge` 判定自体は到達可） | proptest `read_frame_arbitrary_stream` / `too_large_drains_and_stays_aligned` で担保 |
| `w_frm_codec_write_length_exactness`（in-src `src/transport.rs`） | body length `usize` を全て symbolic。loopを持たず `#[kani::unwind(2)]`。assumeなし | body byte transport、write/read shell接続、UTF-8 validation、実 writer | proptest `write_read_roundtrip` / `write_frame_over_cap_refused` と deterministic `frame_length_boundary_connection_regression` で担保 |
| `w_frm_codec_prefix_exactness`（in-src `src/transport.rs`） | wire prefix `[u8; 4]` を全て symbolic。loopを持たず `#[kani::unwind(2)]`。assumeなし | body byte transport、EOF/truncation 分類、oversize drainの実 I/O、UTF-8 validation | proptest `read_frame_arbitrary_stream` / `too_large_drains_and_stays_aligned` と deterministic boundary regression で担保 |
| `w_frm_codec_length_roundtrip`（in-src `src/transport.rs`） | body length `usize` を全て symbolicし、accepted lengthの encode→decodeを検査。loopを持たず `#[kani::unwind(2)]`。assumeなし | body bytesそのものの roundtrip、Vec/Read/Write shell、UTF-8 validation、実 OS transport | proptest `write_read_roundtrip` と deterministic boundary regression で担保 |

旧 `valid_op_soundness`（8 byte の valid UTF-8 `&str` を対象とする soundness
harness）は rf#1015 slice 2 で退役した。新保証は byte 長 0〜16 の exactness と
127/128/129 の concrete boundary matrix（長さごとに分割）に分かれる。旧保証より小さい入力域では
受理だけでなく拒否の exactness まで検査するが、17〜126 byte と 129 byte 超の
任意長は引き続き未証明領域である。

旧 shell 向け `parse_intent_error_contract` harness は rf#1015 slice 8 で退役した。
旧保証は固定 JSON skeleton上の envelope fieldを serde_json parser shellごと検査しようとする
attempted proofであり、rf806では約45 CPU-min・3.7 GiBで verdictを返せなかった。新保証は
size cap と post-parse envelope field gateの allocation-free decision coreに限定される。
serde_json parser correctness、任意 malformed input、object typing、payload identity、float厳密性、
ErrorBody message構築、shell-core接続はこの proofの保証に含めず、proptest・deterministic
regression・統合テストと未証明領域に残す。`event_to_json_wellformed` は `W-IPC-SERDE` の
legacy 状態を表す harnessなので、この sliceでは退役も証明済み扱いもしない。

旧 shell 向け `write_read_roundtrip` harness は rf#1015 slice 7 で退役した。
旧保証は 16 byte bodyを Vec/Cursor/UTF-8 shellごと検査しようとする attempted proofであり、
rf806では約90 CPU-min・5.4 GiBで verdictを返せなかった。新保証は length-prefix/write-cap
decision coreの scalar exactnessに限定される。frame body bytesの輸送、UTF-8 validation、
EOF/truncation 分類、oversize drain、Vec/Read/Write shell、実 OS transportはこの proofの
保証に含めず、proptest・deterministic regression・統合テストと未証明領域に残す。
`read_frame_contract` は `W-FRM-READCLASS` / `W-FRM-OVERSIZE` の legacy 状態を表す
harnessなので、この sliceでは退役も証明済み扱いもしない。

### slice 2 archive harness と rf#1015 archive decision proofs

slice 1 の教訓（任意 symbolic byte 列 × parser は CBMC unwind が非実用的）
を踏まえ、残存する parser-facing archive harness は「concrete な最小 valid skeleton 上の
symbolic gate field」のみを対象にする。任意 malformed stream は
proptest の concrete coverage に委譲する。`ARC-IDX-LIMIT` の形式的証明は
rf#1015 slice 4 で `src/zip/read.rs` の pure decision core exactnessへ移り、
`ARC-PAYLOAD-LIMIT` の TAR declared size gate は slice 5 で、
`ARC-TAR-STRUCT` の typeflag classification は slice 6 でそれぞれ
`src/tar/read.rs` の pure decision core exactnessへ移り、`ARC-PAYLOAD-LIMIT` の
ZIP declared entry-size gateは slice 9で、ZIP entry output hard-cap / cap-overflow
classificationは slice 10で `src/zip/read.rs` の pure decision core exactnessへ移った。

旧 `zip_index_gates` harness は rf#1015 slice 4 で退役した。旧保証は concrete ZIP
skeleton（1 local header + 1 central record + EOCD）上の symbolic budget を
EOCD scan・CD parse・Vec bufferingごと検査しようとする attempted proofであり、
rf806では約87 CPU-min・7.3 GiBの solver-boundで verdictを返せなかった。新保証は
`arc_idx_limit_decision_exactness` が production decision coreの symbolic scalar
exactnessを Unwinding Assertions 有効で示すことである。EOCD scan、CD parse、
Vec allocation、decompression、parser skeleton、parser-core接続条件、複数 entry、
ZIP64、deflate payload、local/central 不整合、overlap audit はこの proofの保証に
含めない。parser-core接続と任意 malformed streamは proptest `zip_index_limit_gates` /
`zip_metadata_limit_gate` / `zip_arbitrary_stream_error_set` / `zip_writer_roundtrip`
に委譲する。

旧 parser-facing `tar_declared_size_gate` harness は rf#1015 slice 5 で退役した。
旧保証は concrete TAR skeletonと header parser shellを含む attempted proofで
あり、rf806では約19 CPU-min・約21.8 GiBまで膨張して verdictを返せなかった。
同じ rf806では declared-size boundary mutant `>`→`>=` が proptestに検出されず、
boundary density gap として残っていた。新保証は
`arc_payload_limit_declared_exactness` が production decision coreの symbolic scalar
exactnessを Unwinding Assertions 有効で示すことである。TAR header parse、PAX/GNU
override、任意 malformed stream、payload streaming、measured total budget、ZIP entry
size gate、decompression、parser-core接続条件はこの proofの保証に含めない。
parser-core接続と任意 malformed streamは proptest `tar_declared_size_gate` /
`tar_total_measured_gate` / `tar_arbitrary_stream_error_set` / roundtrip 系に委譲する。
旧 harness が verdictを返せなかった経緯と mutation 未検出の履歴は削除しない。

rf#1015 slice 9 の ZIP declared entry-size gateでは、退役する旧 shell harnessはない。新保証は
`arc_zip_entry_size_declared_gate_exactness` が production decision coreの symbolic scalar
exactnessを Unwinding Assertions 有効で示すことである。local header decode前の declared size
Accept/Rejectだけを保証し、EOCD/CD/local-header parse、measured total budget、entry output hard cap、
cap overflow classification、decompression、CRC、local/central consistency、overlap audit、ZIP64、
multi-disk、任意 malformed stream、shell-core接続条件はこの proofの保証に含めない。shell-core接続は
`zip_declared_entry_size_connection_regression`、parser shellと roundtrip保証は既存
`zip_entry_size_limit_gate` / `zip_total_measured_gate` / `zip_writer_roundtrip` 系に委譲する。

rf#1015 slice 10 の ZIP entry output hard-cap / cap-overflow classificationでは、退役する旧 shell harnessはない。
新保証は `arc_zip_output_cap_decision_exactness` が production decision coreの symbolic scalar exactnessを
Unwinding Assertions 有効で示すことである。budgetなしでは capが declared size、budgetありでは capが
`min(declared, remaining)`、overflowは remainingが declared未満の場合だけ budget側、equalityまたは超過および
budgetなしでは declared-overproduction側であることだけを保証する。EOCD/CD/local-header parse、measured total
budget accumulation、decompression、CRC、local/central consistency、overlap audit、ZIP64、multi-disk、任意 malformed
stream、shell-core接続条件はこの proofの保証に含めない。shell-core接続は `zip_output_cap_connection_regression`、
parser shellと roundtrip保証は既存 `zip_entry_size_limit_gate` / `zip_total_measured_gate` / `zip_writer_roundtrip`
系に委譲する。

rf#1015 slice 11 の ZIP measured total budget accumulationでは、退役する旧 shell harnessはない。
新保証は `arc_zip_measured_total_decision_exactness` が production decision coreの symbolic scalar exactnessを
Unwinding Assertions 有効で示すことである。remaining budgetは accumulated-beforeが budget未満なら正確な差、
equalityまたは超過なら zeroへ saturationし、accumulationは数学和が `u64::MAX` 以下なら正確な和、超えるなら
`u64::MAX` へ saturationすることだけを保証する。proof inputは production invariantで縮めない全 `u64` domainである。
EOCD/CD/local-header parse、decompression、CRC、local/central consistency、overlap audit、ZIP64、multi-disk、任意 malformed
stream、shell-core接続条件、複数 entry composition、error priorityはこの proofの保証に含めない。shell-core接続は
`zip_measured_total_connection_regression`、parser shellと roundtrip保証は既存 `zip_total_measured_gate` /
`zip_writer_roundtrip` 系に委譲する。

rf#1015 slice 13 の TAR entry progress / read-demand compositionでは、退役する旧 shell harnessはない。新保証は `arc_tar_entry_progress_decision_exactness` が production decision coreの symbolic scalar exactnessを Unwinding Assertions 有効で示すことである。remaining payloadは producedが declared未満なら正確な差、equalityまたは超過なら zeroへ saturationし、read windowは positive entry残量に対し entry残量・budget残量・capacityの最小値、exhausted entryは budget残量に関係なく no-read/non-exhaustion、produced accumulationは数学和が `u64::MAX` 以下なら正確な和、超えるなら `u64::MAX` へ saturationすることだけを保証する。TAR streaming compositionの残部、複数 entry composition、read/discard/pending/finish state machine、shell-core接続条件、TAR header parse、PAX/GNU extension folding、任意 malformed stream、TAR metadata/path/link gate、decompression、CRC、error priorityはこの proofの保証に含めない。shell-core接続は `tar_entry_progress_connection_regression`、parser shellと roundtrip保証は既存 `tar_total_measured_gate` / `tar_pending_recovery` / roundtrip 系に委譲する。

旧 parser-facing `tar_typeflag_classification` harness は rf#1015 slice 6 で退役した。
旧保証は concrete TAR skeleton と header parser shell を含む attempted proofであり、
rf806では約15 CPU-min・約21.8 GiBまで膨張して verdictを返せなかった。新保証は
`arc_tar_struct_typeflag_exactness` が production decision coreを independent oracleと
比較し、全 256 typeflag byte値の exactnessを Unwinding Assertions 有効で示すことである。
TAR header parse、checksum、PAX/GNU extension folding、複数 header、任意 malformed
stream、数値 field 全体、parser-core接続条件はこの proofの保証に含めない。parser-core
接続は `tar_typeflag_connection_regression`、extension foldと任意 malformed streamは
`tar_pax_gnu_extensions` / `tar_arbitrary_stream_error_set` / roundtrip 系に委譲する。
旧 harnessが verdictを返せなかった経緯は削除しない。

| Harness | Bound | 未証明領域 | 補完 |
| --- | --- | --- | --- |
| `arc_idx_limit_decision_exactness`（in-src `src/zip/read.rs`） | declared count `u16`、declared central-directory size `u64`、`max_entries:u32`、`max_metadata_bytes:u64` を全て symbolic。loopを持たず `#[kani::unwind(2)]`。assumeなし | parser-core接続条件、EOCD scan、実 CD parse、複数 entry、ZIP64、deflate payload、local/central 不整合、overlap audit | proptest `zip_index_limit_gates` / `zip_metadata_limit_gate` / `zip_arbitrary_stream_error_set` / `zip_writer_roundtrip` で担保 |
| `arc_zip_entry_size_declared_gate_exactness`（in-src `src/zip/read.rs`） | declared entry size `u64` と `max_entry_uncompressed_bytes:u64` を全て symbolic。loopを持たず `#[kani::unwind(2)]`。assumeなし | shell-core接続条件、EOCD/CD/local-header parse、measured total budget、entry output hard cap、cap overflow classification、decompression、CRC、local/central consistency、overlap audit、任意 malformed stream | connection regression `zip_declared_entry_size_connection_regression` と proptest `zip_entry_size_limit_gate` / `zip_total_measured_gate` / `zip_writer_roundtrip` で担保 |
| `arc_zip_output_cap_decision_exactness`（in-src `src/zip/read.rs`） | declared entry size `u64`、remaining budget `u64`、`None`/`Some` 選択を全て symbolic。loopを持たず `#[kani::unwind(2)]`。assumeなし。symbolic bool + `u64` は full-domain encoding | shell-core接続条件、EOCD/CD/local-header parse、measured total budget accumulation、decompression、CRC、local/central consistency、overlap audit、任意 malformed stream | connection regression `zip_output_cap_connection_regression` と proptest `zip_entry_size_limit_gate` / `zip_total_measured_gate` / `zip_writer_roundtrip` で担保 |
| `arc_zip_measured_total_decision_exactness`（in-src `src/zip/read.rs`） | total budget `u64`、accumulated-before `u64`、measured entry bytes `u64` を全て symbolic。loopを持たず `#[kani::unwind(2)]`。assumeなし | shell-core接続条件、EOCD/CD/local-header parse、decompression、CRC、local/central consistency、overlap audit、任意 malformed stream、複数 entry composition、error priority | connection regression `zip_measured_total_connection_regression` と proptest `zip_total_measured_gate` / `zip_writer_roundtrip` で担保 |
| `arc_payload_limit_declared_exactness`（in-src `src/tar/read.rs`） | effective declared size `u64` と `max_entry_uncompressed_bytes:u64` を全て symbolic。loopを持たず `#[kani::unwind(2)]`。assumeなし | parser-core接続条件、TAR header parse、PAX/GNU override、任意 malformed stream、payload streaming、measured total uncompressed budget、ZIP entry size gate、decompression | proptest `tar_declared_size_gate` / `tar_total_measured_gate` / `tar_arbitrary_stream_error_set` / roundtrip 系で担保 |
| `arc_tar_entry_progress_decision_exactness`（in-src `src/tar/read.rs`） | declared/produced/budget/capacity/read bytesの5つの `u64` を全て symbolic。loopを持たず `#[kani::unwind(2)]`。assumeなし | shell-core接続条件、TAR streaming composition残部、複数 entry composition、read/discard/pending/finish state machine、TAR header parse、PAX/GNU extension folding、任意 malformed stream、TAR metadata/path/link gate、decompression、CRC、error priority | connection regression `tar_entry_progress_connection_regression` と proptest `tar_total_measured_gate` / `tar_pending_recovery` / roundtrip 系で担保 |
| `arc_tar_struct_typeflag_exactness`（in-src `src/tar/read.rs`） | typeflag `u8` 全 256 値を symbolic。loopを持たず `#[kani::unwind(2)]`。assumeなし | parser-core接続条件、TAR header parse、checksum、PAX/GNU extension folding、複数 header、任意 malformed stream、数値 field 全体 | connection regression `tar_typeflag_connection_regression` と proptest `tar_pax_gnu_extensions` / `tar_arbitrary_stream_error_set` / roundtrip 系で担保 |

### slice 3 structured harness（`tests/formal_structured.rs` の `#[cfg(kani)] mod proofs` + `src/input.rs` の in-src proof）

slice 3 以降、CSV limit の形式的証明は parser shell を通さない pure
decision core exactness とする。任意 stream と parser-core接続は
proptest/integration test に委譲する。`BoundedRead` は `pub(crate)`
なので byte-budget の証明は `src/input.rs` 内の
`#[cfg(kani)] mod proofs` に置く（`cargo kani --tests` は lib target
の harness も test mode でカバーする）。旧 parser-facing CSV harnessは
退役し、保証を二重計上しない。

| Harness | Bound | 未証明領域 | 補完 |
| --- | --- | --- | --- |
| `bounded_read_budget_gate`（in-src） | concrete 6 byte input + symbolic `max_bytes:u64`（全値）を 2 byte/read の `read` loop で drive。proof loopは static最大4回で `#[kani::unwind(8)]` | 任意長・任意 backend reader の組合せ・probe 対象 byte の値（`Read` 契約上 0/1 のみ効く） | proptest 各形式の `*_max_bytes_gate`（concrete skeleton + tail/offset）が reader 実経路で担保 |
| `csv_limit_decision_record_count_exactness`（in-src） | symbolic `record_no`/`max_records` の u64 exactness。allocation・I/O・`csv_core`/DFA依存なし | parserが coreを毎回呼ぶ接続条件、任意record内容、複合limit交互作用、`csv_core` correctness | 既存 `csv_max_records_gate` proptestと integration testが接続と実経路を担保 |
| `csv_limit_decision_field_bytes_exactness` / `csv_limit_decision_buffer_growth_exactness`（in-src） | symbolic limit・span/fill/count。field bytesと capacity成長の exactness。boundary-1/boundary/boundary+1は explicit coverで確認 | parserが coreを毎回呼ぶ接続条件、任意field/record内容、複合limit交互作用、`csv_core` correctness | 既存 `csv_max_field_bytes_gate` / `csv_max_record_bytes_gate` / `csv_max_fields_gate` proptestが接続と実経路を担保 |

### slice 4 WAV harness（`src/convert.rs` + `src/wav.rs` の in-src proofs + `tests/formal_wav.rs` の `#[cfg(kani)] mod proofs`）

slice 2/3 と同じ責務分割 — symbolic は「契約を一意に決める軸」
だけに絞り、それ以外は concrete matrix か proptest へ委譲する。
`SampleRate::new` は `TimeBase::new` 経由で Euclid の `while` loop
を含み、symbolic rate の nested `%` unroll が参照 WSL2 環境
（31 GiB）を枯渇させるため、rate を含む harness は全て concrete
boundary matrix として設計した。

| Harness | Bound | 未証明領域 | 補完 |
| --- | --- | --- | --- |
| `write_sample_documented_policy`（in-src `src/convert.rs`） | symbolic `f32`（NaN・±Inf・±0・subnormal・±full-scale・任意内部値を全網羅）× symbolic wire format 選択。`#[kani::unwind(16)]` | 非 wire format（`write_sample` の wire 外への振る舞い）・frame→packet の batch 経路 | 非 wire format は proptest `sample_conversion_matches_documented_policy` が全 10 variant で担保・batch 経路は `encoder_packet_stream_is_the_frame_stream` が担保 |
| `format_tag_membership_is_wire_formats`（in-src `src/wav.rs`） | symbolic `SampleFormat` 全 10 variant（enum discriminant symbolic）— `format_tag` の `Some/None` と `WIRE_FORMATS` membership の一致を尽きる。`#[kani::unwind(8)]` | —（全 variant を尽きるため gap なし） | `wire_new_non_wire_is_unsupported` が実 `Wire::new` 経路の `Unsupported` kind を concrete 全 5 非 wire format で証明・proptest `wire_gate_is_exact_membership_plus_byte_rate` が encoder/muxer 実経路で担保 |
| `wire_new_non_wire_is_unsupported`（in-src `src/wav.rs`） | concrete: 全 5 非 wire `SampleFormat` — `Wire::new` が `ErrorKind::Unsupported` を返すことを尽きる。`#[kani::unwind(8)]` | 診断 payload 文面（`cfg(kani)` で簡略化 — 下記仮定参照） | proptest `declaration_mutations_keep_stable_error_kinds` / `wire_gate_is_exact_membership_plus_byte_rate` が kind 安定集合を実経路で担保 |
| `wire_new_gate_is_exact_{small,mid,large}_align`（in-src `src/wav.rs`） | concrete matrix: 全 13 種の distinct `block_align`（1..128 を 5+4+4 reps に 3 分割）× boundary rate `u32::MAX/align` の ±1。`#[kani::unwind(8)]` | `u32::MAX/align` から ±2 以上離れた interior rate・symbolic rate 全空間 | interior domain は proptest `wire_gate_is_exact_membership_plus_byte_rate`（boundary ±4 band + uniform rate）が担保 |
| `fmt_and_header_layout`（in-src `src/wav.rs`） | concrete matrix: 5 spec（16 byte `fmt ` 2 件 Mono S16/Stereo F32・extensible 40 byte 3 件 5.1 S24/7.1 F32/custom mask S32）。`#[kani::unwind(64)]`（builtin `memcmp` の byte unroll — 最大比較は 40 byte `fmt ` body） | symbolic byte 内容・header 以外の write 経路 | 任意 spec での header 構造は proptest `open_muxer_writes_complete_header` が担保・wire byte 実経路は roundtrip property が担保 |
| `open_muxer_spec_contract_{pcm,extensible}`（tests `#[cfg(kani)]`） | concrete success matrix: 16B form 1 spec（Mono U8）・40B form 3 spec（5.1 S24・7.1 F32・custom mask S32）— 固定 80B 配列の最小 `Write+Seek` sink（`KaniSink`）で `Cursor<Vec<u8>>` splice/growth の SSA 膨張を排除。`#[kani::unwind(96)]` | symbolic `WavOutputSpec`・finalize 後 backpatch・複数 stream | finalize/backpatch・one-shot・receipt 値は proptest `muxer_finalize_receipt_backpatch_one_shot` が担保・16B 非 ext の全 byte layout は `fmt_and_header_layout` が担保 |
| ~~`open_muxer_rejects_invalid_value`~~（tests `#[cfg(kani)]` — residual slice で lane から退役） | concrete: byte-rate overflow 1 spec（7.1 F32・rate = `u32::MAX/32+1`）— `open_muxer` が `ErrorKind::InvalidValue` に写像することを証明する Err-direction harness。`#[kani::unwind(4)]`（到達可能 loop は 1 要素の stream 検証と GCD のみ・header write unroll は dead path） | symbolic spec — および verdict 自体: 参照 WSL2 環境（31 GiB）では `open_muxer` 経由が untrimmed/trimmed/KaniSink/unwind 96/16/4 の全形で cbmc ~31.7 GiB に張り付き（slice 4）、residual slice で gate 直上の `validate_stream` 単体に再設計しても concrete 入力で ~31.9 GiB に張り付いた（dominant cost は `open_muxer` の死後段 CFG ではなく `validate_stream` 自身の error-arm SSA — `Error::with_source` + `format!`/`io::Error` 構築）。verdict 未発行の `not_run`（環境起因）が確定したため harness は lane から除去 — 永久に verdict を返さない証明を残すと全実行が fail になる。`evidence/rf806` residual 節を参照 | `wire_new_gate_is_exact_*` が source gate の `InvalidValue` を exhaustive に証明（この正確な spec を boundary matrix で網羅）・`open_muxer` の `?` 伝播は proptest `muxer_rejections_are_invalid_data_and_not_sticky` が任意 spec で担保 |

仮定（slice 4 固有）:

- `Wire::new` の `Unsupported` 経路は `std::io::Error::new` +
  `format!`（symbolic `{:?}` 診断）を構築し、その式構築だけで
  参照 WSL2 環境が枯渇することを実測（concrete 列挙でも同様）。
  そのため `src/wav.rs` では診断文面の構築を
  `unsupported_sample_format` helper に隔離し、`cfg(kani)` 時は
  `format!` を持たない軽量診断に切り替える（`ErrorKind`・分岐
  条件は不変 — 診断 payload 文面のみの `cfg` 差分）。この置換は
  `scarecrow173/ruviform#1015` が再評価する legacy exception であり、
  新規 proof にコピーしてよい pattern ではない。これにより
  非 wire format の `Unsupported` kind は
  `wire_new_non_wire_is_unsupported` が concrete 全 5 variant で
  証明し、membership の exhaustiveness は
  `format_tag_membership_is_wire_formats`、encoder/muxer 実経路は
  proptest が担保する 3 つの補完関係。
- symbolic `SampleRate::new` は `TimeBase::new` の Euclid `while`
  loop で nested symbolic `%` を生成し、rate を symbolic に含む
  harness は全て参照環境で infeasible だった。rate を使う全
  harness は concrete matrix に留め、「boundary の ±1」という
  gate 契約が効く点のみを証明する（interior は proptest に委譲
  — slice 1 の「式を弱めない・bound を正直に記録する」方針と同一）。
- `#[kani::unwind(N)]` は `fmt ` body の slice 比較（≤40 byte）・
  builtin memmove/memcmp・GCD loop の concrete unroll を含む
  最小 bound + 余裕。bound 超過は unwinding assertion として
  loud に検出される（`fmt_and_header_layout` は `memcmp` loop の
  unwinding assertion を 1 度検出し、unwind を 16→64 に
  引き上げて解決した記録あり）。
- `open_muxer` の Err-direction（「必ず `InvalidValue` で reject
  される」= Ok 経路の不可達性証明）は、成功 direction の
  witness 探索と比べ構造的に重い。slice 4 では `open_muxer` 全
  CFG encoding が ~31.7 GiB envelope に張り付き、支配コストは
  post-gate CFG と推定していたが、residual slice で gate 直上の
  `validate_stream` 単体への最小再設計でも concrete 入力で
  ~31.9 GiB に張り付いた — 支配コストは `validate_stream` 自身の
  error-arm SSA（`Error::with_source` + `format!`/`io::Error`
  構築 ×6 site）であることが確定した。verdict 未発行の
  harness は `not_run` と fail-closed で記録し、`pass` 扱いしない
  — union 補完の分担は上表のとおり（composite harness 自体は
  lane から退役 — 永久に verdict を返さない証明を残すと全
  実行が `fail` になるため）。

仮定:

- `kani::assume` は正当な事前条件または明示的な限定領域に限り、
  充足可能性と必要経路の到達可能性を評価する。Bound を縮める場合は
  保証差を記録する。個別 harness がこの評価軸を満たすかどうかの
  再審査は `scarecrow173/ruviform#1015` に委ねる。
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
- CSV harness 固有の legacy exception は slice 3 で解消した。旧実装では
  `CsvReader::new` が呼ぶ `csv_core` の `build_dfa` が proof boundaryに入り、
  io-structured invocation は `#[kani::unwind(128)]` +
  `--no-unwinding-checks` で当該 loop を意図的に truncate していた。
  この検査無効化に依存する旧結果は通常の Proof Passに含めない履歴として残す。
  slice 3 は `build_dfa` を proof boundaryに含めず、pure decision coreを
  Unwinding Assertions 有効で検証する。c >= 128 rowの到達不能性は例外根拠に
  使わない。parser-core接続条件は Kani未証明であり、proptest依存の保証差を
  ledgerへ残す。
- serde_json の float 復号精度: `float_roundtrip` feature は無効の
  ため、最短表現 decimal が parse 時に 1 ulp ずれる場合がある。
  proptest 側は `json_value_eq`（整数は厳密・float は 1 ulp 以内）
  で「wire 上の意味的同一性」を検証する。これは codec 契約の
  外側にある serde_json 既知の精度限界であり、契約違反ではない。

## Legacy exception（`scarecrow173/ruviform#1015` の再評価対象）

次の例外は `POLICIES.md` §6.4 が原則回避または原則禁止とする例外で
あり、独立した妥当性根拠と review を伴う再評価の対象である。

- `ruviform-io-structured` に対するクレート単位の
  `--no-unwinding-checks` invocation — slice 3 で撤廃済み。旧結果は
  legacy-exception result として履歴に残し、通常の Proof Passに数えない。
- WAV backend の `cfg(kani)` による診断 payload 置換 — 未移行の
  legacy exception。

これらを 通常の Proof Pass の根拠や新規 proof にコピー可能な
pattern として扱わない。本文書内の関連記述は、当時の境界と未証明領域を
示す inventory であり、例外の承認ではない。proof placement のための
`#[cfg(kani)] mod proofs` とは区別する。rf#1015 slice 1 の再評価では、
structured は DFA/reader 境界再設計と crate 単位 invocation 撤廃、WAV は
lazy/pure diagnostic boundary への移行と `cfg(kani)` 置換撤廃を採用した。
structured は slice 3 で実施済みであり、WAVは実施完了まで legacy exception
のまま会計する。新 verdict だけを
Proof Pass に計上する（decision matrix: `evidence/rf1015/property-ledger.md`
§4 D-1/D-2）。

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
ことを記録する。rf#1015 slice 3 では CSV decision coreの `max_field_bytes` 比較を
`>` から `>=` へ 1 箇所だけ変え、`mise run test-formal-kani sd-csv-limits`
が semantic assertion failureで exit 1 を返すことを記録した。検出は
unwinding assertion failureではなく boundary equality の反例である。
mutantは完全に revertし、同じ selective laneで緑へ戻した。
rf#806 slice 4 では `convert.rs` の S16 全範囲 scaling の `f32::round`
を `trunc` に 1 箇所だけ変え、`formal_wav` の
`sample_conversion_matches_documented_policy` が shrink つき
fail することを記録する（round-half-away-from-zero 契約の検出
能力を実証 — `.trunc()` は正の内部値で 1 LSB ずれる）。

rf#1015 slice 2 の `W-IPC-OP` では、mutation 対象を proptest だけでなく
selective Kani lane にも対応づける。実施した mutant は `valid_op_bytes` の
長さ上限を `MAX_OP_LEN` から `MAX_OP_LEN + 1` へ緩める 1 箇所であり、
`mise run test-formal-kani w-ipc-op` が boundary matrix の反例で `fail` と
exit 1 を返した。mutant は完全に revert し、同じ selective lane と
`mise run test-formal` の再実行で緑へ戻した。実行記録は
`evidence/rf1015/verification-slice2.md` と `runs.jsonl` を参照する。

rf#1015 slice 4 の `ARC-IDX-LIMIT` では、decision coreの entries gate比較を
`>` から `>=` へ 1 箇所だけ変え、`mise run test-formal-kani arc-idx-limit`
が semantic assertion failureで exit 1 を返した。検出は unwinding assertion
failureではなく、boundary equalityで `Accept` / `MetadataExceeded` を期待する
assertionの反例である。mutantは完全に revertし、同じ selective laneと
`mise run test-formal` の再実行で緑へ戻した。実行記録は
`evidence/rf1015/verification-slice4-archive.md` と `runs.jsonl` を参照する。

rf#1015 slice 5 の `ARC-PAYLOAD-LIMIT` では、decision coreの declared-size gate
比較を `>` から `>=` へ 1 箇所だけ変え、`mise run test-formal-kani
arc-payload-limit` が semantic assertion failureで exit 1 を返した。検出は
unwinding assertion failureではなく、boundary equalityで `Accept` を期待する
assertionの反例である。mutantは完全に revertし、同じ selective laneと
`mise run test-formal` の再実行で緑へ戻した。実行記録は
`evidence/rf1015/verification-slice5-archive-payload.md` と `runs.jsonl` を参照する。

rf#1015 slice 6 の `ARC-TAR-STRUCT` では、decision coreの `TYPE_SYMLINK` 分類を
SymbolicLink から File へ 1 箇所だけ変え、`mise run test-formal-kani
arc-tar-struct` が semantic assertion failureで exit 1 を返した。検出は
unwinding assertion failureではなく、production coreと independent oracleの
exactness assertionの反例である。mutantは完全に revertし、同じ selective laneと
`mise run test-formal` の再実行で緑へ戻した。実行記録は
`evidence/rf1015/verification-slice6-archive-typeflag.md` と `runs.jsonl` を参照する。

rf#1015 slice 7 の `W-FRM-CODEC` では、decision coreの cap 判定を `<=` から `<`
へ 1 箇所だけ変え、`mise run test-formal-kani w-frm-codec` が semantic assertion
failureで exit 1 を返した。検出は unwinding assertion failureではなく、exact-cap
acceptanceを期待する write/prefix/roundtrip assertionの反例である。mutantは完全に
revertし、同じ selective laneと `mise run test-formal` の再実行で緑へ戻した。実行記録は
`evidence/rf1015/verification-slice7-frame-codec.md` と `runs.jsonl` を参照する。

rf#1015 slice 8 の `W-IPC-ENVELOPE` では、decision coreの body size cap 判定を `<=` から `<`
へ 1 箇所だけ変え、`mise run test-formal-kani w-ipc-envelope` が semantic assertion failureで
exit 1 を返した。検出は unwinding assertion failureではなく、exact-cap acceptanceを期待する
size exactness assertionの反例である。mutantは完全に revertし、同じ selective laneと
`mise run test-formal` の再実行で緑へ戻した。実行記録は
`evidence/rf1015/verification-slice8-ipc-envelope.md` と `runs.jsonl` を参照する。

rf#1015 slice 9 の `ARC-PAYLOAD-LIMIT` ZIP declared entry-size gateでは、decision coreの
declared-size gate比較を `>` から `>=` へ 1 箇所だけ変え、`mise run test-formal-kani
arc-zip-entry-size` が semantic assertion failureで exit 1 を返した。検出は unwinding assertion
failureではなく、boundary equalityで `Accept` を期待する assertionの反例である。mutantは完全に
revertし、同じ selective laneと `mise run test-formal` の再実行で緑へ戻した。実行記録は
`evidence/rf1015/verification-slice9-zip-entry-size.md` と `runs.jsonl` を参照する。

rf#1015 slice 10 の `ARC-PAYLOAD-LIMIT` ZIP entry output hard-cap / cap-overflow classificationでは、
decision coreの budget overflow判定を `<` から `<=` へ 1 箇所だけ変え、`mise run test-formal-kani
arc-zip-output-cap` が semantic assertion failureで exit 1 を返した。検出は unwinding assertion
failureではなく、remaining equalityと budgetなしで declared-overproductionを期待する assertionの反例である。
mutantは完全に revertし、同じ selective laneと `mise run test-formal` の再実行で緑へ戻した。実行記録は
`evidence/rf1015/verification-slice10-zip-output-cap.md` と `runs.jsonl` を参照する。

rf#1015 slice 11 の `ARC-PAYLOAD-LIMIT` ZIP measured total budget accumulationでは、decision coreの
accumulationを measured bytesを加えず accumulated-beforeをそのまま返す identityへ 1 箇所だけ変え、`mise run test-formal-kani
arc-zip-measured-total` が semantic assertion failureで exit 1 を返した。検出は unwinding assertion
failureではなく、non-overflow sumと exact `u64::MAX` boundaryを期待する assertionの反例である。mutantは完全に
revertし、同じ selective laneと `mise run test-formal` の再実行で緑へ戻した。実行記録は
`evidence/rf1015/verification-slice11-zip-measured-total.md` と `runs.jsonl` を参照する。

rf#1015 slice 12 の `ARC-PAYLOAD-LIMIT` TAR measured total read-window / accumulationでは、decision coreの
accumulationを measured bytesを加えず accumulated-beforeをそのまま返す identityへ 1 箇所だけ変え、`mise run test-formal-kani
arc-tar-measured-total` が semantic assertion failureで exit 1 を返した。検出は unwinding assertion
failureではなく、non-overflow sumと exact `u64::MAX` boundaryを期待する assertionの反例である。mutantは完全に
revertし、同じ selective laneと `mise run test-formal` の再実行で緑へ戻した。実行記録は
`evidence/rf1015/verification-slice12-tar-measured-total.md` と `runs.jsonl` を参照する。

rf#1015 slice 13 の `ARC-PAYLOAD-LIMIT` TAR entry progress / read-demand compositionでは、decision coreの remaining payload計算が producedを無視して declaredをそのまま返す mutantへ 1 箇所だけ変え、`mise run test-formal-kani arc-tar-entry-progress` が semantic assertion failureで exit 1 を返した。検出は unwinding assertion failureではなく、remaining equalityと exhaustion後 zero remainingを期待する assertionの反例である。mutantは完全に revertし、同じ selective laneと `mise run test-formal` の再実行で緑へ戻した。実行記録は `evidence/rf1015/verification-slice13-tar-entry-progress.md` と `runs.jsonl` を参照する。

slice 2 の `W-IPC-OP` では、verified Sync Commit を
`5024268f56339c91ba0487fe8d87198a8d529cab` とする。これは製品実装・proof
harness・mise laneを変更した最後の commitであり、selective Kani、mutation、
proptest、crate regression の検証対象である。`8cb8a6d58933d04415cfe47f92381a913c547d52`
以降の本 slice の sync commit chainは docs/skill/evidence-onlyでなければならず、
製品/proof/mise 差分を追加する場合は再検証なしに verified Sync Commitを進めない。evidenceは
自分自身を含む commit hashを固定点として書けないため、最終 head SHAは
PR #1027 の head と本文で追跡する。`runs.jsonl` では proof/regression 系 record
が `5024268f` を、evidence本文を検査する prek recordだけが exact runのある
evidence sync commitを `target_sha` に使う。

slice 3 の structured redesign では、verified Sync Commitを
`f6517d639a66b3f2301f92473a70ba7d84e4abf0` とする。これは最新mainを取り込み、
CSV decision core、BoundedRead proof/実装、selective lane、mise laneの最終変更
`01b369a452c69521149de762c5278b5ac331e8ea` を含むmerge commitであり、selective Kani・mutation・proptest・crate regressionの検証
対象である。後続の docs/skill/evidence-only commitは verified Sync Commitを
動かさない。製品/proof/mise差分が加わった場合は再検証なしに進めない。

slice 4 の `ARC-IDX-LIMIT` では、verified Sync Commitを
`acb773db88ef8e40bfb43a491336b00fad7cbb47` とする。これは ZIP index decision core、
in-src proof、旧 parser-facing harness退役、selective laneを含む最後の
製品/proof/mise変更 commitであり、selective Kani・mutation・proptest・crate regressionの
検証対象である。後続の docs/skill/evidence-only commitは verified Sync Commitを動かさない。
製品/proof/mise差分が加わった場合は再検証なしに進めない。

slice 5 の `ARC-PAYLOAD-LIMIT` では、verified Sync Commitを
`7a00436aa3ed1e85c11a00b1f05d93a626833a2e` とする。これは TAR declared-size
decision core、in-src proof、旧 parser-facing harness退役、selective laneを含む最後の
製品/proof/mise変更 commitであり、selective Kani・mutation・proptest・crate regressionの
検証対象である。後続の docs/skill/evidence-only commitは verified Sync Commitを動かさない。
製品/proof/mise差分が加わった場合は再検証なしに進めない。

slice 6 の `ARC-TAR-STRUCT` では、verified Sync Commitを
`9729366a37ebf10174132f829c5cdf07b0c2c980` とする。これは TAR typeflag decision core、
in-src proof、旧 parser-facing harness退役、selective laneを含む最後の
製品/proof/mise変更 commitであり、selective Kani・mutation・proptest・crate regressionの
検証対象である。後続の docs/skill/evidence-only commitは verified Sync Commitを動かさない。
製品/proof/mise差分が加わった場合は再検証なしに進めない。

slice 7 の `W-FRM-CODEC` では、verified Sync Commitを
`82d260dfa0b98d9cf325dc1b085090f2bb2c6d0f` とする。これは frame length-prefix/write-cap
decision core、in-src proof、旧 shell roundtrip harness退役、selective laneを含む最後の
製品/proof/mise変更 commitであり、selective Kani・mutation・proptest・crate regressionの
検証対象である。後続の docs/skill/evidence-only commitは verified Sync Commitを動かさない。
製品/proof/mise差分が加わった場合は再検証なしに進めない。

slice 8 の `W-IPC-ENVELOPE` では、verified Sync Commitを
`bf5c61a1c3d1d305851e56a1558424fe90f94b24` とする。これは IPC envelope decision core、
in-src proof、旧 shell harness退役、deterministic regression、selective laneを含む最後の
製品/proof/mise変更 commitであり、selective Kani・mutation・proptest・crate regressionの
検証対象である。後続の docs/skill/evidence-only commitは verified Sync Commitを動かさない。
製品/proof/mise差分が加わった場合は再検証なしに進めない。

slice 9 の `ARC-PAYLOAD-LIMIT` ZIP declared entry-size gateでは、verified Sync Commitを
`f582f3cb700285e46ee24f99ba9c80841249156c` とする。これは ZIP declared entry-size decision core、
in-src proof、deterministic regression、selective laneを含む最後の製品/proof/mise変更 commitであり、
selective Kani・mutation・proptest・crate regressionの検証対象である。`e04db55b` は test module
documentationだけの comment-only descendantであり、verified Sync Commitを動かさない。後続の
docs/skill/evidence-only commitも verified Sync Commitを動かさない。製品/proof/mise差分が加わった場合は
再検証なしに進めない。

slice 10 の `ARC-PAYLOAD-LIMIT` ZIP entry output hard-cap / cap-overflow classificationでは、verified Sync Commitを
`93df62a7b49a6f57e4fb2a27d68cb64b9079e416` とする。これは ZIP entry output cap decision core、in-src proof、
deterministic regression、selective laneを含む最後の製品/proof/mise変更 commitであり、selective Kani・mutation・
proptest・crate regressionの検証対象である。後続の docs/skill/evidence-only commitは verified Sync Commitを動かさない。
製品/proof/mise差分が加わった場合は再検証なしに進めない。

slice 11 の `ARC-PAYLOAD-LIMIT` ZIP measured total budget accumulationでは、verified Sync Commitを
`1ffa93a90c11849778bd22c391c33d986ecaabd1` とする。これは ZIP measured total budget accumulation decision core、in-src proof、
deterministic regression、selective laneを含む最後の製品/proof/mise変更 commitであり、selective Kani・mutation・
proptest・crate regressionの検証対象である。後続の docs/skill/evidence-only commitは verified Sync Commitを動かさない。
製品/proof/mise差分が加わった場合は再検証なしに進めない。

slice 12 の `ARC-PAYLOAD-LIMIT` TAR measured total read-window / accumulationでは、verified Sync Commitを
`486b52963757fc7dd807fae287eedc078440600c` とする。これは TAR measured total read-window/accumulation decision core、in-src proof、
deterministic regression、selective laneを含む最後の製品/proof/mise変更 commitであり、selective Kani・mutation・
proptest・crate regressionの検証対象である。後続の docs/skill/evidence-only commitは verified Sync Commitを動かさない。
製品/proof/mise差分が加わった場合は再検証なしに進めない。

slice 13 の `ARC-PAYLOAD-LIMIT` TAR entry progress / read-demand compositionでは、verified Sync Commitを `deca7e9b4752f1994df736d7d70f1bcd1b14d026` とする。これは TAR entry progress/read-demand composition decision core、in-src proof、deterministic regression、selective laneを含む最後の製品/proof/mise変更 commitであり、selective Kani・mutation・proptest・crate regressionの検証対象である。後続の docs/skill/evidence-only commitは verified Sync Commitを動かさない。製品/proof/mise差分が加わった場合は再検証なしに進めない。

## 判定語彙と fail-closed

`pass` / `fail` / `not_run` / `unsupported` / `blocked` — verify-
ruviform skill の語彙に従う。環境欠如は `not_run` +
`environment_blocked` で exit 0、証拠のない lane を `pass` と記録
しない。ローカル検証は変更範囲の検証であり、merge 後の対象 SHA
に対する必須 CI 結果が統合検証の証拠（本記録はそれを代替しない）。

要求状態、verdict、保証種別、対象（実装/モデル/抽象化）、同期状態、
失敗原因は独立した軸である。Timeout、OOM、環境欠如、プロセスの exit 0、
CI green を Proof Pass としない。証跡には対象 SHA、tool version、実行
条件、保証範囲、mutation 証跡を記録し、再検証と対応確認が完了するまで
検証済み sync commit を進めない。

## 手法適合候補（固定昇格路ではない）

以下は tool fit の観点で各手法が適合し得る候補領域の inventory であり、
採用義務・順序・昇格経路ではない。実際の手法・境界・必要保証は
`POLICIES.md` §6 に従い Property ごとに決定する。

| 候補 target | 対象 crate / 契約 | Kani が適合し得る理由 |
| --- | --- | --- |
| parser family | `ruviform-ui-theme`（`rvf-dtcg-profile.v1` の parse/validate → typed `ResolvedTheme`） | wire codec と同型 — bounded symbolic byte 列に対し「panic なし・エラー分類が安定集合・受理なら構造契約を満たす」を尽きられる。`ruviform-io-archive`（tar/zip header・`limits` の上限 gate）は slice 2 で、`ruviform-io-structured`（JSON/YAML/TOML/CSV/XML reader・`InputLimits`/`BoundedRead` byte budget・`DocumentErrorKind` 安定集合・per-format named limit gate）は slice 3 で、`ruviform-io-wav`（RIFF/WAVE header・`Wire` gate・canonical `f32`→wire 変換・muxer finalize/`WavReceipt`）は slice 4 で本形を適用済み |
| unsafe 局在コード | `ruviform-math`（SIMD dispatch）、`ruviform-audio-device` / `ruviform-midi-device` / `ruviform-observability` の `#[expect(unsafe_code)]` 局所箇所 | unsafe ブロックの前後条件・pointer/len 不変条件を symbolic に検証 — 通常 test では到達不能な入力経路をカバーできる（harness は unsafe 自体ではなく公開境界の契約を検証する設計にする） |
| DSP helper 算術 | `ruviform-dsp`（biquad/fft/fir/resample/window の係数・index・累積計算）、`ruviform-math` の scalar helper | 「オーバーフロー/除算ゼロ/index 範囲外が起きない」「精度 bound」等の算術不変条件は bounded symbolic 入力で尽きやすい。数学的定理としてより強い保証が必要な個別 Property では Lean が適合し得る |

- ordering / state machine 系（session・dock・close 協議）は
  TLA+ が適合し得る候補（`scarecrow173/ruviform#389` 系）。
- 全サイズの数学的不変条件（DSP・算術）は Lean 4 が適合し得る候補。
- 本 slice で確立した「coverage 記録 + sync commit + 1 コマンド
  再実行 + mutation-fail 証跡」の型は、固定昇格路ではなく個別に
  選ばれた新規検証にも適用できる。
