---
id: "TASK-c2c4wEGmlw9xtzdbWf5HKz"
type: "task"
status: "done"
title: "T1 IO-03 Compression 設計正本照合・契約試験棚卸し・検証"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source: []
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
    - "docs/plans/issue-285-io-03-compression-contract-audit.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-285-io-03-compression-contract-audit.md"
    - "docs/tasks/issue-285-io-03-compression-serde-boundary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-285-io-03-compression-contract-audit.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 IO-03 Compression 設計正本照合・契約試験棚卸し・検証

## Work

`ruviform-io-compression` の実装と Issue #285 設計正本を照合し、
ENC/DEC/SEC/BUF 契約試験を棚卸しして不足分を
`tests/` に追加する。fixture は `tests/common/mod.rs` に集約し、
feature gate を維持する。設計正本の snapshot・証拠索引を実装
現実に合わせて更新し、mise lane で検証する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-285-io-03-compression-contract-audit.md`

## Dependencies

- `docs/tasks/issue-285-io-03-compression-serde-boundary.md`
  （設計正本 slice・done）

## Affected Files

- `crates/ruviform-io-compression/tests/common/mod.rs`
- `crates/ruviform-io-compression/tests/encoder_contract.rs`
- `crates/ruviform-io-compression/tests/decoder_contract.rs`
- `docs/designs/issue-285-io-03-compression-serde-boundary.md`
- 本 doc チェーン

## Done When

- [x] test-spec R1〜R12 の全契約に対応する試験が `tests/` に
      存在し、`mise run test-io-compression`（feature matrix
      全組合せ）で pass
- [x] 設計正本照合の結果が design 文書に記録され、差分があれば
      設計正本が更新済み（結果: `src/` 変更なし・snapshot/
      証拠索引のみ更新）
- [x] `mise run fmt-check`/`check-io-compression`/
      `lint-io-compression`/`test-io-e2e`/`test-rt`/`test-io-msrv`
      が pass
- [x] managed docs index 再生成済み

## Verification

- [x] `mise run test-io-compression` 実走（5 feature 組合せ）
- [x] `mise run check-io-compression`/`lint-io-compression`
      実走（`-D warnings`）
- [x] `mise run test-io-e2e`/`test-rt`/`test-io-msrv` 実走
- [x] `cargo test -p ruviform-io-compression --all-features`
      実走（補助証拠・canonical は mise）

## 内容

既存 `tests/` を Issue 受入条件の ENC/DEC/SEC/BUF 契約へ写像
して棚卸しし、failure-path（writer write 失敗・finish 時底流
flush 失敗・失敗後 writer 回復・decoder source I/O 失敗）・
Drop 無書込・flush≠finish・budget ちょうど/+1・高圧縮率
payload・連結単位の独立検証・`NotExhausted` 非破壊性の不足を
`encoder_contract.rs`/`decoder_contract.rs` に追加した。
`common/mod.rs` の `FailAfter` に受理 byte 保持を加え、
`FailFlush`/`RecoveringWriter`/`FailingReader` を新設した。
照合の結果 `src/` の設計不整合は確認されず実装変更はなし。

## 完了条件

- test-spec R1〜R12 の全判定が実測で満たされる
- 未実施項目（性能・Fuzz・Consumer 回帰・#288 gate・OS
  matrix）が NotRun/Unsupported として理由・owner つきで
  記録される
- 公開 API・feature・依存・他クレートを変更していない
