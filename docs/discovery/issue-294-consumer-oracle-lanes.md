---
id: "DISC-XxEhtltezK4yxgxXo5SeB7"
type: "discovery"
status: "resolved"
title: "Issue #294 consumer oracle lanes — consumer-verify 両 verify job への oracle lane 配線 現状監査"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/627"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-294-consumer-oracle-lanes.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-294-virtual-first-evidence-schema.md"
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
    - "docs/designs/issue-294-virtual-first-evidence-schema.md"
    - "docs/designs/issue-294-gui-contract-lane-wiring.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-294-consumer-oracle-lanes.md"
  verifies: []
  verified-by: []
  references:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/impl/ir/issue-294-virtual-first-evidence-schema.md"
    - "ci/consumer-verify/evidence.mjs"
    - ".github/workflows/consumer-verify.yml"
    - "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
  defers: []
  deferred-by: []
---

# Issue #294 consumer oracle lanes — 現状監査

## Summary

Issue #294 の oracle slice: `consumer-verify.yml` の両 verify job（`verify-audiolens`/`verify-synvelis`）へ consumer 独立 Oracle lane（`rf628_oracle` 系 test）を probe→run→record の 3 分岐で配線するための現状監査。schema v2（Case schema optional field `oracle`/`fixture_version`/`resolved_ruviform_sha`）は `ci/consumer-verify/evidence.mjs:80-93`・CLI flag（同:419-421）で受理済みで script 変更は不要。両 consumer の oracle test file は merge 済みで canonical 実行経路 `mise run test <pkg> --test <file>`（両 repo `mise.toml` `[tasks.test]` = `cargo test --locked -p` + `raw_args` 透過）が使える。single-SHA lockfile guard が `ruviform-audio-test` を含む全 ruviform source を候補 SHA へ pin 済みのため、`resolved_ruviform_sha`・`fixture_version` は guard 済み解決値で正直記録できる。確度: high（コード・consumer repo 実測に基づく）。

## Exploration Goal

gui_candidate の RF #390 先例（probe→run→record 3 分岐）を踏襲し、(a) consumer checkout 内の canonical test file 存在 probe、(b) `mise run test <pkg> --test <file>` の file 粒度実行、(c) `--oracle`/`--resolved-ruviform-sha`/`--fixture-version` 付き lane 行の記録、を既存契約（`contract: consumer-verify-tuple/v1`・schemaVersion 2・outcome 語彙・`rec()` spec 方式）を壊さずに実現する方法を確定する。

## Key Issues and Assumptions

- **probe 対象は file 存在のみ**: consumer checkout（`$CONSUMER_DIR`）内の canonical path — AL `crates/al-analysis/tests/rf628_oracle.rs`、SV `crates/synvelis-recording/tests/rf628_oracle.rs` + `rf628_oracle_rt_alloc.rs`。実測で 3 file 全て実在を確認（`G:/repositry/audio-products/{audio-lens,synvelis}/crates/*/tests/`）。crate 配下の移動は canonical 実行不可として `0` → owner 付き `not_run` で正直扱い。confirmed。
- **file 粒度 verdict attribution**: SV は `rf628_oracle`（PCM/Project/fixture oracle）と `rf628_oracle_rt_alloc`（RT no-alloc 計測）で守備領域が異なるため file ごとに 1 実行・1 lane 行。`--test` 併記の単一実行は coverage 同等だが file 別 verdict を正直に attribut できない → 却下（design `metadata.experiments.rejected` に記録）。confirmed。
- **`fixture_version` の記録値**: consumer `Cargo.lock` の `ruviform-audio-test` package `version` を抽出し `ruviform-audio-test-<version>@<RUVIFORM_REV>` 形式で記録（実測: 両 consumer lock で `version = "0.1.0"`・source は `scarecrow173/ruviform.git#<sha>` — guard が `?rev=$RUVIFORM_REV#$RUVIFORM_REV` を強制するため宣言値=解決値）。version+SHA の opaque hash 化は可読性・監査性で却下。抽出失敗時は flag 省略（null）で捏造しない。confirmed。
- **`rec()` spec の additive 拡張**: 第 1 引数を `"<case_id>|<level>|<environment_kind>|<requirement>[|<resolved_sha>[|<oracle>[|<fixture_version>]]]"` へ拡張し非空 field を flag へ渡す。既存 5 field 呼出しは不変。代替の oracle 行だけ `node "$EV" lane` 直接呼出しは分岐ロジックを record step に散らすため却下。confirmed。
- **step id → lane 行の全写像**: 新規 step id（`oracle_probe`/`oracle`/`oracle_rt`）は contract test の「every recorded-capable step id maps to a lane row」が `steps.<id>.outcome` の record 内参照を要求。probe='1' 分岐は run step outcome、else 分岐は `steps.oracle_probe.outcome` を参照する形で充足。confirmed。
- **`--expected`/`--observed` は記録しない**: consumer test は機械可読の期待値/観測値を出力しないため捏造禁止・null のまま（schema v2 は optional で受理）。confirmed。
- **downstream 非破壊**: `ci/daily/decide.mjs` は `kind`/`contract`/run/consumer/tuple/status のみ検査し lanes 未読 — `contract: consumer-verify-tuple/v1` 維持で回復経路は不変。`docs/development-environment.md` は schemaVersion 2 記述で同期済み（:96）— lane 列挙への oracle 追記のみ。confirmed。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. `rec()` spec を `[|<oracle>[|<fixture_version>]]` へ additive 拡張 | case 宣言が 1 箇所に集約・既存呼出し不変・fail-closed | CASE_SPEC_RE grammar 更新が必要 | yes |
| B. oracle 行のみ `node "$EV" lane` 直接呼出し | rec() 不変 | optional flag を各分岐で手書き・spec/flag 対応が分散 | no |
| C. SV を `--test` 併記の単一 step/単一行 | step 数削減 | file 別 verdict を正直に attribut できない（rt_alloc fail が oracle fail に埋没） | no |
| D. `fixture_version` を version+SHA の hash 化 | 固定長 | 可読性・監査性が低い。「同一 fixture version」AC の比較可能性は `name-version@sha` 形式で十分 | no |
| E. probe で `mise tasks info test` や task 存在も検査 | task 不在を事前検出 | `test` task は両 repo の ancient 契約で file 存在が実質 proxy。過剰な防御的チェック | no |

