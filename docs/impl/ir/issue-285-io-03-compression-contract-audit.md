---
id: "IMPL-BneVKCjBRwNA5pxIVfIZ6X"
type: "impl"
status: "completed"
title: "Issue #285 IO-03 Compression — 設計正本照合・契約試験棚卸し"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/282"
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
    - "docs/tasks/issue-285-t1-contract-audit.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-300-ruviform-io-compression-gzip-raw-deflate-compression-core.md"
    - "docs/impl/ir/issue-301-io-compression-zstd.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-285-io-03-compression-contract-audit.md"
    - "docs/designs/issue-285-io-03-compression-contract-audit.md"
    - "docs/specs/issue-285-io-03-compression-contract-audit.md"
    - "docs/test-specs/issue-285-io-03-compression-contract-audit.md"
    - "docs/designs/issue-285-io-03-compression-serde-boundary.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-285-io-03-compression-contract-audit.md"
  verified-by: []
  references: []
  defers:
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #285 IO-03 Compression — 設計正本照合・契約試験棚卸し

## Summary

Issue #285 受入条件の「設計正本照合」「ENC/DEC/SEC 契約試験の
棚卸しと不足分追加」を閉じた。`src/` 全 8 ファイルを approved
設計正本へ写像して照合し、公開 API・状態機械・error 語彙・
budget 意味論の設計不整合は確認されなかったため実装変更は
なし。`tests/` の既存試験を契約へ写像して棚卸しし、
failure-path・flush/finish/Drop 区別・連結単位境界・高圧縮率
budget・`NotExhausted` 非破壊性の不足を
`encoder_contract.rs`/`decoder_contract.rs` に追加（gzip/
deflate/zstd 各系）、`common/mod.rs` に `FailFlush`・
`RecoveringWriter`・`FailingReader` fixture を新設して
`FailAfter` を受理 byte 保持へ強化した。設計正本
`docs/designs/issue-285-io-03-compression-serde-boundary.md`
は snapshot と証拠項目対応表へ本 slice の試験・実測を追記
した（契約内容の変更なし）。

## Context

- 上流: [spec](../../specs/issue-285-io-03-compression-contract-audit.md)・
  [design](../../designs/issue-285-io-03-compression-contract-audit.md)・
  [test-spec](../../test-specs/issue-285-io-03-compression-contract-audit.md)・
  [plan](../../plans/issue-285-io-03-compression-contract-audit.md)・
  [task](../../tasks/issue-285-t1-contract-audit.md)
- 正本設計: `docs/designs/issue-285-io-03-compression-serde-boundary.md`
- 実装: `crates/ruviform-io-compression`（#300/#301 merge 済み）

## Implementation

### 照合

- `src/{lib,error,options,receipt,stream,gzip,deflate,zstd}.rs`
  を設計正本の「Serde 境界」「Format/Version 契約」
  「Finish/状態機械/所有権」「Resource Budget」「Error 契約」
  「Backend 選定」各節へ写像。差分なしを確認。
- `tests/` 既存 8 file を ENC/DEC/SEC/BUF 証拠項目へ写像し、
  failure-path と境界条件の穴を特定。

### Changes（test-spec R1〜R12 対応）

- Encoder（`tests/encoder_contract.rs`、gzip/deflate/zstd 各
  系）: `new_fails_when_header_write_fails`・
  `header_write_respects_output_budget`・
  `flush_after_finish_is_ok`・
  `flush_mid_stream_then_finish_still_decodes`・
  `failed_encoder_drop_writes_no_trailer`/`_no_final_block`/
  `_no_epilogue`・`failed_encoder_releases_usable_writer`・
  `finish_fails_when_underlying_flush_fails`・
  `sequential_encoders_concatenate_members_on_one_writer`/
  `_frames`・`write_after_finish_rejected_but_finish_stays_valid`・
  `output_budget_exact_boundary_succeeds`（zstd）。
- Decoder（`tests/decoder_contract.rs`、gzip/deflate/zstd 各
  系）: `finish_is_idempotent_after_verified_end`・
  `chained_decoder_resumes_at_member_boundary`/
  `chained_decoder_decodes_next_stream`/
  `chained_decoder_decodes_next_frame`・`read_error_latches_failed`・
  `decoded_budget_enforced_on_high_ratio_payload`（~1 MiB
  zeros・実圧縮 <1/100）・`finish_mid_output_is_not_exhausted`。
- Fixture（`tests/common/mod.rs`）: `FailAfter.inner` で受理
  byte を保持、`FailFlush`（flush 常時失敗）・
  `RecoveringWriter`（`new`/`after_ok` で失敗窓を制御）・
  `FailingReader`（`fail_at` 以降 `ConnectionReset`）を新設。

