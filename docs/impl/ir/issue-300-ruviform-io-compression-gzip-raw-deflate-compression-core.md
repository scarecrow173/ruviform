---
id: "IMPL-034RUlcumqbCyTpQ93jXxC"
type: "impl"
status: "completed"
title: "Issue 300 ruviform-io-compression gzip・raw deflate Compression Core"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/300"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-compression/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-compression/README.md"
      - type: "file"
        path: "crates/ruviform-io-compression/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/options.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/gzip.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/deflate.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/encoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/decoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/gzip_format.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/deflate_format.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/composition.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/api_surface.rs"
      - type: "file"
        path: "docs/specs/issue-300-io-compression-gzip-deflate.md"
      - type: "file"
        path: "docs/designs/issue-300-io-compression-gzip-deflate.md"
      - type: "file"
        path: "docs/plans/issue-300-io-compression-gzip-deflate.md"
      - type: "file"
        path: "docs/test-specs/issue-300-io-compression-gzip-deflate.md"
      - type: "file"
        path: "docs/tasks/issue-300-t1-crate-core.md"
      - type: "file"
        path: "docs/tasks/issue-300-t2-tests.md"
      - type: "file"
        path: "docs/tasks/issue-300-t3-ci-lane.md"
      - type: "file"
        path: "docs/impl/ir/issue-300-ruviform-io-compression-gzip-raw-deflate-compression-core.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "ci/impact/map.mjs"
      - type: "file"
        path: "ci/impact/tests/classify.test.mjs"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "docs/designs/issue-333-impact-based-ci.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
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
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
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
  implements:
    - "docs/tasks/issue-300-t1-crate-core.md"
    - "docs/tasks/issue-300-t2-tests.md"
    - "docs/tasks/issue-300-t3-ci-lane.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-300-io-compression-gzip-deflate.md"
    - "docs/designs/issue-300-io-compression-gzip-deflate.md"
    - "docs/plans/issue-300-io-compression-gzip-deflate.md"
    - "docs/designs/issue-333-impact-based-ci.md"
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/301"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-300-io-compression-gzip-deflate.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 300 ruviform-io-compression gzip・raw deflate Compression Core

## Summary

IO-IMP-06（Issue #300）として、独立クレート `ruviform-io-compression`
を新設し、gzip（RFC 1952 member framing・CRC32/ISIZE 検証・連結
member・trailing 拒否）と raw deflate（RFC 1951 単一 stream・checksum
なし）の `Encoder<'w, W: Write + ?Sized>` / `Decoder<'r, R: BufRead +
?Sized>` を実装した。flate2 は `rust_backend` のみ・optional dep の
内部 primitive（`Compress`/`Decompress`/`Crc`）に閉じ込め、gzip の
member framing・byte accounting・state machine はクレート側で所有する。
Encoder は `finish()` のみが成功証明（冪等 receipt・Failed terminal）、
Decoder は verified end のみ `Ok(0)`・`finish()` は未読出力を
`NotExhausted` で拒否し、0-length `read(&mut [])` は EOF 根拠にしない。
`EncodeLimits`/`DecodeLimits` は累積 budget として強制。CI には
feature matrix（default/gzip/deflate/両方）を走る専用 lane
`compression-verify` を icc-verify 型で追加した。

## Context

- 実施タスク: [T1](../../tasks/issue-300-t1-crate-core.md) /
  [T2](../../tasks/issue-300-t2-tests.md) /
  [T3](../../tasks/issue-300-t3-ci-lane.md)
- 上流: [plan](../../plans/issue-300-io-compression-gzip-deflate.md)、
  [design](../../designs/issue-300-io-compression-gzip-deflate.md)、
  [spec](../../specs/issue-300-io-compression-gzip-deflate.md)、
  [test-spec](../../test-specs/issue-300-io-compression-gzip-deflate.md)、
  Issue #285（IO-03 設計）・#282（横断設計親 Issue）
- 境界: zstd（IO-IMP-07）は Issue #301 — 本クレートの共有語彙
  （`DecodeScope`/`EncodeLimits`/`DecodeLimits`/receipt/error kind）を
  再利用する設計で、実装は持ち込まない。Archive/File 消費者統合・
  AudioLens Job/Dataset・Synvelis Project/Revision の意味論は
  消費者側の責務（本クレートに持ち込まない）。IO-06 の横断受入は
  Issue #288、cross-repo 追跡は Issue #219。

## Implementation

### Changes

- `crates/ruviform-io-compression/`（新設）— 非 RT 専用の汎用圧縮 I/O
  基盤。`default = []`・feature `gzip`/`deflate`・`publish = false`
  継承・MSRV 1.89。
