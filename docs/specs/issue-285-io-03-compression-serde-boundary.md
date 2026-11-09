---
id: "SPEC-7BYANBc86b8qnIn77CPmoE"
type: "spec"
status: "approved"
title: "Issue #285 IO-03 Compression — Serde境界・Format/Version契約・Consumer利用面・証拠項目対応 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/plans/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/tasks/issue-285-io-03-compression-serde-boundary.md"
      - type: "doc"
        path: "docs/test-specs/issue-285-io-03-compression-serde-boundary.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-300-io-compression-gzip-deflate.md"
    - "docs/specs/issue-301-io-compression-zstd.md"
    - "docs/designs/issue-300-io-compression-gzip-deflate.md"
    - "docs/designs/issue-301-io-compression-zstd.md"
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/specs/issue-296-io-file-os-backend.md"
    - "docs/specs/issue-303-io-archive-tar.md"
    - "docs/designs/issue-303-io-archive-tar.md"
    - "docs/specs/issue-286-io-04-zip-tar-archive.md"
    - "docs/designs/issue-286-io-04-zip-tar-archive.md"
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/specs/issue-306-io-composition-e2e.md"
    - "docs/designs/issue-409-scoped-enumeration.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/287"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/296"
    - "https://github.com/scarecrow173/ruviform/issues/300"
    - "https://github.com/scarecrow173/ruviform/issues/301"
    - "https://github.com/scarecrow173/ruviform/issues/302"
    - "https://github.com/scarecrow173/ruviform/issues/303"
    - "https://github.com/scarecrow173/ruviform/issues/304"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/409"
    - "https://github.com/scarecrow173/audio-lens/issues/139"
    - "https://github.com/scarecrow173/synvelis/issues/82"
    - "https://github.com/scarecrow173/synvelis/issues/84"
    - "https://github.com/scarecrow173/synvelis/issues/25"
  defers: []
  deferred-by: []
---

# Issue #285 IO-03 Compression — Serde境界・Format/Version契約・Consumer利用面・証拠項目対応 設計

## What Should Be Built

IO-03「Compression — Gzip/Deflate/Zstd Stream Adapter・終了・
リソース制限」（Issue #285、親 #282）の **正本設計文書** を
`docs/` チェーン（spec → design → test-spec → plan → task）
として作成する。具体的には
`docs/designs/issue-285-io-03-compression-serde-boundary.md`
を新設し、Issue #285 本文の合意済み設計（v0.1・v0.2、
2026-11-04）と最終整合性レビュー追記（実装 Issue 分解
#300/#301）を、次の構成要素として文書化する。

1. **Serde 境界と driver/backend 中立契約**: Compression は
   byte-stream 層であり Serde・document 層に依存しない。
   公開 API に許可される外部型は `std::io::{Read, Write,
   BufRead}` と Ruviform 独自型のみで、`serde` の trait すら
   要求しない。backend（`flate2`/`zstd-safe`/`zstd-sys`）の
   型・Error・feature 名・version は非公開で、診断は
   `Error::source()` の `dyn Error` 経由のみ。自作 codec
   registry・format 自動判定・Archive 内 codec との統合を
   作らない。
2. **Format/Version 契約**: Raw Deflate（RFC 1951）・
   Gzip（RFC 1952）・Zstd Frame（RFC 8878）の 3 形式を
   区別し、Zlib wrapper（RFC 1950）は別形式として明示的に
   対象外とする。Format 別 `EncodeOptions`/`DecodeOptions`
   （level semantics は形式別で統一しない）、`DecodeScope`
   （`WholeInput`/`SingleUnit`）、連結 member/frame の
   処理、checksum の有無を `IntegritySummary` の
   checked/unchecked で区別する契約。gzip header は固定・
   再現可能（mtime=0・filename なし）で、backend version
   間の圧縮 byte 列完全一致は保証しない。
