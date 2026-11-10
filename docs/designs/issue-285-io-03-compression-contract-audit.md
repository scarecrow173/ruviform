---
id: "DESIGN-8C8VqGGcRx0CT4AfwBMnLR"
type: "design"
status: "approved"
title: "Issue #285 IO-03 Compression — 設計正本照合・契約試験棚卸し 設計"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/285"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-285-io-03-compression-contract-audit.md"
      - type: "file"
        path: "docs/designs/issue-285-io-03-compression-contract-audit.md"
      - type: "file"
        path: "docs/test-specs/issue-285-io-03-compression-contract-audit.md"
      - type: "file"
        path: "docs/plans/issue-285-io-03-compression-contract-audit.md"
      - type: "file"
        path: "docs/tasks/issue-285-t1-contract-audit.md"
      - type: "file"
        path: "docs/impl/ir/issue-285-io-03-compression-contract-audit.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-compression/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/encoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/decoder_contract.rs"
      - type: "file"
        path: "docs/designs/issue-285-io-03-compression-serde-boundary.md"
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
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/285"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-285-io-03-compression-contract-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-285-io-03-compression-serde-boundary.md"
    - "docs/designs/issue-306-io-composition-e2e.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines:
    - "docs/designs/issue-285-io-03-compression-serde-boundary.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-285-io-03-compression-contract-audit.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #285 IO-03 Compression — 設計正本照合・契約試験棚卸し 設計

## Context

- 上流: [spec](../specs/issue-285-io-03-compression-contract-audit.md)
- 正本設計: [issue-285 serde-boundary](../designs/issue-285-io-03-compression-serde-boundary.md)
  （approved・実装 merge 済みの設計正本）
- 実装: `crates/ruviform-io-compression`（#300 gzip/raw deflate・
  #301 zstd で merge 済み）

本書は「Issue #285 設計と実装の照合結果」と「契約試験棚卸しで
追加した試験の設計判断」を記録する。新しい圧縮アーキテクチャを
発明するものではなく、approved 設計正本への適合を証拠で固定
する slice である。

## 照合結果サマリ（設計正本 vs 実装）

`src/` 全 8 ファイルと `tests/` 全 8 ファイルを approved 設計
正本の各契約節へ写像して照合した。**公開 API・状態機械・
error 語彙・budget 意味論は設計正本と一致しており、`src/`
の変更は不要と判断した**（設計不整合は確認されなかった）。

| 設計正本の契約 | 実装の対応 | 判定 |
| --- | --- | --- |
| `Encoder<'a, W: Write>` 借用・`Decoder<'a, R: BufRead>` 借用 | `src/{gzip,deflate,zstd}.rs` + `src/stream.rs` | 一致 |
| `finish()` が完了証明・冪等・`Failed` latch・Drop 無書込 | `src/stream.rs` 状態機械 + `tests/*_contract.rs` | 一致（本 slice で failure-path を追加 pin） |
| `flush()` は pending 伝搬のみ・完了証明でない | `src/stream.rs` `flush` 実装 | 一致（`flush_mid_stream_then_finish_still_decodes` で補強） |
| `CompressionError`/`CompressionErrorKind` 12 variant・backend error は `source()` のみ・kind 正規化 | `src/error.rs` | 一致 |
| `EncodeLimits`/`DecodeLimits` 累積 byte/unit 予算・`max_window_log`（zstd） | `src/options.rs` + per-format decoder | 一致（ちょうど/+1・高圧縮率を追加 pin） |
| `DecodeScope::WholeInput`/`SingleUnit`・連結 member/frame 累積・trailing 拒否 | `src/stream.rs` + per-format | 一致（`chained_decoder_*`・`sequential_encoders_*` を追加 pin） |
| `IntegritySummary` checked/unchecked 分離 | `src/receipt.rs` | 一致 |
| backend 型・version・feature 名の公開 API 非露出 | `src/lib.rs` 公開面 + `tests/api_surface.rs` | 一致 |
| 非 RT・`std::io` 合成のみ・archive/file 非依存 | crate 依存 `Cargo.toml`（flate2/zstd-safe のみ optional） | 一致 |

## 契約試験棚卸し → 追加試験の対応

既存試験で未カバーだった headless 契約を、外部観測可能な振る舞い
（error kind・receipt・sink の byte 列・writer/sink の再利用可否）
のみで固定した。fixture は `tests/common/mod.rs` に集約。

### Encoder 系（`tests/encoder_contract.rs`）

| 追加試験（format 別） | 契約 |
| --- | --- |
| `new_fails_when_header_write_fails`（gzip） | header 書込失敗は構築時 `Io`・底流は呼出側に残る |
| `header_write_respects_output_budget`（gzip） | header も output budget に従い部分 header を出さない |
| `flush_after_finish_is_ok`（gzip） | `Finished` の flush は no-op（積層 writer の無条件 flush を許容） |
| `flush_mid_stream_then_finish_still_decodes`（全形式） | 途中 flush は完了でなく member/frame を壊さない |
| `failed_encoder_drop_writes_no_trailer`/`_no_final_block`/`_no_epilogue` | terminal 失敗後の Drop は追加 byte を書かない・部分出力は検証を通らない |
| `failed_encoder_releases_usable_writer`（全形式） | `Failed` latch 後に底流 writer が無傷で再利用可能・部分出力は呼出側所有 |
| `finish_fails_when_underlying_flush_fails`（全形式） | `finish` 末尾の底流 flush 失敗は `Io` で latch |
| `sequential_encoders_concatenate_members_on_one_writer`/`_frames`（gzip/zstd） | 単一 writer 上の連続 encoder が有効な連結 stream を作る |
| `write_after_finish_rejected_but_finish_stays_valid`（deflate/zstd） | `Finished` 後の write は `InvalidState`・finish 冪等は維持 |
| `output_budget_exact_boundary_succeeds`（zstd） | output budget ちょうどは成功 |