### 設計正本更新

- `docs/designs/issue-285-io-03-compression-serde-boundary.md`:
  Context に本 slice の棚卸し・追加試験・検証 lane 実測を追記、
  証拠項目対応表（ENC-02/03/04/05・DEC-01/02・SEC-01/02/04・
  BUF-01）へ新試験名を併記。契約記述は無変更。

### Decisions During Implementation

- `src/` 無変更: 照合で設計不整合が確認されなかったため、
  task の「既存実装の変更は設計不整合が確認できた最小限」の
  条件に従い test/docs のみを変更した。
- fault fixture は write 回数ではなく「最初に byte を吐く
  呼出」で失敗を駆動し、回数は引数で明示 — backend 更新で
  emission 粒度が変わっても試験が fail して検出される
  （偽装 pass しない）。
- 高圧縮率 budget 試験は ~1 MiB zeros（実圧縮 <1/100 を assert）
  で「本当に展開が起きる」ことを fixture 自体で保証した。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

実測値（mise canonical lane）:

| コマンド | OS | toolchain | 結果 |
| --- | --- | --- | --- |
| `mise run fmt-check` | Windows 11 x64 | cargo 1.99.0 | exit 0 |
| `mise run check-io-compression` | 同上 | 同上 | exit 0（default/gzip/deflate/zstd/結合の 5 組合せ） |
| `mise run lint-io-compression` | 同上 | 同上 | exit 0（`-D warnings`・5 組合せ） |
| `mise run test-io-compression` | 同上 | 同上 | exit 0（5 組合せ・45 test-result ok・失敗 0） |
| `mise run test-io-e2e` | 同上 | 同上 | exit 0（合成境界・layered completion proofs 含む全 suite） |
| `mise run test-rt` | 同上 | 同上 | exit 0 |
| `mise run test-io-msrv` | 同上 | rust@1.89.0 | exit 0（io-compression `--features deflate` 補完を含む） |
| `mise run check` / `lint` / `test`（workspace） | 同上 | cargo 1.99.0 | exit 0 |

補助証拠（canonical = mise）: `cargo test -p
ruviform-io-compression --all-features` exit 0（unit/
integration 149 + doctests 9 = 158 pass）。

## Risks

- `RecoveringWriter::after_ok` の失敗窓は write 呼出回数基準
  であり、encoder が内部 buffer する形式（deflate の小 write）
  では「失敗させたい呼出」の特定が emission タイミング依存に
  なる。現行試験は flush/finish を挟んで必ず emission を
  起こす構成で回避している。
- `FailingReader` は `fill_buf` のみ失敗させ、`read` 経路は
  `fill_buf` 経由で同じ失敗を観測する。`read` を直接 override
  する backend 側の別経路は対象外（decoder は `BufRead`
  契約で `fill_buf`/`consume` を使う）。
- 検証は headless Windows のみ。cross-OS 実機（macOS/Linux の
  `zstd` vendored build）・実ネットワーク FS・両 Consumer 実
  repo での回帰は本 slice では未検証（Follow-ups 参照）。

## Follow-ups

- 性能計測・ベンチマーク・長時間 Fuzz campaign — 本 slice は
  headless 契約試験のみ。後続: #300/#301 設計の性能節・#288
  性能 gate・per-format test-spec の audit 規則。
- 両 Consumer（AudioLens #139・Synvelis #82/#84）実 repo での
  Round-trip/保存結果回帰 — consumer-verify（#219/#294）と
  #288 gate の責務（`Unsupported`: 本 worktree の権限外）。
- #288 Integration Gate・cross-OS 実機（macOS/Linux `zstd`
  vendored build）・実ネットワーク FS — headless Windows の
  み。後続: CI 既存 OS lane・#288。
- `Cancelled` kind の上位 Job 連携 — 上位 Job/Consumer 実装時
  （設計正本の Deferred と同一）。
- Issue #285 の close — Coordinator が PR merge 後に判断。

## References

- [spec](../../specs/issue-285-io-03-compression-contract-audit.md)
- [design](../../designs/issue-285-io-03-compression-contract-audit.md)
- [test-spec](../../test-specs/issue-285-io-03-compression-contract-audit.md)
- [plan](../../plans/issue-285-io-03-compression-contract-audit.md)
- [task](../../tasks/issue-285-t1-contract-audit.md)
- 正本設計: `docs/designs/issue-285-io-03-compression-serde-boundary.md`
- `crates/ruviform-io-compression/tests/{common/mod,encoder_contract,decoder_contract}.rs`
