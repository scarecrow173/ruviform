---
id: "TSPEC-gdsPYN7GKtS63uz3sB59lW"
type: "test-spec"
status: "approved"
title: "consumer-verify oracle lane — probe/run/record 3 分岐・oracle field 記録の構造契約"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-294-consumer-oracle-lanes.md"
    modified:
      - type: "file"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-294-consumer-oracle-lanes.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-294-virtual-first-evidence-schema.md"
    - "docs/test-specs/issue-294-gui-contract-lane-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-consumer-oracle-lanes.md"
    - "docs/designs/issue-294-consumer-oracle-lanes.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-294-consumer-oracle-lanes.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# consumer-verify oracle lane の構造契約（Feature / Rule / Example）

## Feature: canonical file probe

- **Rule**: `oracle_probe` step は consumer checkout 内の canonical test file 存在を file 単位で判定し `oracle`（SV は `oracle_rt` も）を `$GITHUB_OUTPUT` へ出す。**Example**: AL `crates/al-analysis/tests/rf628_oracle.rs`、SV `crates/synvelis-recording/tests/rf628_oracle.rs` + `rf628_oracle_rt_alloc.rs`。
- **Rule**: probe の `if` は `gate == 'run' && tuple_ok == 'true'`（SV は `presence == 'present'` 併記）のみ — `fetch`/`check`/`test` outcome に依存しない。

## Feature: file 粒度の正直な実行

- **Rule**: run step は `git lfs pull` の後に canonical command を実行し、`fetch` 成功 + probe output `'1'` のみを `if` に持つ（`check`/`test` 成否に非依存）。**Example**: `mise run test al-analysis --test rf628_oracle`、`mise run test synvelis-recording --test rf628_oracle`、`mise run test synvelis-recording --test rf628_oracle_rt_alloc`。
- **Rule**: SV の 2 file は別 step・別 lane 行 — `--test` 併記の単一実行は file 別 verdict を埋没させるため禁止。
- **Rule**: 新規 step id（`oracle_probe`/`oracle`/`oracle_rt`）は record 内に `steps.<id>.outcome` の参照を持つ（既存の全 id→lane 行ルールに適合）。

## Feature: 3 分岐 record と oracle field

- **Rule**: probe='1' → `o(steps.oracle[_rt].outcome)`、probe='0' → owner 付き `not_run`（AL `#127`/SV `#115`・再実行条件・代用禁止を reason に明記）、else → `o(steps.oracle_probe.outcome)` + 未完了理由。
- **Rule**: oracle 行の spec は `"${CONSUMER_NAME}-rf628-oracle[-rt-alloc]|$DEF|$RUVIFORM_REV|<oracle id>|$FIXTURE_VER"` — `resolved_ruviform_sha`/`oracle`/`fixture_version` を全分岐で記録。`oracle`+`fixture_version` は spec field 6/7 のペア必須。
- **Rule**: `fixture_version` は `$CONSUMER_DIR/Cargo.lock` の `ruviform-audio-test` version から `ruviform-audio-test-<version>@<RUVIFORM_REV>` を構成し、抽出失敗時は flag 省略（null・捏造禁止）。
- **Rule**: `--expected`/`--observed` は記録しない（consumer test が機械可読値を出さない）。
- **Rule**: 分類は `OS-L1|deterministic-backend|Required`（既定 `$DEF`）。`unsupported` 固定行・既存 `not_run` 行・`contract: consumer-verify-tuple/v1`・schemaVersion 2・outcome 語彙は不変。

## Examples

- probe が `oracle=1`・run step success → lane 行 outcome `pass`・`oracle:"rf628_oracle"`・`resolved_ruviform_sha:<40hex>`・`fixture_version:"ruviform-audio-test-0.1.0@<40hex>"`。
- consumer ref が oracle test 未同梱（probe `oracle=0`）→ `not_run` + reason に `#127`/`#115` Owner と代用禁止。
- fetch 失敗で run step skipped（probe='1'）→ `skipped` 行（pass 化しない）。
- `rec()` spec `"${CONSUMER_NAME}-rf628-oracle|$DEF|$RUVIFORM_REV|rf628_oracle|$FIXTURE_VER"` が CASE_SPEC_RE を通過し、field 6/7 が `--oracle`/`--fixture-version` へ写像される。

## Guarantees

- `mise run test-ci-consumer-verify` が CASE_SPEC_RE grammar・rshaRows 件数（AL 9/SV 12）・canonical path/command・owner not_run・oracle field 記録を pin し、offline で検証する。
- `actionlint` が workflow 構文を検証する。実走は post-merge dispatch（Coordinator 所有・NotRun）。

## Non-goals

- per-case (per-test fn) verdict、実 OS lane、consumer-side GUI fixture、contract v2、evidence.mjs 変更、consumer repo 変更。