- `src/error.rs` — `CompressionError`/`CompressionErrorKind`
  （`Io`/`InvalidInput`/`InvalidFormat`/`TruncatedInput`/`CorruptData`/
  `ChecksumMismatch`/`TrailingData`/`LimitExceeded`/`Unsupported`/
  `Cancelled`/`InvalidState`/`NotExhausted`）。`io::Error` に包んでも
  `source()` 経由で kind を復元できる変換規則。backend の詳細は
  erased `source()` のみで、公開面に flate2 型・version を出さない。
- `src/options.rs` — 共有語彙 `DecodeScope`（`SingleUnit`/
  `WholeInput`）・`EncodeLimits`・`DecodeLimits`。format 別
  `EncodeOptions`/`DecodeOptions` は各 module 側。
- `src/receipt.rs` — `EncodeReceipt`/`DecodeReceipt`/`IntegritySummary`
  — `finish()` のみが返す観測済み証跡（byte 数・unit 数・integrity
  内訳）。
- `src/stream.rs` — private deflate pump・byte 計測・budget 検査・
  sync flush・finish の共有コア。`DeflateIn::pump` は空 input でも
  `decompress()` に掛けて内部 pending output を先に drain してから
  初めて starvation を報告する（実装上の重要点 — 入力を eager に
  消費する decompressor が内部出力を残したまま source starvation に
  誤判定されるのを防ぐ）。
- `src/gzip.rs` — header（FEXTRA/FNAME/FCOMMENT/FHCRC・
  `MEMBER_HEADER_MAX = 64 KiB` 制限）と trailer（CRC32/ISIZE）を
  クレート側で parse/検証する member framing。`WholeInput` は連結
  member を単一 stream として復号し member 末尾の trailing byte を
  `TrailingData` で拒否、`SingleUnit` は最初の member のみ復号し
  後続 byte は呼出し側 `BufRead` に残す。`read_member_boundary` は
  1 byte の `fill_buf`（`[0x1f]` のみ）を EOF/trailing と誤認しない
  よう 2 byte magic を跨いで確定する。
- `src/deflate.rs` — 同コアの単一 stream 実装（checksum なし、
  integrity = stream end 検証のみ）。`WholeInput` は stream 後の byte
  を拒否。
- `Encoder` 状態機械 — `Writing → Finishing → Finished | Failed`。
  `finish()` は trailer/stream-end を emit し receipt を返す唯一の
  成功証明・冪等（保存 receipt を返す）。`flush` は deflate sync
  flush であって完了ではない。`Drop` は一切 write しない（未完了の
  trailer が黙って付かない）。失敗は terminal `Failed` に latch し
  以後の操作は `InvalidState`。
- `Decoder` 状態機械 — `Read` は `Ok(0)` を `VerifiedEnd`（format 末尾・
  trailer/checksum・scope の入力境界契約がすべて成立）でのみ返す。
  `read(&mut [])` は状態を変えず EOF 根拠にしない。`Decoder::finish`
  は `VerifiedEnd` のみ `Ok(DecodeReceipt)`、途中は `NotExhausted`。
  `compressed_bytes_consumed` はこの decoder が論理的に消費した byte
  数（呼出し側 `BufReader` の prefetch 量とは別に追跡）。
- `tests/` — 契約試験（encoder/decoder）、wire-format・相互運用試験
  （gzip/deflate・flate2 backend との cross-check 含む）、合成試験
  （TAR-shaped payload・transcode・`dyn Read`/`dyn Write`）、
  API surface 試験（backend 型が公開されないこと）を feature gate 付き
  で整備。
- workspace 登録 — ルート `Cargo.toml` `members`（`ruviform-io-file`
  直後）、`crates/README.md` 層表、crate README（責務・完了/検証
  セマンティクス・limits・許可依存・RT 契約・実装 Issue）。
- `compression-verify` lane（T3）— `mise.toml` に
  `check/lint/test-io-compression`（全 feature matrix を `&&` 連結）、
  `ci/impact/map.mjs` に `LANE_IDS`/`LANES`（ubuntu・
  `features: ["gzip","deflate"]`）/`OWNER_RULES`（`mise-task-pkgs`）、
  `ci.yml` に `impact` outputs・job・`ci-gate` needs、
  `classify.test.mjs` に `crate:ruviform-io-compression` →
  `verify + compression-verify` の期待ケース、design #333 の lane 表・
  crate→lane-owner 導出・`features` 語彙・`ci-gate` needs 記述を更新。

### Decisions During Implementation

- **gzip framing は flate2 高層 adapter に委譲せずクレート側で所有**:
  `MultiGzDecoder` 等では member 境界・消費 byte・未読 `BufRead`
  保存・scope 別 trailing 判定を呼出し側に提供できないため、
  `Compress`/`Decompress`/`Crc` の低層 primitive のみ借用し
  header/trailer codec を実装した（設計確定どおり）。
- **sync flush は「出力 < scratch buffer」で終了**: `Z_SYNC_FLUSH` 系
  flush は呼出しごとに空 stored-block marker を必ず emit するため
  `made == 0` を終了条件にすると無限ループになる（実測で test
  process が GB 級に膨張）。scratch を満たさない出力で打ち切る規則に
  修正。