### Decoder 系（`tests/decoder_contract.rs`）

| 追加試験（format 別） | 契約 |
| --- | --- |
| `finish_is_idempotent_after_verified_end`（全形式） | 検証済み `finish` は receipt を返し冪等 |
| `chained_decoder_resumes_at_member_boundary`/`_decodes_next_stream`/`_decodes_next_frame` | `SingleUnit` が次単位の byte を底流に残し、新 decoder が独立検証できる |
| `read_error_latches_failed`（全形式） | source の mid-stream I/O 失敗は `Io` に正規化され terminal `Failed` に latch |
| `decoded_budget_enforced_on_high_ratio_payload`（全形式） | ~1 MiB zeros（実圧縮 ~1/100 以下）の decoded cap がちょうど/+1 で強制される |
| `finish_mid_output_is_not_exhausted`（deflate/zstd） | 未 drain の `finish` は `NotExhausted` で状態を壊さず、その後 drain→verified end に到達 |

### Fixture（`tests/common/mod.rs`）

- `FailAfter` に `inner: Vec<u8>` を追加 — 失敗前に受理した
  byte を保持し「部分出力が検証を通らない」「受理 prefix のみ
  が sink に残る」を直接 assert 可能にした。
- `FailFlush`（write は全受理・flush は常に `BrokenPipe`）—
  `finish` 末尾の底流 flush 失敗経路を駆動。
- `RecoveringWriter`（`new(fail_calls)` = 先頭 N 回失敗、
  `after_ok(ok, fail)` = 先頭 ok 回成功→fail 回失敗→以降受理）
  — 失敗後の底流再利用を検証。
- `FailingReader`（`fail_at` byte 以降 `fill_buf` が
  `ConnectionReset`）— decoder の source I/O 失敗伝播を駆動。

## 設計正本の更新

`docs/designs/issue-285-io-03-compression-serde-boundary.md` は
実装 merge 後に approved 済みで、契約記述は実装と一致していた。
本 slice では次のみを更新した（契約内容の変更なし）:

- Context の「実装の現状」snapshot に、本 slice（2026-11-10）
  の契約試験棚卸し・追加試験・検証 lane 実測を追記。
- 証拠項目対応表の ENC/DEC/SEC/BUF 各行に、本 slice で追加した
  試験名を併記（既存正本への参照は維持）。

## 検証方針

- 正規入口は `mise.toml` タスク。実施済み: `fmt-check`・
  `check-io-compression`・`lint-io-compression`・
  `test-io-compression`（default/gzip/deflate/zstd/結合の
  5 組合せ）・`test-io-e2e`・`test-rt`・`test-io-msrv`
  （rust@1.89.0・io-compression `--features deflate` 補完を含む）。
- 未実施（NotRun/Unsupported）: 性能計測・Fuzz campaign・両
  Consumer 実 repo 回帰・#288 Integration Gate・macOS/Linux
  実機 `zstd` vendored build・Issue close。理由と後続 owner は
  spec の Deferred Design Concerns と IR に記録。

## Risks and Trade-offs

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | 追加試験が実装内部に依存し将来の正当な変更を壊す | low | med | 外部観測（error kind・receipt・sink byte・再利用可否）のみを assert。内部状態・private API に触れない |
| 2 | fault fixture の失敗タイミング仮定（write 回数）が backend 更新でずれる | med | low | 失敗は「最初に byte を吐く呼出」で駆動し、回数は fixture 引数で明示。ずれた場合は試験が fail して検出される（偽装 pass しない） |
| 3 | 高圧縮率 fixture（1 MiB zeros）が遅い・メモリを食う | low | low | 1 MiB は headless で ~0.1s 未満。上限は decoded bytes であり生成側の圧縮率のみが前提 |
| 4 | docs 更新が「Consumer 受入完了」と誤解される | low | med | spec/IR で実施済み検証と NotRun を分離記録。Consumer 実測は #288/#219/#294 の段階 |

## Deferred Design Concerns

- 両 Consumer 実測・consumer-verify tuple | #288・#219/#294
  の責務 | consumer 候補 tuple の実走時 | 片側 pass を両
  Consumer 合格と誤認するリスク。
- 性能・バッファ戦略の実測 | 本 slice は契約試験のみ | #300/
  #301 設計の性能節・#288 性能 gate | 圧縮率/速度回帰を契約
  試験で検出できない。
- macOS/Linux `zstd` vendored build・CI OS lane | headless
  Windows のみ | CI 既存 OS lane | OS/C toolchain 差異が
  未検証のまま残る。
- 長時間 Fuzz・backend 更新時の再検証 | per-format test-spec
  の audit 規則 | backend upgrade PR 時 | corrupt 入力網羅性
  が有限試験に留まる。

## Verification Notes

- [x] 照合結果サマリ表が設計正本の各契約節へ対応し、差分なし
      の判定根拠（`src/` 各 file・既存 test file）を持つ。
- [x] 追加試験表が test-spec の R1〜Rn と 1:1 に対応し、
      feature gate を維持している。
- [x] `src/` の変更が「設計不整合が確認できた最小限」の条件を
      満たすか（結果: 変更なし）。
- [x] 未実施項目が NotRun/Unsupported として理由・owner つきで
      spec/IR に記録されている。
