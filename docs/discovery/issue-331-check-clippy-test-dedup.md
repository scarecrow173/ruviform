---
id: "DISC-034RIZZ08rxXFAvXxiesKc"
type: "discovery"
status: "resolved"
title: "Issue #331 check/clippy/test 重複評価の調査・実測"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/331"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/specs/issue-331-check-clippy-test-dedup.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/exp/issue-327-ci-baseline.md"
    - "docs/impl/exp/issue-329-sccache-compile-cache.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/ruviform/issues/329"
  defers: []
  deferred-by: []
---

# Issue #331 check/clippy/test 重複評価の調査・実測

## Summary

`verify` lane の `check-ci`(`cargo check --workspace --all-targets --profile ci`)と `lint-ci`(`cargo clippy --workspace --all-targets --profile ci -- -D warnings`)は、workspace member 140 コンパイルユニットの Rust frontend 処理を完全に二重実行している。依存(152 ユニット)は両コマンドが同一 rustc・同一 fingerprint で共有するため重複しない。`cargo clippy` は clippy-driver(rustc + clippy lint 登録)で workspace member をコンパイルするため、check の診断は全て包含される — check-ci step を verify lane から除去しても検出能力は等価で、ユニット数ベースで frontend パス 1 回分(140/280 ユニット)を削減できる(confirmed・ローカル JSONL 実測)。

## Exploration Goal

Issue #331(CI-OPT-04)は `check-ci`/`lint-ci`/`test-ci` の重複する Rust frontend 処理と profile 間再ビルドの有無を実測し、検出能力の等価性を条件に `check` 削減・コマンド順序変更・profile 統一を選択適用する。本 discovery は「どの重複が存在し、どれが除去可能で、どれが除去できないか」を実測で確定する。

## Key Issues and Assumptions

- `cargo check` と `cargo clippy` の成果物共有関係 — 検証済み: deps は共有されるが ws member は異なるコンパイラバイナリ(rustc vs clippy-driver)のため別 fingerprint で二重コンパイルされる(confirmed)。
- `cargo test` は check-mode と異なる emit(codegen)と cfg(test)を要求するため check/clippy とは成果物を共有できない(confirmed — 除去不可の重複でなく別責務)。
- `cargo clippy` が `cargo check` の検出能力を包含するか — clippy-driver は rustc の全診断を発行しつつ clippy lint を追加する。本リポジトリに `cfg(clippy)` 条件付きコードは存在しない(grep 0 件)。注入実験で実証(test-spec C4-C8・exp 記録)。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: verify lane から check-ci step を除去(clippy が型検査を包含) | 140 ws ユニットの frontend 二重実行を除去 + step/起動 overhead 削減。検出能力等価(注入実験で実証) | clippy-driver が ICE する稀なケースで check 由来の「素の rustc 受理」信号が失われる(ただし同 job 内で失敗は可視) | **yes** |
| B: 順序変更のみ(lint → check → test) | なし — check を残す限り ws member は依然 2 回 frontend 評価される。順序を変えてもユニット数は不変 | 効果なし | no |
| C: profile 統一(dev/ci の統合) | target ディレクトリ一本化 | ローカル開発の incremental ビルドと CI の `CARGO_INCREMENTAL=0` を同じ target に混在させて双方の再利用を破壊する。`--profile ci` 維持要件にも反する | no |
| D: test-ci 側も check 相当を兼ねる | `cargo test` は codegen + cfg(test)で別 fingerprint — 重複は構造的に除去不可 | なし | n/a(分析のみ) |
| E: 変更せず記録のみ | リスクゼロ | 実測で有意な削減余地が確認されたため見送り要件(Issue 実装 3「改善しない場合」)に該当しない | no |

## Tentative Conclusions and Hypotheses

- **[confirmed]** `verify` の `check-ci` step は `lint-ci` が包含する(同一 target 集合・同一 feature・同一 profile。ws member を rustc か clippy-driver のどちらがコンパイルするかだけの差で、clippy-driver は rustc 診断の superset)。除去して検出能力を失わない。
- **[confirmed]** deps(152 ユニット)は check/clippy 間で既に共有済み(a-clippy で 152 fresh)。check を落としても deps のメタデータコンパイルは lint-ci 内で 1 度だけ行われる — コストは移動するだけで増えない。
- **[confirmed]** `test-ci` の依存 codegen(114 build-mode ユニット)と ws cfg(test) ユニット(135)は構造上 check/clippy と共有不能。39 ユニット(build script・proc-macro)のみモード横断で共有される。
- **[confirmed]** profile 間再ビルドは存在しない — check-ci/lint-ci/test-ci は全て `--profile ci` + `CARGO_INCREMENTAL=0` で `target/ci/` を共有する(dev profile の `target/debug/` とは分離。混在する再ビルドはない)。

