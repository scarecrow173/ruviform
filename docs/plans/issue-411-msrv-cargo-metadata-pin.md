---
id: "PLAN-XVTjRRZhCMBS0tmK8M7Nf5"
type: "plan"
status: "completed"
title: "Issue #411 — cargo_metadata MSRV 修復 実装計画"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/411"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-411-msrv-cargo-metadata-pin.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-411-msrv-cargo-metadata-pin.md"
    - "docs/designs/issue-411-msrv-cargo-metadata-pin.md"
  derived-by:
    - "docs/tasks/issue-411-msrv-cargo-metadata-pin.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-411-msrv-cargo-metadata-pin.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #411 — cargo_metadata MSRV 修復 実装計画

## Goal

`ruviform-ui-test` の `cargo_metadata` 依存を MSRV 1.89 と整合する
0.19 系へ修復し、通常解決で `cargo-platform` が MSRV 超過版へ
再発収束しない恒久状態を作る。guard の型付き検査意味は保持。

## Steps

1. RED 診断: `mise.toml` に `test-ui-test-msrv`（
   `mise exec rust@1.89.0 -- cargo test -p ruviform-ui-test`、
   `target/msrv-ui-test`）を追加し `mise run` で失敗を実測。
2. `crates/ruviform-ui-test/Cargo.toml` の `cargo_metadata` を
   `0.23` → `0.19` へ変更（選定根拠は design 参照）し、
   `mise run check`（= `cargo check --workspace --all-targets`）
   で Cargo.lock を正規再生成。
3. compile/lint の互換差異を最小限調整（`String` 化した name 系の
   `implicit_clone` 抵触 → `.clone()`）。検査意味は変更しない。
4. GREEN: `mise run test-ui-test-msrv` で Rust 1.89 全 target pass。
5. 正規検証: `fmt-check` / `check` / `lint` / `test` /
   `check-ui-driver-boundary` / `test-ui-driver-contract` /
   `test-ofx-msrv`（既存 MSRV lane 退行なし確認）。
6. 直接関連 doc 更新: `docs/development-environment.md` opt-in
   タスク節、crate README の依存節（stale 修正）。
7. main 取込 → lock 再生成 → 独立 review → 最終 head 検証 →
   commit/push/PR。Consumer lock 更新・SV `test-msrv` は root へ
   引き渡し。

## Risks

- 0.19.2 の API 差異で guard の検査意味が変わるリスク:
  fixture 正負 + live workspace semantic test が全て 0.19.2 で
  通ることを GREEN 条件とし、型差のみで意味差が出る場合は
  個別に再評価する（実測済み: 差異なし）。
- 共有 Cargo.lock の他 worker 競合: 再解決は `mise run check`
  の通常経路で行い、merge 後は再度正規再生成する。
- `cargo_metadata` 0.19.x への pin による将来修正の取り込み遅延:
  受け入れる（本 crate は test-support で本番 graph に波及しない）。

## Verification Matrix

| 受入条件 | 検証 |
| --- | --- |
| MSRV 1.89 でビルド/テスト | `mise run test-ui-test-msrv` |
| 通常解決が MSRV 範囲へ収束 | Cargo.lock diff（platform 0.1.9）+ manifest 制約 `^0.1.2` の index 根拠 |
| guard 意味保持 | `check-ui-driver-boundary`（verdict pass）+ `tests/driver_boundary.rs` 10 test |
| 正規検証 | fmt-check / check / lint / test / test-ui-driver-contract / test-ofx-msrv |
| Consumer 修復 | SV 側 lock 更新 + `test-msrv`（別作業・root 管轄、本 PR 対象外） |
