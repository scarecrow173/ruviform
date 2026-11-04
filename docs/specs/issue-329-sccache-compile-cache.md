---
id: "SPEC-FuYJCOjgHlGKlqRskoLQGq"
type: "spec"
status: "approved"
title: "Issue #329 CI-OPT-02 sccache 先行導入・3OS Rustコンパイルキャッシュ効果検証"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/329"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/adr/README.md"
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
    - "docs/plans/issue-329-sccache-compile-cache.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-329-sccache-gha-backend.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-329-sccache-compile-cache.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/ruviform/issues/328"
    - "https://github.com/scarecrow173/ruviform/issues/330"
    - "https://github.com/scarecrow173/audio-lens/issues/148"
    - "https://github.com/scarecrow173/synvelis/issues/147"
  defers: []
  deferred-by: []
---

# Issue #329 CI-OPT-02 sccache 先行導入・3OS Rustコンパイルキャッシュ効果検証

## Problem

`ci.yml` は Cargo registry/git index の依存キャッシュ(`actions/cache`・`~/.cargo`)を導入済みだが、コンパイル成果物(object/rlib/dep-info)はジョブ間で再利用されず、全 12 required checks が毎 run 全量コンパイルしている。`verify`・`ui-verify` の 3 OS matrix で Rust コンパイルが wall time の支配項であり、IR 中間表現の再生成は入力不変でも冪等に繰り返される。sccache(`RUSTC_WRAPPER`)によるコンテンツアドレス化されたコンパイルキャッシュを先行導入し、Cold/Warm の実測で効果と安全性を検証する。

## Diagnosis(変更前に診断する)

