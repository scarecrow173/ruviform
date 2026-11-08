---
id: "SPEC-0k38sOnYLZQngydDDLygHu"
type: "spec"
status: "proposed"
title: "Issue #314 slice 1 — headless Pixel Golden Utility コア"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/464"
    - "https://github.com/scarecrow173/ruviform/issues/426"
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-314-golden-core.md"
      - type: "file"
        path: "docs/designs/issue-314-golden-core.md"
      - type: "file"
        path: "docs/plans/issue-314-golden-core.md"
      - type: "file"
        path: "docs/tasks/issue-314-golden-core.md"
      - type: "file"
        path: "docs/test-specs/issue-314-golden-core.md"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/key.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/image.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/diff.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/store.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/golden/report.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/golden.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/golden/README.md"
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-test/README.md"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-390-driver-contract-lab.md"
    - "docs/designs/issue-390-driver-contract-lab.md"
  blocks: []
  supersedes: []
  superseded-by: []
  derives-from:
    - "docs/specs/issue-390-driver-contract-lab.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-314-golden-core.md"
  related:
    - "docs/designs/issue-390-driver-contract-lab.md"
    - "docs/specs/issue-310-browser-gui-test-harness.md"
---

# Issue #314 slice 1 — headless Pixel Golden Utility コア

## 背景

Issue #314（RF-GT-05）は GPUI Native の Semantic/Visual Golden・Offscreen /
Viz Regression を扱う実装 Issue であり、規模が大きいため Coordinator が
slice 化した。本 spec は **slice 1（headless Golden Utility コア）** のみを
対象とし、実 pixel の capture 経路（offscreen 実測・`TestIo::screenshot`
の実 backend）は slice 3、`VisualGuiTest`/`VisualCapture` 能力契約の
#464 接続は slice 2、#426 Story fixture 接続は slice 4 とする。

Issue 本文の確定契約:

- 本 Issue の Owner は「実際に得た pixel screenshot の取得・環境正規化・
  baseline 比較・差分承認・証拠」のみ。`GpuiObserver::drawn()` の DrawPlan
  は semantic/描画計画であり pixel ではない — 描画 plan 比較の pass を
  Golden の pass にしない。
- Golden key は Story ID + Theme + State + Driver/OS/GPU class +
  DPI/font + Revision を維持する。比較許容差・mask・diff approval・
  audit を必須とする。
- Screenshot 未取得を成功扱いせず `Unsupported`/`NotRun` を記録する。
- 数値ソース・LOD・Receipt・RT-safety・AudioLens の f64/NaN/gap・
  Synvelis の RT/Receipt は独立した semantic oracle に委譲し、
  pixel 一致で代替しない。
- baseline 更新は diff approval/audit を必須とする。

## 問題

- pixel screenshot を baseline と比較する共通 utility（key・環境 metadata・
  画像 decode・diff・baseline store・証拠出力）が存在しない。
- `TestIo::screenshot` / `proto::Screenshot` の seam は #312 で存在するが、
  headless backend は `unavailable` を返すのみで、画像を decode/比較する
  経路がない。
- `ruviform-ui-theme` の golden は正規化 JSON であり pixel ではない
  （混同禁止）。driver-lab の判定語彙・evidence 形式は存在するが
  pixel 比較結果を記述する schema がない。
- baseline の repo 配置・更新承認手順が未定義であり、無承認の baseline
  差し替えを機械的に防ぐ境界がない。

## 目的

- driver-neutral な Golden Utility コアを `ruviform-ui-test` に実装し、
  capture 経路（slice 3）と consumer 接続（slice 2/4）が依存できる
  型・手続きを確定する。
- 合成画像のみで diff・mask・許容差・key 照合・baseline 読み書きを
  決定的に検証できること（実描画・実 OS に依存しない）。
- 「pixel pass は semantic pass を意味しない」境界を型と文書で固定する。

## 要件

- R1: `GoldenKey` DTO — Story ID + Theme + State + Driver class +
  OS class + GPU class + scale factor(DPI) + font class + Revision。
  canonical 文字列（evidence 用）と file system 安全な stem を生成し、
  serde round-trip 可能。GPUI/wry 等の toolkit 型を含まない。
- R2: `EnvironmentMeta` DTO — capture 時の環境証拠（os/arch/backend/
  gpu/scale/font/capture path/revision）。取得不能な値は `None` で
  記録し、捏造しない。
- R3: `RgbaImage` — RGBA8 画素バッファ。`from_png`/`to_png`
  （`png` crate 経由、8bit RGB/RGBA 系への正規化）、
  `from_screenshot`（`proto::Screenshot` の `png`/`rgba-raw` を
  base64 decode して変換）を提供。次元・長さを検証する。