3. **Finish/Drop/所有権契約**: 借用型
   `Encoder<'w, W: Write + ?Sized>` /
   `Decoder<'r, R: BufRead + ?Sized>` のみを初期公開形とし、
   `Encoder` の `Writing -> Finishing -> Finished | Failed`
   と Decoder の per-format `Phase` + `failed` latch
   （`Phase::Done` = VerifiedEnd）の状態機械、`finish()`
   のみを完了証拠とする冪等契約、Drop が一切書込まない
   こと、`NotExhausted`/`InvalidState` の error 語彙を
   固定する。
4. **Resource Budget 契約**: `EncodeLimits`（入力/出力
   byte）・`DecodeLimits`（圧縮入力/復号出力/unit 数）の
   連結 stream 累積 budget、Zstd `max_window_log`、上限
   ちょうど許容・+1 byte は `LimitExceeded` の境界包含、
   CPU 時間・backend 内部 heap・consumer 側メモリを hard
   limit と主張しない正直な境界。
5. **Consumer 利用面**: AudioLens（Dataset 圧縮候補・
   AL #139）と Synvelis（Backup 圧縮 tar.gz・SV #82/#84）
   が、schema/version/canonical bytes を Ruviform に移さず
   `std::io` 合成で利用できる面。圧縮前/後どちらの byte を
   Artifact Hash の対象にするかは Consumer が決める。
6. **他 IO gate との証拠項目対応**: Issue #285 の
   ENC/DEC/SEC/INT/ARC/COMP/BUF 証拠項目と Design
   Acceptance Gate を、既存の per-format test-spec
   （#300/#301）・合成 E2E（#306）・cross-consumer gate
   （#288 の IO-03 matrix 行 C-N1/C-N2/C-X1〜C-X4・E-N1/
   E-X1/E-X2）へ対応付ける索引。

## Why Now

- IO-03 の実装は main に揃っている: `ruviform-io-compression`
  クレートが #300（gzip・raw deflate・PR #356）・#301
  （zstd・PR #359）で merge 済み。`gzip`/`deflate`/`zstd`
  feature matrix が `Cargo.toml` に定義され、
  `mise run test-io-compression` が feature matrix の検証
  入口である。
- ただし Issue #285（設計 Issue）自体の正本設計文書は
  未作成で、Serde 境界・format/version 契約・finish/検証
  意味論・resource budget・Consumer 利用面・証拠項目対応が
  per-format 文書（#300/#301）に分散したままである。
- IO-06 cross-consumer gate（#288）の受入 matrix は IO-03
  を C-N1/C-N2/C-X1〜C-X4 の行で参照しており、その根拠と
  なる設計正本が #285 に求められている。合成 E2E（#306・
  `ruviform-io-e2e`）は TAR→Gzip/Zstd→StagedFile の実 OS
  境界証拠を既に持つ。

## Users and Value

- **Ruviform maintainer / IO-03 Owner**: Serde 境界・
  format/version 契約・finish/検証意味論・budget・error
  分類の正本を得て、後続の format 追加・backend 更新・
  Consumer 統合の判断基準を固定できる。
- **AudioLens / Synvelis Owner**: Dataset/Backup の圧縮
  保存を Ruviform に移さずに利用する面と、checksum の
  位置づけ（整合性検出であり真正性保証ではない）・
  publish 前検証の責務分界を確認できる。
- **#288（IO-06 gate）Owner**: IO-03 matrix 行
  （C-N1/C-N2/C-X1〜C-X4）の根拠となる設計正本と証拠
  項目の対応を辿れる。
- **後続の実装・保守担当**: per-format 詳細は
  #300/#301 の設計・test-spec を正本として参照し、本書
  では横断契約のみを確認できる。

## Scope

### In Scope

- `docs/` 配下の文書チェーン（spec/design/test-spec/plan/
  task）と生成 index のみ。**コード変更なし**。
