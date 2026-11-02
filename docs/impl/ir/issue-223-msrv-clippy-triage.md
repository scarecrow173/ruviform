---
id: "IMPL-034QsY0hYnbSJXS8DBOdDa"
type: "impl"
status: "completed"
title: "issue-223-msrv-clippy-triage"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/223"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/issue-223-msrv-clippy-triage.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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
    adopted: []
    rejected: []
---

# issue-223-msrv-clippy-triage

## Summary

MSRV Rust 1.89 上でのみ発火する既存 pedantic clippy 失敗（Issue #223）を分類・整理した。`cargo +1.89.0 clippy --workspace --all-targets` の実測では 5 系統の失敗を確認: `float_cmp`（272 箇所）・`similar_names`（35 箇所）・`unfulfilled_lint_expectations`（11 箇所）・`must_use_candidate`（1 箇所）・`range_plus_one`（2 箇所）。pin toolchain（clippy 1.99.0）では全て不発火であることを確認済み。

分類結果: `unfulfilled_lint_expectations` の 11 箇所は 1.89 で期待が充足されない属性自体の不整合であり、コード修正（`invert` の iterator 書換）または `expect`→`allow` 変換で両 toolchain で整合させた。`must_use_candidate`・`range_plus_one` は誤検出系のため局所 `allow`＋理由コメントで処理。`float_cmp`・`similar_names` は 1.89 検出器の版差ノイズ（pin でも pedantic に残るため `workspace.lints` での allow は gate 弱化になる）と判定し、AGENTS.md に MSRV 検査用 `-A` 除外コマンドとして明文化した。

## Context

Issue #223（追跡 Issue）を実装ソースとし、Owner 方針「lint gate は pin toolchain（rust 1.99.0）のみ、MSRV 1.89 は build/test で検証、両 version で同一 lint 結果を要求しない」に従う。タスク文書: `docs/tasks/issue-223-msrv-clippy-triage.md`。

## Implementation

### Changes

- `crates/ruviform-numeric/src/linalg.rs` — `invert` の `#[expect(clippy::needless_range_loop)]` を削除し、内部の range-index ループを `split_at_mut` + `iter_mut`/`zip` ベースに書換（`m[col]` 正規化・非 pivot 行消去を upper/lower 分割で表現）。演算順序・行順序を維持するため結果は bit-exact。両 toolchain で lint 不発火となり属性自体が不要化。
- `crates/ruviform-math` の `suboptimal_flops` expect 10 箇所 → `#[allow]` 化（`src/simd_poc.rs` ×2・`src/dsp.rs` ×2・`tests/simd_poc/fearless_impl.rs` ×2・`multiversion_impl.rs` ×2・`pulp_impl.rs` ×2）。`reason` に `allow not expect — undetected by MSRV clippy 1.89` を追記。これらは 1.89 では検出されず `expect` が unfulfilled 警告となるが、1.99 では発火して suppression が必要なため、`allow` が両 toolchain で整合する唯一の形。
- `crates/ruviform-plugin-core/src/event.rs` — `iter()` に `#[allow(clippy::must_use_candidate)]` + 理由コメント。1.89 の `#[must_use]` 提案は誤り方向（`Iterator` trait が既に must_use であり、付与すると pin clippy で `double_must_use` に抵触 — 実測で確認）。
- `crates/ruviform-viz/src/display.rs`・`spectrogram.rs` — `#[allow(clippy::range_plus_one)]` + 理由コメント。`b..b + 1` / `k..k + 1` は `Vec<Range<usize>>` の要素型制約上 `..=` が型検査不適合の 1.89 誤検出。
- `AGENTS.md` — 「MSRV clippy 検証（Issue #223）」節を暫定規約に追加: MSRV 検査コマンド `cargo +1.89.0 clippy --workspace --all-targets -- -D warnings -A clippy::float_cmp -A clippy::similar_names` と、`workspace.lints` で allow しない理由を明文化。
- `Cargo.toml` — `[workspace.lints.clippy]` に同方針のコメントを追記（中央 allow による pin gate 弱化を防ぐ意図の記録）。