- R4: `DiffPolicy` — per-channel `u8` 許容差（r/g/b/a）と region mask
  （画素座標 rect の集合）。`diff` 純粋関数が統計（diff 画素数・
  比較対象画素数・masked 画素数・channel 別最大差分・差分 bbox）と
  差分画像（差分画素=赤、masked=減光）を返す。
- R5: 判定語彙 `Pass/Fail/NotRun/Unsupported/Blocked` — driver-lab の
  5 値語彙と整合。pixel 比較が測定した結果のみ `Pass`/`Fail` を出し、
  baseline 不在・比較対象 0 画素は `Blocked`（理由付き）、サイズ不一致は
  `Fail`。`NotRun`/`Unsupported` は capture 層が screenshot 未取得・
  非対応を記録するための語彙として型を提供する。
- R6: `BaselineStore` — repo 内 baseline 配置
  `<root>/<story>/<theme>/<state>/<stem>.png` + `<stem>.json`
  sidecar（key・environment・policy・schema version）。load は
  sidecar の key と要求 key の一致を検証する。candidate 出力
  （`write_candidate`）と承認済み配置への `promote` を分離し、
  自動更新経路（update env 等）は提供しない。
- R7: evidence — `rvf.golden.evidence/v1` JSONL 行（key・environment・
  verdict・理由・diff 統計・baseline/artifact path）を
  `RUVIFORM_GOLDEN_EVIDENCE` 指定時に append。driver-lab evidence と
  同じ「生成物は commit しない」運用。
- R8: 文書 — 共通 Golden 出力/比較/レビュー手順、Environment Metadata
  の項目、Pixel 差分条件、semantic Oracle 参照境界を
  `docs/` と crate README/fixture README に記録する。
- R9: 境界 — `gpui`/`wry`/OS handle 型を公開 signature に含めない
  （`check-ui-driver-boundary` で機械検査）。`ruviform-ui-test` は
  test-only のまま（本番クレートから誰も依存しない）。

## 受入条件

| ID | 条件 | 検証 |
| --- | --- | --- |
| AC-1 | `GoldenKey` が canonical 文字列・file stem・serde round-trip を持ち、全要素の差異が key 差異になる | `golden_key_*` unit tests |
| AC-2 | `RgbaImage` が PNG encode/decode round-trip と `Screenshot`（png/rgba-raw）decode を行い、不正次元・長さ・format を拒否する | `golden_image_*` unit tests |
| AC-3 | per-channel 許容差・mask・差分統計・差分画像が仕様どおり（境界値・mask 除外・全透明画素の扱いを含む） | `golden_diff_*` unit tests |
| AC-6 | `BaselineStore` が layout・sidecar 検証・candidate/promote を行い、key 不一致 sidecar を拒否する | `golden_store_*` tests |
| AC-4 | baseline 不在は `Blocked`、サイズ不一致は `Fail`、許容差内は `Pass`、超過は `Fail` となり、理由が report/evidence に残る | `golden_compare_*` tests |
| AC-5 | evidence JSONL が `rvf.golden.evidence/v1` schema で append され、未設定時は副作用がない | `golden_evidence_*` tests |
| AC-7 | 公開 signature に toolkit/GPUI 型が現れない | `mise run check-ui-driver-boundary` |
| AC-8 | DoD の文書化項目（共通手順・Environment Metadata・Pixel 差分条件・semantic Oracle 参照）が docs に存在する | doc review |

## 範囲外

- pixel capture 経路: offscreen 実測・`TestIo::screenshot` の実 backend
  実装・実 Window capture（slice 3 / #313 系）。本 slice では capture が
  `unavailable`/`Unsupported` の場合の記録語彙のみを用意する。
- `VisualGuiTest`/`VisualCapture` 能力契約の #464 への接続（slice 2）。
- #426 Story fixture との接続・AudioLens/Synvelis 両 Theme fixture の
  pixel 適用（slice 4）。
- semantic oracle（f64/NaN/gap/PTS/RT/Receipt）の実装 — 独立層の責務。
- `crates/ruviform-ui-gpui/**`・`crates/ruviform-ui/**`・consumer repo・
  `ui/**`・`poc/**` の変更。
- baseline の初回大量生成（実 capture がないため不可能 — slice 3 以降）。

## 制約

- 非 RT（test/検証基盤）。RT クレートから参照されない。
- MSRV 1.89、`workspace.lints` 準拠、公開 enum は `#[non_exhaustive]`。
- 新規依存は `png`（PNG codec）と `base64`（`Screenshot.data_base64`
  decode）のみ。いずれも既に lock tree 内（gpui 系経由）に存在し、
  理由は `Cargo.toml` の依存追加記録と PR 本文に記載する。
- `Screenshot` 未取得・decode 失敗を成功扱いしない。