## Tentative Conclusions and Hypotheses

- confirmed: 両 verify job に `oracle_probe`（file 存在 → `oracle`/`oracle_rt` output）→ run step（`git lfs pull` + `mise run test <pkg> --test <file>`、fetch 成功のみ依存・check/test 成否に非依存）→ record 3 分岐（probe=1→run outcome / probe=0→owner 付き not_run / else→probe outcome）の lane を追加する。
- confirmed: oracle 行は file あたり 3 分岐行（AL 1 file=3 行・SV 2 file=6 行）で、全分岐が `resolved_ruviform_sha=$RUVIFORM_REV`・`oracle=<test target>`・`fixture_version=<lock 由来>` を記録する。分類は既定 `OS-L1|deterministic-backend|Required`（headless deterministic oracle lane）。
- hypothesis: consumer ref が oracle test 未同梱（古い SHA）でも Cargo.lock に `ruviform-audio-test` があれば fixture_version は記録され、なければ flag 省略 — いずれも捏造しない。

## Open Questions

- なし（probe path・実行 command・記録値・分岐構造・contract test 更新箇所は design/spec で確定可能。per-case (per-test fn) JSONL emitter は consumer repo gate の後続 slice、実 OS lane は #629/#630/#631・#313/#314、post-merge dispatch 実走は Coordinator 所有として deferred 記録する）。

## Promotion Candidates

- [x] spec-doc needed: `docs/specs/issue-294-consumer-oracle-lanes.md`（oracle lane の要求・AC）
- [ ] adr-doc needed: なし（#627 design §2 Case schema・#390 GUI lane 先例の適用であり新規 ADR を要しない）

## Sources and Relations

- Issue #294（OPEN 継続 — 本 slice は `Refs #294` のみ、Issue レベル AC は分割受入）
- `docs/designs/issue-627-virtual-first-verification-contract-design.md` §2 Case schema・§4 独立 Oracle 規則
- `docs/impl/ir/issue-294-virtual-first-evidence-schema.md`（schema v2 の rec() spec 方式・Decisions 先例）
- `docs/designs/issue-294-gui-contract-lane-wiring.md`（probe→run→record 3 分岐先例）
- `.github/workflows/consumer-verify.yml`（AL :377-438,563-656 / SV :850-899,1039-1136 — 実測）
- `ci/consumer-verify/evidence.mjs`（OPTIONAL_LANE_FIELDS :80-93、CLI flag :419-421 — 実測）
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs`（CASE_SPEC_RE :380-381、rshaRows :396-400、id→lane 行 :127-137、GUI pin :183-283 — 実測）
- consumer repo 実測（読み取りのみ）: `audio-lens/crates/al-analysis/tests/rf628_oracle.rs`・`synvelis/crates/synvelis-recording/tests/rf628_oracle.rs`/`rf628_oracle_rt_alloc.rs`・両 `mise.toml` `[tasks.test]`（`cargo test --locked -p` + raw_args）・両 `Cargo.lock` の `ruviform-audio-test`（version 0.1.0・ruviform git source）
- 探索 JSON: `G:/repositry/audio-products/.devin/dispatch-specs/rf294-explore.explore.json`（verdict DISPATCHABLE）

## Evidence

- 2026-11-17 実測: `evidence.mjs` `OPTIONAL_LANE_FIELDS`（L80-93）に `oracle`/`fixtureVersion`/`resolvedRuviformSha` 含む、`lane` CLI が `--oracle`/`--fixture-version`/`--resolved-ruviform-sha` を受理（L419-421）。`SCHEMA_VERSION=2`・`CONTRACT="consumer-verify-tuple/v1"`。
- 2026-11-17 実測: consumer oracle test 3 file が両 repo main に実在。`al-analysis`/`synvelis-recording` の Cargo.toml は `ruviform-audio-test.workspace = true` を dev-dep 宣言、autotests 無効化なし → `cargo test --test <file>` で到達可能。
- 2026-11-17 実測: 両 consumer `mise.toml` `[tasks.test]` は `raw_args = true` + `run = "cargo test --locked -p"` → `mise run test <pkg> --test <file>` が `cargo test --locked -p <pkg> --test <file>` へ透過。
- 2026-11-17 実測: 両 consumer `Cargo.lock` に `name = "ruviform-audio-test"` / `version = "0.1.0"` / `source = "git+https://github.com/scarecrow173/ruviform.git#<sha>"`。workflow の single-SHA guard（AL :388-398 / SV :859-869）は `scarecrow173/ruviform.git` を含む全 source 行が `?rev=$RUVIFORM_REV#$RUVIFORM_REV` であることを強制。
- 2026-11-17 実測: gui_candidate 先例（AL :471-507,624-637 / SV :947-983,1103-1116）は probe output 3 分岐（'1'→step outcome / '0'→owner 付き not_run / else→probe outcome）を Record step に実装済み — oracle lane は同型。
