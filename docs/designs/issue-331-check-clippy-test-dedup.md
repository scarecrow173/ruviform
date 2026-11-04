---
id: "DESIGN-034RJ00iyqN3oWCf99pDXu"
type: "design"
status: "approved"
title: "Issue #331 設計 — check-ci を lint-ci へ統合する等価性証明と適用"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/331"
    - "docs/specs/issue-331-check-clippy-test-dedup.md"
    - "docs/discovery/issue-331-check-clippy-test-dedup.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-331-check-clippy-test-dedup.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-331-check-clippy-test-dedup.md"
    - "docs/discovery/issue-331-check-clippy-test-dedup.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-331-check-clippy-test-dedup.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-331-check-clippy-test-dedup.md"
    - "docs/discovery/issue-331-check-clippy-test-dedup.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://doc.rust-lang.org/cargo/commands/cargo-check.html"
    - "https://doc.rust-lang.org/clippy/usage.html"
  defers: []
  deferred-by: []
---

# Issue #331 設計 — check-ci を lint-ci へ統合する等価性証明と適用

## Overview

`ci.yml` `verify` job の `check-ci` step を除去し、`lint-ci`(`cargo clippy --workspace --all-targets --profile ci -- -D warnings`)を型検査・警告・lint の単一 frontend パスとする。`test-ci` は build-mode codegen + cfg(test) の別責務として不変。`mise.toml` では `verify-ci` の列を CI に揃え、`check-ci` タスクは CI lane 非所属のローカル型チェック入口として残す。

## 検査責務の対応表(現状 → 変更後)

| 検査 | 現状の所有者 | 変更後の所有者 | 備考 |
| --- | --- | --- | --- |
| ws member 型エラー | `check-ci`(rustc)+ `lint-ci`(clippy-driver)の二重検出 | `lint-ci` のみ | clippy-driver は rustc 全診断を発行 |
| ws member rustc 警告 | `check-ci` + `lint-ci`(`-D warnings`) | `lint-ci` のみ | clippy-driver 経由でも rustc lint 発火 |
| clippy lint | `lint-ci` のみ | `lint-ci` のみ | check は clippy lint を見ない |
| dep の型エラー | `check-ci`/`lint-ci` の dep メタデータコンパイルで検出 | 同左(dep は常に rustc・check-mode でコンパイル。失敗すれば clippy も失敗) | |
| テスト実行・失敗 | `test-ci` のみ | `test-ci` のみ | clippy/check はテストを実行しない |
| doc test | `test-ci` のみ | `test-ci` のみ | check/clippy は doc test を対象外 |
| build.rs / proc-macro コンパイル | 両コマンドで実行 | 同左(build/proc-macro ユニットはモード横断共有) | |

## 等価性の構造的根拠

1. **同一ユニット集合**: `cargo check --workspace --all-targets --profile ci` と `cargo clippy --workspace --all-targets --profile ci` は同一の feature 解決・同一のコンパイルユニット集合(実測 292 ユニット、kind 別内訳一致)を生成する。target 選択(`--all-targets`)・feature(既定)・profile(`ci`)・workspace 範囲が一致するため。
2. **診断 superset**: clippy-driver は rustc を内包するラッパで、rustc の全エラー・警告診断に加えて clippy 系 lint を登録する。ws member を rustc がコンパイルするか clippy-driver がコンパイルするかの差は「clippy lint の追加評価」のみであり、rustc 診断の検出能力は保存される。
3. **dep 側の検査は同一**: `RUSTC_WORKSPACE_WRAPPER=clippy-driver` により ws member のみが clippy-driver でコンパイルされ、dep は両コマンドとも素の rustc の check-mode でコンパイルされる。dep のコンパイル失敗はどちらのコマンドでも job を落とす。
4. **`cfg(clippy)` 差分なし**: clippy-driver が設定する `cfg(clippy)` 条件付きコードは本リポジトリに存在しない(`grep -rn "cfg(clippy)" crates/` 0 件)。条件コンパイル差による診断差は起こり得ない。
5. **残余リスクの評価**: clippy-driver が rustc の受理するコードで ICE する稀なケースでは、失敗ステップが `check-ci` から `lint-ci` に移るだけで job の失敗検出能力は同一(失敗はログで可視・診断種別は変わるが検出は失われない)。発生率は極小で、発生時も toolchain pin 更新または一時的な step 復帰で対処可能。

## ci.yml 差分設計

```yaml
      - name: Check formatting (mise run fmt-check)
        run: mise run fmt-check

      # Issue #331 (CI-OPT-04): `cargo clippy` は ws member を clippy-driver で
      # check-mode コンパイルし rustc の全診断 + clippy lint を発行するため、
      # 別立ての `cargo check` step は同一ユニット集合の二重 frontend 評価で
      # あった。型検査・警告検出は本 step が包含する。
      - name: Type-check & lint workspace (mise run lint-ci)
        run: mise run lint-ci

      - name: Test workspace (mise run test-ci)
        run: mise run test-ci
```

