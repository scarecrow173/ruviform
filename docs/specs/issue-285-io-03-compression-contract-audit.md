---
id: "SPEC-7TCHVfCUzxmF7YWrv1aS4M"
type: "spec"
status: "approved"
title: "Issue #285 IO-03 Compression — 設計正本照合・契約試験棚卸し slice"
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
    - "https://github.com/scarecrow173/ruviform/issues/285"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-285-io-03-compression-serde-boundary.md"
    - "docs/designs/issue-285-io-03-compression-serde-boundary.md"
    - "docs/test-specs/issue-285-io-03-compression-serde-boundary.md"
    - "docs/plans/issue-285-io-03-compression-serde-boundary.md"
    - "docs/tasks/issue-285-io-03-compression-serde-boundary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/designs/issue-306-io-composition-e2e.md"
  refines:
    - "docs/specs/issue-285-io-03-compression-serde-boundary.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-285-io-03-compression-serde-boundary.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #285 IO-03 Compression — 設計正本照合・契約試験棚卸し slice

## What Should Be Built

Issue #285（IO-03 Compression — Gzip/Deflate/Zstd Stream
Adapter・終了・リソース制限）の設計正本 slice。`ruviform-io-compression`
は #300（gzip/raw deflate）・#301（zstd）で実装 merge 済みであり、
設計正本 `docs/designs/issue-285-io-03-compression-serde-boundary.md`
も approved で存在する。本 slice は Issue 受入条件のうち
headless で誠実に実行可能な **「設計正本と実装の照合」「ENC/DEC/
SEC 契約試験の棚卸しと不足分の追加」** に限定して閉じる。

追加するもの:

1. **設計正本照合**: Issue #285 本文の設計（Format 別
   Reader/Writer・所有権・Finish 状態遷移・Backend 型隠蔽・
   リソース制限）と `src/` 実装・既存試験を項目ごとに照合し、
   差分があれば設計正本を実装済みの現実に合わせて更新する。
   照合結果は本 slice の design/IR に証拠つきで記録する。
2. **契約試験の棚卸しと追加**: `tests/` の既存試験を Issue
   受入条件の ENC-01〜03・DEC-01〜03・SEC-01〜02・BUF・
   failure-path 契約へ写像し、不足していた headless 検証可能な
   契約（downstream writer 失敗・finish 時の底流 flush 失敗・
   失敗後の writer 回復・decoder の source I/O 失敗正規化・
   Drop 無書込・flush≠finish・budget ちょうど/+1・検証済み
   decoder 終了と非枯渇）を `crates/ruviform-io-compression/
   tests/` に追加する。既存実装の変更は設計不整合が確認できた
   場合の最小限に留める。
3. **文書チェーン**: 本 slice の spec/design/test-spec/plan/
   task と実装記録（`docs/impl/ir/`）を作成し、managed docs
   index を再生成する。

## Why Now

Issue #285 の実装 Issue（#300/#301）は merge 済みだが、受入条件
の「完全入力/途中 EOF/破損/Checksum/過大展開/Cancel/Writer
Error の契約と試験」「ENC-01〜03・DEC-01〜03・SEC-01〜02」は
実装の存在だけでは閉じず、契約試験の網羅性を棚卸しする必要が
ある。既存試験は主要契約を押さえているが、failure-path
（writer 失敗・flush 失敗・writer 回復・source I/O 失敗）と
flush/finish/Drop の区別・連結単位の独立検証・高圧縮率 payload
での decoded budget など、headless で検証可能な穴が残っていた。
本 slice でその穴を塞ぎ、設計正本と実装・試験の対応を証拠つき
で固定する。

## Users and Value

- Issue #285 受入監査: 「設計正本照合・契約試験棚卸し」の
  実測証拠（追加試験名・mise lane 結果）を docs チェーンと
  PR に集約する。
- 後続 consumer 受入（#288・#219/#294）: compression 層の
  failure-path 契約が試験で pin されていることの根拠。
- 保守者: backend 更新・feature 追加時に再検証すべき契約の
  棚卸し表（test-spec R1〜Rn）が残る。

## Scope

### In Scope

- `crates/ruviform-io-compression/tests/` への契約試験追加
  （`common/mod.rs` の fault fixture 強化を含む）。実装
  `src/` の変更は設計不整合が確認できた場合の最小限のみ。
- `docs/designs/issue-285-io-03-compression-serde-boundary.md`
  の実装現状 snapshot と証拠項目対応表の、本 slice で追加した
  試験・実測を反映した更新。