- Issue #285 本文（合意済み設計 v0.1・v0.2・最終整合性
  レビュー追記）の要求事項の文書化。
- IO-01（#295/#296）・per-format 実装（#300/#301）・
  Archive（#302/#303/#304）・合成 E2E（#306）・旧 API 撤去
  （#305）・rooted capability（#409）・cross-consumer gate
  （#288）の main 上の設計・実装成果を消費した、Serde
  境界・Format/Version 契約・Consumer 利用面・証拠項目
  対応の記述。

### Out of Scope

- `crates/`・`Cargo.toml`・`Cargo.lock`・`mise.toml`・
  `.github/` の変更（本 slice は docs 設計専用）。
- per-format の詳細 API・framing 内部・test 規則の再定義
  （#300/#301 の設計・test-spec が正本。本書は横断契約と
  索引のみを所有する）。
- AudioLens/Synvelis repo の変更・実測（cross-repo gate、
  #219/#294 の consumer-verify が Owner）。
- 新規 CI Workflow・証拠 YAML/JSON スキーマ・Dashboard の
  作成。
- Zlib wrapper（RFC 1950）・preset dictionary・所有型
  API・async/no_std・Archive 内 codec 統合（ZIP entry
  codec は IO-04 側の Capability）。

## Requirements

- Serde 境界: `serde` への依存・serde 型の公開を持たない
  こと、backend 型/Error/feature/version の非公開、
  `source()` 経由の診断到達、自作 codec/registry を作ら
  ないことを、根拠（実装・test-spec の audit 規則）と共に
  明文化する。
- Format/Version 契約: 3 形式（Raw Deflate/Gzip/Zstd
  Frame）の区別、Format 別 options/level semantics、
  `DecodeScope`、連結単位の処理、checksum の
  checked/unchecked 区別、再現性の範囲（同一 backend
  version 内の round-trip と header 固定性。backend
  version 間の byte 列一致は非保証）を明文化する。
- Finish/所有権契約: Encoder/Decoder の借用型・状態機械・
  `finish()` 冪等・Drop 無書込・`NotExhausted`/
  `InvalidState`・失敗時の底流 Writer 所有権保全を
  明文化する。
- Resource Budget: `EncodeLimits`/`DecodeLimits`/
  `max_window_log` の累積適用・境界包含・「保証できない
  制限を偽装しない」規則を明文化する。
- Error 契約: `CompressionErrorKind` の正規化分類語彙と、
  `impl Read`/`impl Write` での `io::Error` 包摂時も
  `source()` で kind を復元できる契約を明文化する。
- Consumer 利用面: AudioLens/Synvelis が schema/version を
  Ruviform に移さず利用できる面と、checksum≠真正性・
  単一 File Publish ≠ 上位層 finish・Ruviform 単一
  Publish ≠ Project Transaction/HEAD CAS の責務分界を
  記述する。
- 証拠項目対応: Issue #285 の ENC/DEC/SEC/INT/ARC/COMP/BUF
  と Design Acceptance Gate の各項目を、既存 test-spec・
  #288 matrix 行・実装 Issue/PR へ対応付ける索引を設計
  文書に含める。
- 「設計の合意 ≠ テスト完了」を明記し、実装済み部分と
  未検証・Consumer 側未実施部分を分離して記録する。

## Acceptance Criteria

- [ ] Serde 境界・Format/Version 契約・Finish/所有権・
      Resource Budget・Error 契約・Consumer 利用面・証拠
      項目対応を含む正本設計文書が `docs/designs/` に存在
      する。
- [ ] 公開 API に serde 依存・serde 型・backend 型を露出
      しない契約と、その検証根拠（per-format test-spec・
      `api_surface.rs`・#288 matrix C-X 系の backend 非露出
      規則）が対応付けられている。
- [ ] Backend 選定表（採用版・MSRV 1.89 整合・ライセンス・
      メンテ・Feature 構成・不採用候補の理由）が per-format
      設計への参照を伴い記述されている。
