---
id: "TSPEC-034RJ01zO4VjU2gWOLQXgi"
type: "test-spec"
status: "approved"
title: "Issue #331 テスト仕様 — check/ci 統合の不変条件・注入実験・実測"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/331"
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
    - "docs/plans/issue-331-check-clippy-test-dedup.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-331-check-clippy-test-dedup.md"
    - "docs/specs/issue-331-check-clippy-test-dedup.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-331-check-clippy-test-dedup.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-331-check-clippy-test-dedup.md"
  verified-by: []
  references:
    - "docs/discovery/issue-331-check-clippy-test-dedup.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
  defers: []
  deferred-by: []
---

# Issue #331 テスト仕様 — check/ci 統合の不変条件・注入実験・実測

## Scope

`.github/workflows/ci.yml` `verify` job の check step 除去・`mise.toml` `verify-ci` 列変更・`AGENTS.md` 記述同期を対象とする。job 名・job 数・required checks・他 lane(`rt-verify`/`icc-verify`/`ui-verify`/`l1-real-verify`/`bench`/consumer 系)・Rust ソース・Cargo.toml/Cargo.lock は検証対象外側(非変更であることを検証する側)。

## Invariants

- INV-1(検証非破壊): 12 required checks の job 名・job 数・trigger・permissions・既存 step 名(check step 除く)は不変。diff は check step 除去 + lint step 名/コメント調整 + mise/AGENTS.md 記述のみ。
- INV-2(検査対象同一): `cargo clippy --workspace --all-targets --profile ci` が検査するユニット集合は `cargo check --workspace --all-targets --profile ci` と同一(package・target kind・feature・profile が一致)。変更前後で「検査される target 集合」は減らない。
- INV-3(検出能力同等): rustc エラー/警告・clippy lint・テスト失敗の検出能力は変更前後で同等。check が検出していたものは clippy が必ず検出する(superset)。
- INV-4(feature/OS 範囲不変): `rt-verify`(rt_ filter)・`icc-verify`(icc feature)・`ui-verify`(webview feature + UI 系)・`l1-real-verify`(host feature)・`bench` の範囲は一切不変。通常 workspace 検証で代替しない。
- INV-5(profile/env 不変): `--profile ci`・`CARGO_INCREMENTAL=0`・`--workspace`・`--all-targets`・`-D warnings` は維持。sccache・cargo registry cache の設定は非変更。
- INV-6(ローカル入口存続): `check`/`lint`/`test`(dev profile)と `check-ci`/`lint-ci`/`test-ci` の個別 mise task は全て存続。`verify-ci` は CI 相当列として `fmt-check → lint-ci → test-ci`。

## Cases

| Case | 検証 | 実施者 |
| --- | --- | --- |
| C1: 静的妥当性 | `mise exec -- actionlint .github/workflows/ci.yml` exit 0。`mise exec -- prek run --all-files` green(check-yaml 等)。`mise tasks` で verify-ci 列を確認 | worker |
| C2: 非変更 diff | `git diff` で ci.yml の変更が check step 除去 + lint step 名・コメントのみ、mise.toml/AGENTS.md は記述同期のみ | worker |
| C3: 検査対象同一(ユニット集合) | `--message-format=json-render-diagnostics` で check/clippy の compiler-artifact を集計し、package/target kind 別のユニット集合が一致(292 ユニット・kind 内訳一致)を記録 | worker(実施済み・discovery/exp 参照) |
| C4: 注入 — lib 型エラー | ruviform-numeric `src/lib.rs` に `let _x: i32 = "…"` を注入 → `mise exec -- cargo clippy --workspace --all-targets --profile ci -- -D warnings` が E0308 で rc≠0 | worker(実施済み) |
| C5: 注入 — rustc 警告(lib) | `let unused_binding_xyz = 42;` を注入 → 同コマンドが `unused variable` で rc≠0(-D warnings で昇格) | worker(実施済み) |
| C6: 注入 — clippy lint | `byte as u64` を注入 → 同コマンドが `clippy::cast_lossless` で rc≠0 | worker(実施済み) |
| C7: 注入 — cfg(test)/example/bench | cfg(test) mod 内型エラー・example/bench 内 unused var を注入 → 同コマンドが各診断で rc≠0(--all-targets のカバレッジ確認) | worker(実施済み) |
| C8: 注入 — 失敗テスト + check 対称確認 | `panic!` テスト注入 → `cargo test --workspace --profile ci` が FAILED で rc≠0。同じ型エラーを `cargo check` でも検出する対称確認(i8) | worker(実施済み) |
| C9: CI 3OS green + required | PR run で `verify` 3OS・rt-verify・icc-verify・ui-verify・l1-real-verify・bench 全緑。`gh pr checks` で required 集合観測 | worker(gh) |
| C10: 時間差分 | `collect-ci-metrics.mjs` で対象 run を収集 → `docs/impl/exp/data/issue-331-verify-step-runs.jsonl`。lint-ci/test-ci step 時間と job_exec_s を直前 main run と比較して exp に記録 | worker(gh) |
| C11: Rollback | `git revert` で step 復帰のみ。文書化のみ(実 revert は不要) | 記録 |

## Evidence

- worker: actionlint/prek 出力・`git diff` 要約・注入実験ログ(rc・診断パターン)・ユニット集計表・PR run URL・metrics jsonl を IR/exp に記録。
- `collect-ci-metrics.mjs` で対象 run の run/job/step レコードを収集し `docs/impl/exp/data/issue-331-verify-step-runs.jsonl` に保存。
- NotRun とした検証は理由と後続実施先を IR に明記する(Pass に擬装しない)。

## 注入実験の結果(2026-11-04・実施済み)

| id | 注入 | 実行 | rc | 検出診断 |
| --- | --- | --- | --- | --- |
| i1 | lib 型エラー(E0308) | clippy | 101 | `error[E0308]` / mismatched types |
| i2 | lib unused var(rustc 警告) | clippy | 101 | unused variable(-D warnings で error) |
| i3 | lib `byte as u64`(clippy) | clippy | 101 | `clippy::cast_lossless` |
| i4 | cfg(test) mod 型エラー | clippy | 101 | `error[E0308]` |
| i5 | example unused var | clippy | 101 | unused variable |
| i6 | bench unused var | clippy | 101 | unused variable |
| i7 | `panic!` 失敗テスト | test | 101 | FAILED / panicked |
| i8 | lib 型エラー(対称確認) | check | 101 | `error[E0308]` |

全注入は `git checkout` で完全 revert 済み・working tree クリーン。clippy は check と同等以上の診断を全ケースで rc≠0 として検出した。