- `ci.yml` 6 job のうち Rust コンパイルを持つのは `verify`(3 OS)・`rt-verify`・`icc-verify`・`ui-verify`(3 OS)・`bench`・`l1-real-verify`(3 OS)の全 12 job。依存キャッシュは `~/.cargo/registry` 系のみで、`target/ci/`(ci profile 成果物)は一切永続化されていない。
- Baseline(#327・`docs/impl/exp/issue-327-ci-baseline.md`)では ruviform 代表 run(41780346388)が wall 318s・runner total 1738s。Rust コンパイルが verify 系 lane の支配的コストであり、再利用で効く余地が大きい。
- `verify` は同一 runner 上で `check-ci`→`lint-ci`→`test-ci` を連続実行する。同 profile(`ci`)・同 feature のため cargo のローカル fingerprint 再利用が効く一方、**ジョブ間・OS 間・PR 間**の再利用はゼロ。
- sccache は rustc 呼び出しを wrapper し、compiler 実行ファイルのハッシュ + 完全な引数 + プリプロセス済みソースのハッシュをキーとする content-addressed キャッシュ。OS/toolchain/RUSTFLAGS/target/feature/profile の違いはすべてキーに含まれるため、構成違いの成果物が誤共有されないことが仕組み上保証される。
- GHA backend(`SCCACHE_GHA_ENABLED`)は cache service API を直接叩く。`ACTIONS_RESULTS_URL`/`ACTIONS_RUNTIME_TOKEN` は runner が **JS action にのみ**注入し `run` step には渡さない(公式仕様)ため、export 用の JS action ステップが必須。
- `consumer-verify.yml` は本 Issue 対象外(dispatch-only・Ruviform 側変更は ci.yml のみ)。

## Requirements

1. 維持されている sccache 導入 action(`mozilla-actions/sccache-action`)を評価し、採用時は公式リリースバージョン指定 + 不変 commit SHA で pin する。sccache 本体バージョンも明示 pin する。
2. `ci.yml` の `verify` 3 OS と `ui-verify` 3 OS を先行対象とする。`rt-verify`・`icc-verify`・`l1-real-verify`・`bench` は効果と分離が確認できてから段階展開とし、本 PR では変更しない。
3. `RUSTC_WRAPPER=sccache` + `SCCACHE_GHA_ENABLED` + 統計出力(`sccache --show-stats`)+ 環境確認ステップを各対象 job に入れる。credential env の export は JS action 経由で行い、token 不在時は GHA backend を無効化した fallback(ローカル disk backend)に倒す。
4. コンパイル互換性を保つ: OS/arch/toolchain/profile/feature/RUSTFLAGS/target triple が異なる成果物を共有しないことを sccache のキー構造で保証し、MSRV・異なる feature 構成との不正共有がないことを記録する。
5. 既存 `actions/cache`(Cargo registry/git)との役割分離を維持する(依存取得物 = actions/cache、コンパイル成果物 = sccache)。GitHub cache backend の権限・容量・失効・fork PR での読み書きを監査して記録する。
6. Cold/Warm・Cargo.lock 更新・toolchain 更新・feature 切替・cache miss/破損・キャッシュ無効時ビルドを測定・評価する。CI 上では 2 連続 push で cold→warm を実測し、補助条件(依存 SHA 更新・feature 切替・破損・無効)はローカル実測または repo variable による無効化 run で補う。
7. Before/After は `ci/metrics/collect-ci-metrics.mjs`(#327)の契約(run/job/step/cache record・report)に整合させ、対象 run URL を記録する。
8. 失敗時の無効化手順(repo variable `CI_SCCACHE_DISABLE` による kill switch・`SCCACHE_GHA_VERSION` bump による purge・git revert)を文書化し、Consumer 展開の前提(AL #148 / SV #147)を整理する — ただし Consumer への適用自体は本 PR スコープ外。
9. 12 required checks・job 名・trigger・RT/Golden/f64/GUI/OS/feature/MSRV/consumer 互換カバレッジを一切削らない。Pass/Fail/NotRun/Unsupported を分離して報告する。

## Acceptance Criteria

- AC1: Linux/Windows/macOS の `verify`・`ui-verify` が通常検証を成功させ、job log に `sccache --show-stats` の hit/miss 統計が記録される。
- AC2: 同一 PR の 2 連続 run で warm cache の正しい compile cache hit を確認し、対象 run の wall time / runner total / cache 転送関連の実測値を Before/After(#327 契約)で記録する。
- AC3: toolchain/feature/profile/依存 SHA の違いで誤った成果物を使わないことが、sccache キー構成(compiler hash + args + preprocessed source)の静的検証と、feature 切替(verify=default / ui-verify=webview feature)・ローカル Cargo.lock 更新実験で示される。
- AC4: cache miss(cold)・破損(ローカル注入)・無効(`CI_SCCACHE_DISABLE=1`)のいずれでも正規ビルドが成功する。
- AC5: Windows/macOS を含む Cold/Warm 比較と採否基準(hit 率・転送コスト・runner 時間の効果)を exp 記録として残す。
- AC6: RT/GUI/feature-gated lane の検証範囲・job 名・required checks 集合を変更しない(diff が sccache 関連 step/env/コメント追加のみ)。
- AC7: Consumer 展開の前提条件(AudioLens #148・Synvelis #147)と Rollback 手順が文書化される。

## Non-goals

- `rt-verify`・`icc-verify`・`l1-real-verify`・`bench` への sccache 適用 — 効果・分離確認後の段階展開(後続 Issue/PR)。
- AudioLens・Synvelis 各 repo への実適用 — AL #148 / SV #147 が所有。本 PR は前提整理のみ。
- `consumer-verify.yml`・`consumer-impact-check.yml` の変更。
- 既存 `actions/cache`(Cargo registry/git)の変更・置換 — 役割分離を維持。
- runner キャパシティ・課金・merge queue 対応。
- sccache ローカル disk backend の永続化(`.cache/sccache` を actions/cache で保存) — GHA backend が主経路であり、local backend は credential 不在時の fallback に留める(YAGNI)。