- [ ] 借用 Writer/BufRead のみの公開形・`finish()` 完了証
      明・Drop 無書込・`NotExhausted`/`InvalidState`・
      `DecodeScope` 境界保持が、状態機械として明文化されて
      いる。
- [ ] Issue #285 の証拠項目（ENC/DEC/SEC/INT/ARC/COMP/BUF）
      と Design Acceptance Gate が既存 test-spec（#300/
      #301）と #288 matrix 行（C-N1/C-N2/C-X1〜C-X4・E-N1/
      E-X1/E-X2）へ対応付けられている。
- [ ] 両 Consumer の schema/version/canonical bytes が
      Ruviform に移動されず利用可能である旨と、責務分界が
      明記されている。
- [ ] 「設計合意と実装/検証の分離」が明記され、未実施項目
      は `not run`/`NotRun`/`Unsupported` と記録する方針が
      明示されている。
- [ ] 変更が `docs/` のみであり、`crates/`・`Cargo.toml`・
      `Cargo.lock`・`mise.toml`・`.github/` に差分がない。

## Deferred Design Concerns

- Consumer 側の実測・移行（AL Dataset 圧縮の Golden・
  byte 互換、SV Backup tar.gz/zip の旧経路互換）| cross-repo
  gate（#219/#294 consumer-verify）と各 Consumer Issue が
  Owner であり本 slice の対象外 | Consumer 候補 tuple の
  実走時 | 設計どおりの互換性が実測で裏付けられないまま
  採用されるリスク。
- macOS/Linux 実機での `zstd` feature（vendored C build）
  経路の実測 | 本 slice の作業環境は Windows のみ | CI 既存
  OS lane または実機環境が利用可能になった時点 | OS/C
  toolchain 差異が zstd feature で未検証のまま残る。
- 所有型 API（`finish(self)->Result<W,_>`）・zlib
  wrapper・preset dictionary・skippable frame 書込 | 実需が
  確認できた時点 | #300/#301 設計の Deferred Design
  Concerns と同一の再開条件 | 公開面の拡張なしに Consumer
  が必要とする合成が実現できないリスク。
- `Cancelled` kind の上位 Job 連携 | Compression 専用
  Scheduler を作らない方針のもと、上位 Job 制御の実装時 |
  中断後の `finish()` 成功扱い防止と StagedFile abort の
  配線が未検証のまま残るリスク。

## Sources and Relations

- 正本 Issue: [#285](https://github.com/scarecrow173/ruviform/issues/285)
  （IO-03 合意済み設計 v0.1・v0.2・2026-11-04・最終整合性
  レビュー追記 — 実装 Issue #300/#301）。
- 親設計正本: [#282](https://github.com/scarecrow173/ruviform/issues/282)
  （汎用 I/O 基盤 横断設計）。
- 実装 Issue: IO-IMP-06 = #300（merge 済み、PR #356）、
  IO-IMP-07 = #301（merge 済み、PR #359）。IO-01 =
  #295/#296。IO-04 = #302/#303/#304。IO-05 = #305。
  合成 E2E = #306。rooted capability = #409。
- Gate 接続: #288（IO-06 cross-consumer gate — 本書の
  証拠項目の一部が同 gate の IO-03 matrix 行の根拠と
  なる）。
- Consumer Issue: AudioLens #139、Synvelis #82/#84/#25
  （状態は 2026-11-09 時点の snapshot）。
- 参照設計文書: `docs/designs/issue-300-…`・`issue-301-…`
  （per-format 正本）、`docs/designs/issue-295-…`・
  `issue-296-…`（IO-01 File 契約）、`docs/designs/
  issue-303-…`・`issue-304-…`（Archive TAR/抽出）、
  `docs/designs/issue-306-…`（合成 E2E）、`docs/designs/
  issue-409-…`（rooted capability）、`docs/designs/
  issue-288-…`（cross-consumer gate）。
