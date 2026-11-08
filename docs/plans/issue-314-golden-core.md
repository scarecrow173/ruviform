---
id: "PLAN-Z43s6oJzXX8u7P4xAKcbau"
type: "plan"
status: "in-progress"
title: "Issue #314 slice 1 — Golden Utility コアの実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-314-golden-core.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/README.md"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-314-golden-core.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-314-golden-core.md"
    - "docs/designs/issue-314-golden-core.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-314-golden-core.md"
    - "docs/test-specs/issue-314-golden-core.md"
  verified-by:
    - "docs/test-specs/issue-314-golden-core.md"
---

# Issue #314 slice 1 — Golden Utility コアの実装計画

## 対象

設計 `docs/designs/issue-314-golden-core.md`（D1〜D7）を実装する。
単一 PR・単一 crate（+ 共有 helper の小規模 refactor）であり、
分割計画は不要。

## ファイルマップ

| ファイル | 変更内容 | 対応設計 |
| --- | --- | --- |
| `crates/ruviform-ui-test/Cargo.toml` | `png = "0.18"`・`base64 = "0.22"` 追加 + 依存追加理由コメント | D3 |
| `crates/ruviform-ui-test/src/lib.rs` | `pub mod golden` + re-export 追加 | D1 |
| `crates/ruviform-ui-test/src/driver_lab.rs` | evidence path 解決・JSONL append を `pub(crate)` helper に抽出（公開 API 不変） | D6 |
| `crates/ruviform-ui-test/src/golden.rs` | module root・`GoldenVerdict`/`GoldenError`・re-export | D1, D6 |
| `crates/ruviform-ui-test/src/golden/key.rs` | `GoldenKey`/`EnvironmentMeta`/`GpuMeta`/`CapturePath` + canonical/file_stem | D2 |
| `crates/ruviform-ui-test/src/golden/image.rs` | `RgbaImage`・PNG codec・`from_screenshot`・次元上限 | D3 |
| `crates/ruviform-ui-test/src/golden/diff.rs` | `ChannelTolerance`/`MaskRect`/`DiffPolicy`/`diff`（統計+差分画像） | D4 |
| `crates/ruviform-ui-test/src/golden/store.rs` | `BaselineStore`/`Baseline`（layout・load・candidate・promote・compare） | D5 |
| `crates/ruviform-ui-test/src/golden/report.rs` | `GoldenReport`/`GoldenEvidence`/evidence append | D6 |
| `crates/ruviform-ui-test/tests/golden.rs` | 統合 test（store round-trip・evidence・screenshot decode・compare verdict） | AC 全般 |
| `crates/ruviform-ui-test/fixtures/golden/README.md` | baseline layout・承認/監査手順の記録 | R8, D5 |
| `crates/ruviform-ui-test/README.md` | golden 節の追加（責務・手順・semantic oracle 境界） | R8, D7 |
| `mise.toml` | `test-ui-golden` task 追加 | 検証 lane |
| `docs/development-environment.md` | task 対応表への `test-ui-golden` 追記 | R8 |
| `Cargo.lock` | `cargo build`/`cargo test` で自動更新 | — |
| `docs/**` | spec/design/plan/task/test-spec | — |

## 依存順序

1. `Cargo.toml` 依存追加 → `cargo check -p ruviform-ui-test` で lock 更新確認。
2. `golden/key.rs`（DTO のみ・依存なし）。
3. `golden/image.rs`（png/base64 使用）。
4. `golden/diff.rs`（image に依存）。
5. `golden/store.rs` + `golden/report.rs`（serde_json 使用）。
6. `golden.rs` root + `lib.rs` export。
7. `driver_lab.rs` helper 抽出 + `report.rs` からの利用。
8. unit tests（各 module 内）→ `tests/golden.rs`。
9. `fixtures/golden/README.md`・crate README・`mise.toml`・
   `development-environment.md`。
10. 検証（下記）→ review-protocol → commit/PR。

## リスク

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `png` 0.18 API の細部（`Encoder`/`Decoder`/`Transformations`）の相違 | 実装手戻り | docs.rs で API 確認済み。コンパイルで早期検証 |
| MSRV 1.89 で `png`/`base64` が解決不能 | `test-ui-test-msrv` 失敗 | 両 crate の rust-version は 1.73/低位で適合見込み。msrv lane で実測し、失敗時は代替（0.17 系等）を検討して報告 |
| boundary guard が新規 dep を違反判定 | `check-ui-driver-boundary` 失敗 | `png`/`base64` は forbiddenDepRoots 非該当。guard 実行で確認 |
| `driver_lab.rs` helper 抽出で既存挙動が変わる | driver-contract 回帰 | 抽出は移動のみ。`test-ui-driver-contract` で確認 |
| f64 `scale_factor` の文字列化が非決定的 | key/file stem 不安定 | `{:?}` format で固定し unit test で固定値を検証 |
| 大きな PNG の decode メモリ | test 環境の OOM | `MAX_DIMENSION`/`MAX_PIXELS` 上限 + `png::Limits` |
| golden verdict と driver-lab verdict の語彙乖離 | consumer-verify 写像で齟齬 | 5 値を完全一致させ、写像規則（`blocked`→`not_run`）は README/設計に記録 |

## ロールバック

変更は追加的（新 module・新 test・新 task・新依存）。`golden` module・
deps・task・docs を除去すれば完全に元へ戻る。`driver_lab.rs` の
helper 抽出は公開 API を変えない純粋 refactor で、revert は容易。
永続データ形式・外部契約の変更はない（`rvf.golden.*` schema は新規）。

## 検証マトリクス

| 段階 | コマンド | 目的 |
| --- | --- | --- |
| 1 | `mise run test-ui-golden` | golden 統合 test（AC-1〜AC-6） |
| 2 | `mise run test` | workspace 全体（lib unit test を含む） |
| 3 | `mise run check-ui-driver-boundary` | AC-7（toolkit 型非含有） |
| 4 | `mise run test-ui-driver-contract` | driver_lab helper 抽出の回帰 |
| 5 | `mise run fmt-check` / `mise run check` / `mise run lint` | 警告ゼロ・整形 |
| 6 | `mise run test-ui-test-msrv` | MSRV 1.89（png/base64 推移依存含む） |
| 7 | doc chain の review-protocol + `doc_status audit` | 文書整合 |
