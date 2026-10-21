---
id: "ADR-034JNYt58J8NlUkn6RfJMs"
type: "adr"
status: "accepted"
title: "workspace MSRV 引上げ（1.85 → 1.89）と fearless_simd 本番昇格の解禁"
created: "2026-10-21"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/52"
  changes:
    added: []
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "dsp-simd-backend-adoption.md"
    - "rust-dev-foundation-toolchain-and-ci-profile.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../specs/establish-rust-dev-foundation.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/msrv-raise-for-fearless-simd.md"
    - "dsp-simd-backend-adoption.md"
    - "../specs/dsp-simd-poc-adr.md"
  derived-by:
    - "../designs/msrv-raise-design.md"
    - "../plans/implement-issue-52-msrv-raise.md"
    - "../tasks/apply-msrv-1-89-and-sync-docs.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/34"
    - "https://github.com/scarecrow173/ruviform/issues/53"
    - "https://releases.rs/docs/1.89.0/"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# workspace MSRV 引上げ（1.85 → 1.89）と fearless_simd 本番昇格の解禁

## Context and Problem Statement

採否 ADR [dsp-simd-backend-adoption.md](dsp-simd-backend-adoption.md) 決定 2 で `fearless_simd` 1.1.0 が条件付き採用されたが、その `[dev-dependencies]` → `[dependencies]` 本番昇格には workspace MSRV ≥1.89 の引上げが前提となる（同 ADR 決定 6 で「別途 MSRV 引上げ Issue へ送る」と確定）。現行 `rust-version = "1.85"`（Issue #34 で確定）は edition 2024 の安定化版を下限としたものであり、`fearless_simd` 1.89・`multiversion` 1.86・`criterion` 1.86・`pulp` 推移的 ≈1.89 の要求と矛盾する。引上げ幅・適用範囲・検証基準をここで決定する。

## Considered Options

* **Option A — `rust-version = "1.89"` に引き上げる**。`fearless_simd` の要求をちょうど満たす最小値
* **Option B — より新しい値（例: 1.90 以降）に引き上げる**。余裕を持たせる
* **Option C — 1.85 を維持する**。`fearless_simd` 本番昇格を断念する
* **Option D — メンバー個別の `rust-version` にする**。`ruviform-math` のみ 1.89、他は 1.85 のまま

## Decision Outcome

Chosen option: **Option A — `rust-version = "1.89"` に workspace 全体で引き上げる**

* 根拠:
  * `fearless_simd` 1.1.0 の MSRV 要求（1.89）をちょうど満たす最小値であり、利用側環境の幅を最大化する。不要に新しい値を選ぶと、より古い toolchain でビルドする利用者を失う
  * 採否 ADR 決定 6 の候補一覧で、全候補の要求値（fearless_simd 1.89・multiversion 1.86・criterion 1.86・pulp 推移的 ≈1.89）が 1.89 に収束する。1.89 は現在の全候補を包含する最小共通値
  * edition 2024 let-chains（1.88+）を使う PoC bench/test ターゲットを含めた `--all-targets` の MSRV が 1.89 で成立する見込み — `--lib` 範囲のみ 1.85 適合だった既知制限が解消する。これを実測で確認する
  * Option B（1.90+）は現時点で根拠となる要求がなく、利用側の互換幅を狭めるだけ。将来新依存が新 MSRV を要求した時点で再評価する方が合理的
  * Option C（1.85 維持）は採否 ADR 決定 2 の `fearless_simd` 採用と矛盾する
  * Option D（メンバー個別 MSRV）は workspace の `edition`/`rust-version` 一括継承が正本という既存方針（Issue #34）と矛盾し、依存グラフ上の MSRV が検証しづらくなる

* mise pin（`rust = "1.99.0"`）は開発ツールの pin であり MSRV 宣言値ではない — 変更しない。MSRV は `Cargo.toml` の `rust-version` が正本
* MSRV 検証は `cargo +1.89 check --workspace --all-targets` の成功をもって行う。let-chains を使う bench/test ターゲットを含めて成立することが確認できれば、従来の `--lib` のみの制限も解消する
* `fearless_simd` の `[dependencies]` 昇格そのものは Issue #53 のスコープであり、本 ADR は MSRV 前提の解禁のみを決定する
* CI の `verify-ci` は引き続き mise pin toolchain で検証する。MSRV 専用の CI ジョブは追加しない（rust-version 変更は稀なイベントであり、ローカル `cargo +<msrv>` 検証で十分）

## Implementation Plan

* Affected paths: `Cargo.toml`（`[workspace.package] rust-version`）、`AGENTS.md`（「Rust ワークスペースと暫定開発規約」節の MSRV 言及）、必要なら `README.md` 等の MSRV 言及箇所
* Constraints: workspace 一括 `rust-version` を維持する（メンバー個別 MSRV を作らない）。stable Rust のみ。`rust-version` のコメント（Issue #34 由来・引上げトリガの記述）を新値と根拠に更新する。mise pin は変更しない

## Verification

* [ ] `cargo +1.89 check --workspace --all-targets` が成功する（let-chains を使う bench/test ターゲットを含めて）
* [ ] `Cargo.toml` の `rust-version` が `1.89` になっている
* [ ] `AGENTS.md` 内の MSRV 言及が `1.89` に更新されている
* [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` が全パス（mise pin 1.99 での現行検証を維持）
* [ ] `mise exec -- prek run --all-files` がパスする