- **Deflate decoder の starvation 判定は「出力も進まない」ときのみ**:
  decompressor は入力を eager に消費し内部に pending output を残し
  得るため、空 `fill_buf` を即 `TruncatedInput` にせず、空 input でも
  `decompress()` に掛けて drain を試みてから判定する。
- **gzip member 境界は magic を消費してから parse**: `BufRead` が
  `fill_buf` で `[0x1f]` のみを返す合法ケースを EOF/trailing と誤認
  しないため、境界検査で 2 byte magic を明示的に消費（消費分は
  `compressed` 計上）してから残りの固定 8 byte を読む構造にした。
- **failure 試験は write ではなく flush/finish で誘発**: compressible
  入力は backend 内部にバッファされ小さな `write` が failing writer に
  届かない（非決定的）。sync flush/finish は必ず byte を emit するため
  確定的な失敗入口として使う。
- **budget は「上限ちょうど許容・超過で `LimitExceeded`」**: 上限
  ちょうどの byte は正当に配信し、次の read/write で違反を報告する
  累積 budget semantics — 試験側も「違反は次の操作で表面化」を検証。
- **削減 feature 集合の dead code は cfg gate で解消**: `default=[]`・
  単一 format ビルドで共有 helper が未使用になるため、lint 抑制では
  なく format 別 `#[cfg]` を正確に付けた（`-D warnings` 下で全
  組合せが検証される）。
- **mise task は `&&` 連結・array `run` 不使用**: `ci/impact` の
  `mise-task-pkgs` lane-owner 導出は task body を最初の `[` で打ち切る
  regex parse のため、array 形 `run = [...]` では `-p` 引数を捕捉
  できず lane が「常に expected」に退化する。`&&` 連結はクロス
  プラットフォームで動作し導出にも整合する。

## Related Experiments

### Adopted

None.

### Rejected

None.

## Validation

実施環境: Windows 11 x64（`x86_64-pc-windows-msvc`、mise pin rustc
1.99.0、MSRV 要求は 1.89）・Git Bash。

| Check | コマンド | 結果 |
| --- | --- | --- |
| feature matrix check | `cargo check -p ruviform-io-compression --all-targets`（default/gzip/deflate/gzip,deflate の 4 組合せ） | PASS |
| feature matrix lint | `cargo clippy -p ruviform-io-compression --all-targets [--features …] -- -D warnings`（4 組合せ） | PASS・0 warnings |
| feature matrix test | `cargo test -p ruviform-io-compression [--features …]`（4 組合せ・unit/integration/doctest） | PASS・全緑 |
| classifier contract | `mise run test-ci-impact`（`node --test ci/impact/`：classify/gate/workflow） | PASS・52 件全緑 |

workspace 全体の `mise run fmt`/`check`/`lint`/`test`・`git merge
origin/main` 後の最終再検証は PR 作成直前に実施する（本 IR 執筆時点
では上記 crate/lane スコープの実測のみ記録）。

## Risks

- `flate2` の `rust_backend`（miniz_oxide）経路のみ使用 — SIMD/c
  backend への切替・version bump は feature 変更を伴うため本 Issue の
  範囲外。
- gzip の optional header field（FEXTRA 内部構造等）は長さ制限で防御
  するのみで内容解釈しない — 規格外の extra field 意味論は消費者側。
- CPU 時間・backend 内部メモリの hard limit は宣言しない（Issue #300
  の受入文言どおり累積 byte/unit budget のみ強制）。zip-bomb 級の
  入力対策は呼出し側の `DecodeLimits` 設定に委ねる。
- `compression-verify` は本 PR では shared-config（`mise.toml`・
  `ci/**` 変更）により Full CI で実行される。選択実行下での
  owner 導出は contract test で検証済みだが、実 PR での selective
  skip/run は今後の運用で観測される。

## Follow-ups

- Issue #301（IO-IMP-07）: zstd 実装 — `DecodeScope`/`EncodeLimits`/
  `DecodeLimits`/receipt/error kind の共有語彙を再利用し、
  `compression-verify` lane の `features`/`OWNER_RULES` に `zstd` を
  追加する想定。
- Issue #288（IO-06）: 横断消費者・OS/FS・セキュリティ・性能・互換
  受入 — 本クレートの契約試験結果を受入エビデンスとして引き渡す。
- Issue #286: Archive 設計の TAR → Gzip 合成経路が本クレートの
  Encoder/Decoder を消費する最初の上位レイヤ。
- Issue #219: cross-repo 消費者 CI・影響分類の追跡先。
- AudioLens（#139/#140/#141）・Synvelis（#82）: 消費者側での利用は
  各リポジトリ側の責務（本クレートは汎用 API の提供まで）。