- 本 doc チェーン（spec/design/test-spec/plan/task/impl）と
  managed docs index の再生成。
- `mise run fmt-check`/`check-io-compression`/`lint-io-compression`/
  `test-io-compression`/`test-io-e2e`/`test-rt`/`test-io-msrv`
  の実走と結果記録。

### Out of Scope

- 性能計測・ベンチマーク・Fuzz 実走（proptest/cargo-fuzz 等の
  長時間 campaign）・両 Consumer（AudioLens #139・Synvelis
  #82/#84）の実 repo 回帰・#288 Integration Gate・Issue #285
  の close — headless/権限外のため NotRun/Unsupported として
  記録のみ。
- `crates/ruviform-io-compression/src/` の公開 API 変更・
  backend 変更・feature 追加・依存追加。
- 他クレート（`ruviform-ui-gpui`・`ruviform-plugin-host`・
  `ruviform-observability`・`ruviform-io-structured` 等、他
  Worker 占有範囲）の変更。
- macOS/Linux 実機・実ネットワーク FS での `zstd` feature
  （vendored C build）経路の実測。

## Requirements

- 追加試験は外部から観測可能な契約（error kind・receipt・
  出力 byte・writer/sink の状態）のみを検証し、実装内部に
  依存しない。feature gate（`gzip`/`deflate`/`zstd`）を維持し、
  `default = []` を壊さない。
- failure-path 試験は「失敗が terminal `Failed`/`InvalidState`
  に latch する」「底流 writer が呼出側へ無傷で返る」「Drop が
  追加 byte を書かない」「部分出力が検証済み完了を偽装しない」
  を区別して検証する。
- 未実施の検証は `NotRun`/`Unsupported` と理由・後続 owner を
  明記し、実施済みと偽装しない。
- 設計正本の更新は実装済みの現実との照合結果のみに基づき、
  未実装の将来像を確定済みとして書かない。

## Acceptance Criteria

- [ ] Issue #285 設計（API・所有権・Finish 状態遷移・Backend
      型隠蔽・リソース制限）と `src/` 実装の照合結果が design
      文書に記録され、差分があれば設計正本が更新されている。
- [ ] ENC-01〜03・DEC-01〜03・SEC-01〜02・BUF に対応する
      headless 契約試験の棚卸し表（test-spec）が存在し、
      不足分が `tests/` に追加され `mise run
      test-io-compression`（feature matrix 全組合せ）で pass
      している。
- [ ] failure-path 試験（writer write 失敗・finish 時底流
      flush 失敗・失敗後 writer 回復・decoder source I/O 失敗・
      Drop 無書込・flush≠finish）が gzip/deflate/zstd 各系で
      存在する。
- [ ] 性能・Fuzz・両 Consumer 回帰・#288 gate・MSRV/OS
      matrix の未実施項目が NotRun/Unsupported として理由と
      後続 owner つきで記録されている。
- [ ] `mise run fmt-check`/`check-io-compression`/
      `lint-io-compression`/`test-io-compression`/`test-io-e2e`/
      `test-rt`/`test-io-msrv` が pass、managed docs index が
      再生成済み。

## Deferred Design Concerns

- 両 Consumer 実測・consumer-verify tuple | #288・#219/#294
  の責務 | consumer 候補 tuple の実走時 | 片側 pass を両
  Consumer 合格と誤認するリスク。
- 性能・バッファ戦略の実測比較 | 本 slice は契約試験のみ |
  #300/#301 設計の性能節・#288 性能 gate | 圧縮率/速度の
  回帰を契約試験では検出できない。
- macOS/Linux `zstd` vendored build・CI OS lane | headless
  Windows のみ | CI 既存 OS lane | OS/C toolchain 差異が
  未検証のまま残る。
- 長時間 Fuzz campaign・backend 更新時の再検証 | per-format
  test-spec の audit 規則 | backend upgrade PR 時 | corrupt
  入力の網羅性が有限試験に留まる。

## Sources and Relations

- 親 Issue: #282（IO 設計親）、#285（本 Issue）、#288（IO-06
  cross-consumer gate）、#300/#301（実装 Issue）。
- 上流 docs: `docs/specs|designs|plans|tasks|test-specs/
  issue-285-io-03-compression-serde-boundary.md`（approved
  チェーン）、`docs/designs/issue-300-*`・`issue-301-*`。
- 関連: `docs/designs/issue-306-io-composition-e2e.md`（合成
  E2E 正本）、`docs/impl/ir/issue-300-*`・`issue-301-*`。
