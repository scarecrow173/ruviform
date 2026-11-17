---
id: "IMPL-034ZhkUf1QEsjVLxXSiGbt"
type: "impl"
status: "completed"
title: "Issue #759 plugin-clap host.rs f64 as u32 cast — lint lane 修復"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-759-clippy-cast.md"
    - "docs/discovery/issue-759-clippy-cast.md"
    - "evidence/rf759/verification.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-759-clippy-cast.md"
      - type: "file"
        path: "docs/impl/exp/issue-759-clippy-cast.md"
      - type: "file"
        path: "docs/impl/exp/issue-759-clippy-cast-exp.jsonl"
      - type: "file"
        path: "evidence/rf759/verification.md"
      - type: "file"
        path: "evidence/rf759/lint-plugin-test-clap.txt"
      - type: "file"
        path: "evidence/rf759/lint-plugin-clap.txt"
      - type: "file"
        path: "evidence/rf759/check-plugin-clap.txt"
      - type: "file"
        path: "evidence/rf759/check-plugin-test-clap.txt"
      - type: "file"
        path: "evidence/rf759/test-plugin-clap.txt"
      - type: "file"
        path: "evidence/rf759/test-plugin-test-clap.txt"
      - type: "file"
        path: "evidence/rf759/test-l1-real-plugin.txt"
      - type: "file"
        path: "evidence/rf759/fmt-check.txt"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-759-clippy-cast.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-759-clippy-cast.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-759-clippy-cast.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: ["docs/impl/exp/issue-759-clippy-cast-exp.jsonl"]
    rejected: []
---

# Issue #759 plugin-clap host.rs f64 as u32 cast — lint lane 修復

## Summary

`crates/ruviform-plugin-clap/src/host.rs` の `set_gui_size` にあった
plugin-core `GuiSize { f64 }` → clack `GuiSize { u32 }` の `as`
cast 2 箇所を、境界契約を持つ `gui_size_component(f64) -> u32`
ヘルパーへ集約して修復した。採用契約は **最近接ピクセルへの round
+ `[0, u32::MAX]` への飽和** — 負値・NaN・非有限 → `0`、
`u32::MAX` 超過 → `u32::MAX` で、verb を全域で定義し異常サイズの
最終判定は plugin の `set_size` accept/decline に委ねる
（VST3 adapter の `ViewRect` 飽和写像と整合）。cast lint は clamp
で実際に安全化した上で `#[expect]` + doc comment の契約明記により
局所許可し、`#[allow]` は使用していない。境界契約を固定する unit
test `gui_size_component_saturates_at_pixel_bounds` を host feature
の test module に追加した。`mise run lint
ruviform-plugin-test-clap` を含む全指定検証が exit 0。

## Context

`docs/tasks/issue-759-clippy-cast.md`（Issue `ruviform#759` の
 defect slice）に基づく。spec 相当の要件は Issue 本文の受入条件で
確定済みで、境界契約の選定根拠は
`docs/discovery/issue-759-clippy-cast.md` に記録した。
`ruviform#754`（merge `0450f92f`）で導入された editor verbs 実装の
lint 破損を修復する — `ruviform-plugin-clap` の `host` feature
配下のコードのみを変更し、plugin-core / plugin-vst3 / test fixture
は無変更。

## Implementation

### Changes

- `crates/ruviform-plugin-clap/src/host.rs`:
  - `gui_size_component(value: f64) -> u32` を `unit_f64` の隣に
    追加。`value.round().clamp(0.0, f64::from(u32::MAX)) as u32`
    で最近接ピクセルへの round と `[0, u32::MAX]` 飽和を実装し、
    境界契約を doc comment に明記。`#[expect(clippy::cast_possible_truncation,
    clippy::cast_sign_loss)]` で残存する構文 cast lint を局所許可。
  - `set_gui_size` の `ClapGuiSize { width: size.width as u32,
    height: size.height as u32 }` を `gui_size_component` 呼出へ
    置換し、doc comment を「truncate」から round + 飽和の契約記述へ
    更新。
  - host feature の `mod tests` に
    `gui_size_component_saturates_at_pixel_bounds` を追加 —
    800.4→800 / 800.5→801（round）、-3.0/NaN/-inf→0、
    +inf/1e30/`u32::MAX`+0.4→`u32::MAX` を assert。
