---
id: "IMPL-RF390-REPORT"
type: "impl-record"
status: "completed"
title: "Issue #390 Driver Boundary/Lab 実装記録"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  implements:
    - "docs/specs/issue-390-driver-contract-lab.md"
  derives-from:
    - "docs/designs/issue-390-driver-contract-lab.md"
    - "docs/plans/issue-390-driver-contract-lab.md"
  verified-by:
    - "docs/test-specs/issue-390-driver-boundary-lab.md"
---

# Issue #390 Driver Boundary/Lab 実装記録

## 実施内容

`ruviform-ui-test` に Driver Boundary Guard（`gui-boundary-guard`、
ruleset=`fixtures/driver-lab/boundary.json`）と Driver Lab
registry（cases/stories manifest + 検証 + evidence emission）を実装。
UI-DVR-01〜08 を `requiredCases` で pin し、UI-DVR-01/05/07/08 の
fake layer を実行 fixture 化。残 case/layer は公開契約未着のため
owner+rerun 条件付き `blocked`/`not_run`（stub 代替なし）。

## 検査設計の要点

- dep-graph: workspace member 起点で direct/feature/
  dep.features/transitive/conditional/allowlist 整合/
  adapter-consumer を検査。cargo metadata の resolve graph は
  未 activation optional dep を省略するため、conditional 到達は
  manifest 宣言（`dep.path` 再帰走査）で補完する。registry dep
  の未 activation target は解決不能で skip（design §限界 6）。
- signature: syn + proc-macro2(span-locations) の exposure-aware
  2-pass 走査。crate 外露出 item のみ検査（pub use chain・
  alias `use wry::X as W`・impl header・cfg 保守走査）。
  allowlisted adapter の pub API 漏出も検出対象（private 利用は
  検出しない）。
- evidence: `{manifest,subject,layer,expected,observed,mechanism,
  detail}` JSONL（`RUVIFORM_DRIVER_LAB_EVIDENCE` 指定時のみ。
  `expected`=manifest 宣言、`observed`=実実行結果）。

## 独立 review 記録

- Reviewer A（設計/整合観点）・Reviewer B（機構観点）を独立
  dispatch。確認済み P0/P1:
  - RB-01 P0: dependency 宣言の `features`/`uses_default_features`
    による adapter viaFeatures 起動が未検査 → `check_dep_activation`
    追加（dep 宣言 feature 表 + adapter feature 連鎖 + 非 grant
    consumer の `adapter-consumer` 検査）。
  - RA critical: allowlisted adapter 自身の optional toolkit 辺が
    `forbidden-dep-conditional` で自爆 → 起点 grant exempt +
    adapter outgoing 辺の走査停止。
  - conditional 走査が resolve 辺のみで未 activation optional
    dep を見失う（fixture で検証）→ manifest-level 再帰走査へ
    再設計（検出確認済み）。
  - signature scanner の私的利用誤検出 → exposure-aware 2-pass
    へ再設計（adapter pub API は対象・private mod 内 item は
    re-export chain 経由のみ）。
  - alias 迂回・impl header・dep-activation 制御流・manifest
    binding 語彙（manual+pass の note 必須・requiredCases・
    layer 語彙・story themes）を対応済み。
- 修正後の targeted re-verify: 全 10 boundary fixture テスト +
  5 contract テスト + live workspace verdict pass。

## 検証結果（head `0bc2883`）

| 検証 | 結果 |
|---|---|
| `mise run check-ui-driver-boundary` | pass（verdict: pass、3 check 全 pass） |
| `mise run test-ui-driver-contract` | pass（5 tests） |
| `cargo test -p ruviform-ui-test` | pass（62 tests: boundary 10 + contract 5 + 既存 47） |
| `cargo clippy -p ruviform-ui-test --all-targets -- -D warnings` | pass |
| `mise run fmt-check` | pass |
| workspace 全体 `lint-ci`/`test-ci` | not run（crate 単位のみ） |
| GPUI/実 OS/Native Dock/Visual Golden | not run / blocked（owner Issue 未着） |

evidence: `RUVIFORM_DRIVER_LAB_EVIDENCE=target/driver-lab/evidence.jsonl`
で 4 実行 case（UI-DVR-01/05/07/08 fake）の JSONL 記録を確認。

## 残 gate

- UI-DVR-02/03/04/06: 契約未着（#385/#387/#388）で `blocked`。
- GPUI/実 OS/`headless-ts` layer: #386/#313/#294 接続で `not_run`。
- consumer-verify GUI-CONTRACT lane への canonical task 接続は
  #294 owner 側の行更新で接続可能（`check-ui-driver-boundary`）。
- `required:false` GPUI grant は RF386 merge で活性化。
- evidence 行と manifest binding の機械照合は後続改善。