- `Type-check workspace (mise run check-ci)` step を削除するだけの差分。step 名は `verify` job の required check(`verify (ubuntu-latest)` 等の job 名)に影響しない — required contexts は job 単位。
- sccache step・cargo registry cache・`sccache statistics` step は非変更。sccache の key は rustc/clippy-driver の実行ファイル差を含むため、check-mode 両系統のエントリは既に別 key。check 分のエントリは今後書き込まれなくなるだけ(自然失効)。

## mise.toml 差分設計

```toml
# CI-only tasks: identical checks under `--profile ci` with incremental
# compilation disabled. Artifacts go to target/ci/, separate from dev's
# target/debug/. Not for routine local use (disk usage).

[tasks.check-ci]
description = "CI profile type-check: `cargo check --workspace --all-targets --profile ci` (standalone; not in the CI verify lane — Issue #331 folded it into lint-ci since clippy-driver emits all rustc diagnostics. Kept for local rustc-only type-checks; non-destructive; writes only target/ci/)"
run = "cargo check --workspace --all-targets --profile ci"
env = { CARGO_INCREMENTAL = "0" }

[tasks.lint-ci]
description = "CI profile lint + type-check: `cargo clippy --workspace --all-targets --profile ci -- -D warnings` (clippy-driver emits all rustc diagnostics plus clippy lints — owns type-checking in the CI verify lane since Issue #331; non-destructive; writes only target/ci/)"
run = "cargo clippy --workspace --all-targets --profile ci -- -D warnings"
env = { CARGO_INCREMENTAL = "0" }

[tasks.verify-ci]
description = "Run the CI-equivalent verification sequence: fmt-check -> lint-ci -> test-ci (non-destructive; writes only target/)"
run = [
  "mise run fmt-check",
  "mise run lint-ci",
  "mise run test-ci",
]
env = { CARGO_INCREMENTAL = "0" }
```

## ローカル task 意味づけ(移行手順)

| task | 用途 | CI verify lane 内 |
| --- | --- | --- |
| `check` | dev profile・incremental の快速型チェック(日常の内側ループ) | 外(常時) |
| `check-ci` | ci profile で素の rustc だけに通したい時(clippy-driver 差分の切り分け等) | 外(#331 以降) |
| `lint` | dev profile の lint(= CI lint-ci と同じ lint 集合) | 外(常時) |
| `lint-ci` | ci profile の lint + 型検査 | **内** |
| `verify-ci` | CI 相当列 `fmt-check → lint-ci → test-ci` | 対応列そのもの |

ローカル実行体験への影響: `mise run check`/`lint`/`test`(dev profile)は一切非変更。`mise run verify-ci` を CI 同等列として使う開発者の流れも「実行回数が 1 回減る」だけで検出内容は同一。`check-ci`/`lint-ci`/`test-ci` の個別呼び出しも全て存続。

## Alternatives(検討済み・不採用)

| Option | 不採用理由 |
| --- | --- |
| check → lint → test の順序を lint → check → test に変えるだけ | ws member 二重評価は不変。効果なし |
| lint → test → check(check を最後に) | 同上。check を残す限り 140 ユニットの二重評価は残る |
| test → lint の順に変更 | lint 失敗時に test 結果が出ない現状の安い順序を維持する方が失敗信号が速い(同等検出のまま現行順が最適) |
| dev/ci profile の統一 | dev incremental ビルドと `CARGO_INCREMENTAL=0` の成果物を同 target に混ぜ、双方の再利用を破壊する。`--profile ci` 維持要件違反 |
| `cargo test --all-targets` に変更 | bench/example の test-mode ビルドが増えるだけで負荷増(check-mode でのコンパイル検査は lint-ci が既に提供) |
| check-ci タスク自体の削除 | consumer-verify が自 repo の check-ci を呼ぶ点とは無関係だが、ローカル切り分け用の素 rustc 入口として残す価値がある(YAGNI でも削除でもなく存続) |

## 失敗モードと復旧

| 失敗モード | 影響 | 復旧 |
| --- | --- | --- |
| clippy-driver が rustc 受理コードで ICE | job 失敗(以前は check-ci で落ちていたのと同等) | toolchain bump または `git revert` で check step 復帰 |
| 等価性に穴が見つかる(本設計の想定外診断差) | 検証不足でマージされるリスク | 注入実験(test-spec C4-C8)で事前に潰す。万が一なら revert で即時復帰 |
| 削減効果が期待より小さい | コスト据置 | 実測で記録。効果なしなら無理に変更しない方針を IR に記す |

## Edge cases

| case | 静的評価 |
| --- | --- |
| fork PR / 権限差 | step 削除のみで権限・secret・cache へのアクセスは減る方向のみ。影響なし |
| sccache cold→warm | check 分の key space は今後未使用になるだけ。既存 entry は content-address で誤用しない |
| required checks | job 名・job 数不変 — `verify (<os>)` 3 つ + 他 9 job の required 集合はそのまま |
| MSRV lane | 存在しない(ruviform ci.yml)。consumer 側 1.89 lane は自 repo の task で非依存 |
| doc test / bench | doc test は `cargo test` が担当(不変)。bench target は check-mode では clippy --all-targets がカバー(現状 check/clippy 両方、変更後 clippy のみ — 検査対象は同一) |
| `cargo test` の example ビルド | 変更なし(現状と同じ 6 example unit が test-mode でビルドされる) |