## Open Questions

- なし(実測で方向確定。残りは CI 上の差分実測 = PR 実行時に exp で記録)。

## Promotion Candidates

- [x] spec-doc needed: docs/specs/issue-331-check-clippy-test-dedup.md
- [ ] adr-doc needed: 不要 — 既存アーキテクチャ内の CI 効率化であり、選択肢間に見合う構造的トレードオフを持つ ADR 級決定ではない(design doc の Alternatives 表に残す)。

## Sources and Relations

- Issue: https://github.com/scarecrow173/ruviform/issues/331
- 計測契約正本: docs/designs/issue-327-cross-repo-ci-baseline.md
- sccache 実測: docs/impl/exp/issue-329-sccache-compile-cache.md

## Evidence

### 2026-11-04 ローカル計測(Windows 11 x64 / Git Bash / rust 1.99.0 / `CARGO_INCREMENTAL=0`)

隔離 target dir 2 系統(`target/opt331/A`・`B`)で `--message-format=json-render-diagnostics` の compiler-artifact 行を集計。

| invocation | wall | artifacts | fresh | ws 再コンパイル | dep 再コンパイル |
| --- | --- | --- | --- | --- | --- |
| A: `cargo check --workspace --all-targets --profile ci`(cold) | 31s | 292 | 0 | 140 (34 pkgs) | 152 (135 pkgs) |
| A: `cargo clippy --workspace --all-targets --profile ci`(check 後) | 9s | 292 | **152** | 140 (34 pkgs) | 0 |
| A: `cargo test --workspace --profile ci` | 77s | 288 | 39 | 135 (34 pkgs) | 114 (114 pkgs) |
| B: `cargo clippy --workspace --all-targets --profile ci`(cold) | 32s | 292 | 0 | 140 | 152 |
| B: `cargo check --workspace --all-targets --profile ci`(clippy 後) | 8s | 292 | **152** | 140 | 0 |
| B: `cargo test --workspace --profile ci` | 101s | 288 | 39 | 135 | 114 |

b-test は並行実行した単体テスト再実行と CPU 競合したため a-test より大きい(ユニット数は同一 288。wall は参考値)。

- **除去可能な重複**: ws member 140 ユニット × rustc check-mode(clippy 側は clippy-driver で同 140 ユニットを別途コンパイル)。b-check(clippy 後の check 残余)= 140 ユニット・8s が純粋な除去対象。
- **共有済み**: dep メタデータ 152 ユニット(check↔clippy で同一 fingerprint・fresh 152 で実証)。
- **共有不能**: dep codegen 114 ユニット(build mode)・ws cfg(test) 135 ユニット。test の fresh 39 は build script/proc-macro のモード横断共有。
- non-fresh kind 内訳(check/clippy 同一): lib 64+custom-build 18+proc-macro 10+staticlib 1+cdylib 4+test 49+example 6+bin 12+bench 4(check 側は dep 含め lib 188)。test 側: lib 177+staticlib 1+cdylib 4+test 49+example 6+bin 12(bench なし — `cargo test` は bench target をビルドしない)。

### CI step 実測(verify job・#329/#340 run から gh api 集計)

| run | OS | check-ci | lint-ci | test-ci |
| --- | --- | --- | --- | --- |
| 41816075282(cold) | ubuntu | 49s | 10s | 100s |
| 41816075282(cold) | windows | 77s | 14s | 128s |
| 41816075282(cold) | macos | 89s | 19s | 163s |
| 41816923217(warm) | ubuntu | 31s | 10s | 87s |
| 41816923217(warm) | windows | 51s | 12s | 84s |
| 41816923217(warm) | macos | 76s | 20s | 140s |
| 41820870299 | ubuntu | 47s | 10s | 97s |
| 41820870299 | windows | 92s | 18s | 158s |
| 41820870299 | macos | 78s | 15s | 125s |

check-ci step ≈ deps メタデータ + ws-check 140 ユニット。lint-ci step(10-20s)≈ ws-clippy 140 ユニットのみ(deps fresh)。除去後の lint-ci は deps メタデータを吸収して旧 check-ci 相当の時間になるため、正味削減 = ws-check パス(140 ユニット)+ step/mise 起動 overhead(数秒)。warm CI では ws-check ユニットの大半が sccache hit になるため削減は hit レイテンシ相当が主(約 10-20s 程度/job 見込み)、cold では実コンパイル分がそのまま削減。
