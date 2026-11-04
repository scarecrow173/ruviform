---
id: "SPEC-034RIZZEiovcu0sAxuAPkX"
type: "spec"
status: "approved"
title: "Issue #331 CI-OPT-04 check/clippy/test 重複評価・再コンパイル削減"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/331"
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
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements: []
  implemented-by:
    - "docs/plans/issue-331-check-clippy-test-dedup.md"
  depends-on:
    - "docs/discovery/issue-331-check-clippy-test-dedup.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-331-check-clippy-test-dedup.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
    - "docs/impl/exp/issue-329-sccache-compile-cache.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-331-check-clippy-test-dedup.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-331-check-clippy-test-dedup.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/ruviform/issues/329"
  defers: []
  deferred-by: []
---

# Issue #331 CI-OPT-04 check/clippy/test 重複評価・再コンパイル削減

## Problem

`ci.yml` の `verify` job は同一 runner 上で `check-ci`(`cargo check --workspace --all-targets --profile ci`)→ `lint-ci`(`cargo clippy --workspace --all-targets --profile ci -- -D warnings`)→ `test-ci`(`cargo test --workspace --profile ci`)を連続実行する。実測のとおり、`cargo check` と `cargo clippy` は依存を共有する一方、workspace member 140 コンパイルユニットを rustc / clippy-driver の別バイナリで二重に frontend 評価している。`cargo clippy` は ws member の型検査・rustc 警告も全て発行するため、独立 `check` step は冗長コストであり、検出能力を落とさず削減できる。

## Diagnosis(変更前に診断する)

- workspace = 34 パッケージ・292 check-mode ユニット(ws 140 + dep 152)。`cargo clippy` は dep を素の rustc の check-mode で、ws member を clippy-driver でコンパイルする — dep メタデータは check/clippy 間で同一 fingerprint のため共有済み(ローカル実測 `fresh=152`)。
- ws member は check-mode でも「コンパイラ実行ファイル差」で fingerprint が分かれ、rustc check 140 ユニットと clippy-driver check 140 ユニットがそれぞれ別々に生成される。これが唯一の除去可能な重複。
- `cargo test` は build-mode codegen(dep 114 ユニット)と cfg(test)(ws 135 ユニット)を要求し、check-mode とは構造上共有不能 — 「重複」ではなく別責務のコンパイルであり、対象外。
- `check-ci`/`lint-ci`/`test-ci` は全て `--profile ci`・`CARGO_INCREMENTAL=0`・`target/ci/` 共有で、profile 間再ビルドは存在しない(dev profile の `target/debug/` は意図的に分離)。
- CI step 実測(#329 run・最新 main run): check-ci 31-92s / lint-ci 10-20s / test-ci 84-163s。check-ci step の大半は dep メタデータコンパイルで、除去後は lint-ci がそれを吸収する — 正味削減は ws-check パス(140 ユニット)+ step/起動 overhead。
- CI 詳細データ・ユニット内訳は `docs/discovery/issue-331-check-clippy-test-dedup.md` を参照。

## Requirements

1. `ci.yml` `verify` job の `Type-check workspace (mise run check-ci)` step を除去し、`lint-ci` が型検査を包含する構成にする。lint step 名・コメントは包含する検査内容を正直に表現する。
2. `mise.toml` の `verify-ci` タスクを `fmt-check` → `lint-ci` → `test-ci` に揃えて CI と同一列を維持する。`check-ci` タスク自体は削除せず、CI verify lane には含まれない旨と残存用途(ローカルでの素の rustc による高速型チェック)を description に明記する。
3. `CARGO_INCREMENTAL=0`・`--profile ci`・`--workspace`・`--all-targets`・既定 feature・`-D warnings` は一切変更しない。
4. `rt-verify`/`icc-verify`/`ui-verify`/`l1-real-verify`/`bench`/`consumer-impact-check`/`consumer-verify` の job・feature・OS 範囲は非変更。通常 workspace 検証で代替できると仮定しない。
5. 変更前後の検証対象 package/target/feature/profile/OS の差分表を exp 記録に残す(差分は check step の有無のみで、検査対象集合は同一であることを示す)。
6. 警告(rustc 系・clippy 系)・型エラー・テスト失敗の検出能力が変更前後で同等であることを、意図的なエラー注入実験(lib/test-cfg/example/bench target それぞれへの注入 + 失敗テスト)で実証する。
7. コンパイル時間/critical path/Runner 時間の差分を #327 計測契約(`ci/metrics/collect-ci-metrics.mjs`)に従って実測・記録する。比較で改善しない場合は統合を見送り結果を記録する(正当な成果)。
8. ローカル mise task の意味・移行手順(`check`/`check-ci`/`verify-ci` の使い分け)を AGENTS.md / mise.toml description に明文化する。
9. 12 required checks・job 名・trigger・permissions・RT/Golden/f64/GUI/OS/feature/MSRV/consumer 互換カバレッジを削らない。Pass/Fail/NotRun/Unsupported を分離して報告する。

## Acceptance Criteria

- AC1: `ci.yml` の差分が `verify` job の check step 除去 + lint step 名・コメント調整のみであり、`mise exec -- actionlint` が exit 0。12 required checks の job 名・job 数・trigger は不変。
- AC2: 変更前後の検証対象差分表(package/target/feature/profile/OS)で、check/clippy が検査する target 集合が同一(292 ユニット・kind 別内訳一致)であることをローカル JSONL 実測で示す。
- AC3: 型エラー(lib・cfg(test)・example・bench)、rustc 警告、clippy lint、失敗テストの注入実験で、`lint-ci`/`test-ci` 単独が変更前と同じ診断を rc≠0 で検出することを記録する。
- AC4: 3OS の `verify` が成功し、rt-verify/icc-verify/ui-verify/l1-real-verify/bench の欠落がない(全 required checks green)。
- AC5: 対象 run の wall time / runner total / step 時間を `collect-ci-metrics.mjs` で収集し、Before(直前 main run)との差分を exp に記録する。
- AC6: `verify-ci`/`check-ci`/`lint-ci`/`test-ci` のローカル意味が AGENTS.md・mise.toml description で更新され、ローカル実行体験(快速型チェック入口)を壊さないことが文書化される。
- AC7: Rollback 手順(git revert で step 復帰のみ・永続形式変更なし)が plan/IR に記録される。効率悪化が観測された場合は無理に変更しない方針とともに。

## Non-goals

- `rt-verify`/`icc-verify`/`ui-verify`/`l1-real-verify`/`bench` レーンへの本最適化の展開(feature・実行条件が異なるため個別評価)。
- `consumer-verify.yml`・`consumer-impact-check.yml` の変更(consumer は自 repo の `check-ci` 等を自前で実行 — 本変更と独立)。
- dev profile タスク(`check`/`lint`/`test`/`build`)の変更・dev/ci profile 統合。
- sccache 設定・cargo registry cache の変更(#329/#330 の範囲は維持)。
- `cargo test` の target 選択変更(`--all-targets` 追加等 — bench/example の test-mode ビルド増は最適化でなく負荷増)。
- Cargo.lock・Cargo.toml・ソースコードの変更。