- doc chain（最小構成）: `docs/discovery/`、`docs/tasks/`、
  `docs/impl/ir/`、`docs/impl/exp/`（md + jsonl）を新規作成。
- `evidence/rf759/`: 検証コマンドのログと
  `verification.md`（コマンド・exit code・境界契約の記録）。

### Decisions During Implementation

- **境界契約に飽和写像を採用（拒否ではなく）**: neutral `GuiSize`
  に定義域の明文化がなく `HostError::InvalidValue` による拒否の
  根拠が弱いこと、兄弟 VST3 adapter が同一 verb に飽和写像を
  適用済みで adapter 間の挙動分岐を避けることが理由。最終判定は
  plugin の `set_size` に残るため verb の正直性は保たれる。
- **round の採用**: 既存コードは `as` の truncate だったが、
  VST3 adapter の `.round()` と丸めを揃え、論理単位→ピクセルの
  写像として最近接がより正確と判断。in-range の丸め以外の
  観測可能な差は境界値のみ。
- **`f64::from(u32::MAX)`**: 初回実装の `u32::MAX as f64` が
  `clippy::cast_lossless` に抵触したため `From` 変換へ修正
  （`unit_f64` の既存スタイルと一致、実害なしの lint 連鎖を解消）。
- **`#[expect]` の選択**: clamp 後も clippy は値域を追跡せず cast
  lint を発火するため、helper 関数に `#[expect]` + 理由を局所許可。
  `#[expect]` は lint が将来発火しなくなれば unfulfilled
  expectation として検出されるため恒久的な隠蔽にならない。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-759-clippy-cast-exp.jsonl`:
  lint 帰属の実測確認、飽和 vs 拒否の契約選定、`#[expect]` 範囲と
  `cast_lossless` 連鎖の観測、境界 test の固定を採用。

### Rejected

- `HostError::InvalidValue` による拒否案 — neutral `GuiSize` の
  定義域未明文化 + VST3 adapter との verb 挙動分岐を理由に棄却
  （exp jsonl seq 4）。

## Validation

- `mise run fmt-check`: exit 0（`evidence/rf759/fmt-check.txt`）。
- `mise run check ruviform-plugin-clap`: exit 0。
- `mise run lint ruviform-plugin-clap`: exit 0。
- `mise run test ruviform-plugin-clap`: exit 0。
- `mise run lint ruviform-plugin-test-clap`: exit 0 — Issue の
  破損 lane（受入条件）。
- `mise run check ruviform-plugin-test-clap`: exit 0。
- `mise run test ruviform-plugin-test-clap`: exit 0（非回帰 —
  `editor_gui_verbs_decline_without_open_gui` 等の GUI verb 系を含む）。
- `mise run test-l1-real-plugin`: exit 0 — host feature の
  `--lib --bins` 段で `gui_size_component_saturates_at_pixel_bounds`
  を含む 71 lib tests + runner bin tests、`#[ignore]` E2E 段
  （l1_real_adapter / l1_real / fault / stateful）も全 pass。
- `mise exec -- prek run --all-files`: exit 0。
- OS: Windows（dispatch worktree）。toolchain は mise 管理の
  既定（`mise.toml`）。MSRV lane はタスク指定により未実施。

## Risks

- なし（境界値のみの写像契約変更で、通常サイズの観測可能な挙動は
  round 化による ±0.5px 未満の差に留まる。`set_gui_size` の
  `can_resize` gate・`PluginFailed`/`Unsupported` の error 経路は
  無変更）。

## Follow-ups

- なし。docs index（`docs/**/README.md`）は Coordinator が統合時に
  `doc-maintenance` で再生成する（Worker 非編集の取り決め）。