### Decisions During Implementation

- **`float_cmp`/`similar_names` を `workspace.lints` で allow しない**: 両 lint は clippy 1.99 でも `pedantic` に含まれる（probe で確認: `-W clippy::pedantic` 下で両 toolchain とも発火）。中央 allow は 1.89 には効くが pin gate を将来の違反にも閉じる → Owner 方針「pin gate を弱めない」に反するため、MSRV 側コマンドライン `-A` による除外ポリシーとした（Issue 本文の「lint 除外対象の明文化」に相当）。
- **`suboptimal_flops` は expect→allow**: 検出境界が toolchain 間でずれる（1.89 では一部パターン不検出→unfulfilled、1.99 では検出→expect 必要）。`expect` は充足を要求するため版差を表現不能。`allow` は充足不要求で両版で安全。
- **`invert` は iterator 書換で属性削除**: `#[expect]` を温存すると 1.89 で unfulfilled のまま残る。`split_at_mut` による同時 mutable borrow 回避は borrow checker 適合かつ `needless_range_loop` 不発火の形で、消去順序も維持される。
- **`#[must_use]` を付与しない**: 1.89 の `must_use_candidate` 提案に従うと pin の `double_must_use`（`Iterator` が trait レベルで must_use）に違反する。正解は `must_use_candidate` を理由付き allow で抑えること。

## Related Experiments

### Adopted

なし（exploration ではなく分類実装タスク。実測→分類→修正の直列作業）。

### Rejected

- `workspace.lints.clippy` での `float_cmp`/`similar_names = "allow"` — pin toolchain でも pedantic に残るため lint gate を弱める（却下）。
- `#[must_use]` 付与による `must_use_candidate` 解消 — pin clippy で `double_must_use` に違反（却下、実測で確認）。
- `k..=k` / `b..=b` への `range_plus_one` suggestion 適用 — `Vec<Range<usize>>` 要素型と `RangeInclusive` が型不一致で不適用（却下）。

## Validation

- `cargo +1.89.0 build -p ruviform-math -p ruviform-numeric` — 警告 0 で成功（MSRV build は変更前から clean であり、clippy:: expect は rustc build で unfulfilled を発火しないことを確認）。
- `cargo +1.89.0 clippy -p ruviform-numeric -p ruviform-math -p ruviform-plugin-core -p ruviform-viz --all-targets --message-format=json` — 残存 warning は `float_cmp`（147）・`similar_names`（23）のみ。`unfulfilled_lint_expectations` 0 件。
- `cargo +1.89.0 clippy --workspace --all-targets -- -D warnings -A clippy::float_cmp -A clippy::similar_names` — **exit 0**（ワークスペース全量で文書化除外のみ残る状態）。
- `cargo +1.89.0 clippy --workspace --all-targets --message-format=json` — lint 分類集計: `float_cmp` 272・`similar_names` 35 のみ（他 lint 0）。
- `mise run fmt-check` / `check` / `lint` — pin toolchain（1.99.0）で全グリーン。
- `mise run test` — pin toolchain で全テスト合格（`ruviform-numeric` 105 件含む、0 FAILED）。`invert` 書換の動作等価性を `least_squares`/`condition_number` 経由のテストで担保。

## Risks

- `float_cmp`/`similar_names` は文書化された `-A` 除外であり MSRV 検査の再現性はコマンドラインに依存する。AGENTS.md のコマンドが正本。
- clippy の版差検出は今後の toolchain 更新で変動し得る。MSRV 上で新たな失敗が出た場合は AGENTS.md の方針（除外リストへ機械追加せず個別判定）に従う。
- `suboptimal_flops` の `allow` は 1.99 で suppression が不要になった場合でも警告を出さない（`expect` の充足チェックと異なり古くなっても気付かない）。検出境界の変動を許容するための意図的トレードオフ。

## Follow-ups

- MSRV toolchain が 1.89 から上がる際に、除外 lint（`float_cmp`/`similar_names`）と `allow` 化した `suboptimal_flops` 箇所の再評価を行う。
