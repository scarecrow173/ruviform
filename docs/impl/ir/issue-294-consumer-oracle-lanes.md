---
id: "IMPL-fwt9SAzTdFbcF3sWq52Zlp"
type: "impl"
status: "completed"
title: "Issue #294 consumer oracle lanes — consumer-verify 両 verify job への rf628 oracle lane 配線"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-294-consumer-oracle-lanes.md"
      - type: "doc"
        path: "docs/specs/issue-294-consumer-oracle-lanes.md"
      - type: "doc"
        path: "docs/designs/issue-294-consumer-oracle-lanes.md"
      - type: "doc"
        path: "docs/plans/issue-294-consumer-oracle-lanes.md"
      - type: "doc"
        path: "docs/tasks/issue-294-consumer-oracle-lanes.md"
      - type: "doc"
        path: "docs/test-specs/issue-294-consumer-oracle-lanes.md"
    modified:
      - type: "file"
        path: ".github/workflows/consumer-verify.yml"
      - type: "file"
        path: "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/issue-294-consumer-oracle-lanes.md"
  implemented-by: []
  depends-on:
    - "../../designs/issue-294-consumer-oracle-lanes.md"
    - "../../plans/issue-294-consumer-oracle-lanes.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../specs/issue-294-consumer-oracle-lanes.md"
    - "../../discovery/issue-294-consumer-oracle-lanes.md"
    - "../../test-specs/issue-294-consumer-oracle-lanes.md"
    - "../../designs/issue-627-virtual-first-verification-contract-design.md"
    - "issue-294-virtual-first-evidence-schema.md"
    - "issue-294-gui-contract-lane-tree-boundary-guard-fake-driver-semantic.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/issue-294-consumer-oracle-lanes.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/issue-294-consumer-oracle-lanes.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "fixture_version は consumer Cargo.lock の ruviform-audio-test version + @<RUVIFORM_REV> 形式（ruviform-audio-test-<version>@<40hex>）で記録 — 抽出失敗時は flag 省略（null）で捏造しない"
      - "rec() case spec を field 6=oracle・field 7=fixture_version へ additive 拡張（oracle+fixture は $RUVIFORM_REV 同伴のペア必須 — CASE_SPEC_RE で pin）"
      - "SV の 2 oracle file は別 step・別 lane 行で実行（file 別 verdict attribution）"
    rejected:
      - "SV の `mise run test synvelis-recording --test rf628_oracle --test rf628_oracle_rt_alloc` 単一併記実行 — coverage 同等だが file 別 verdict を埋没させるため却下"
      - "fixture_version の version+SHA opaque hash 化 — 可読性・監査性で却下"
      - "oracle 行のみ `node \"$EV\" lane` 直接呼出し — spec/flag 対応が分散するため却下（rec() 拡張で一箇所に集約）"
---

# Issue #294 consumer oracle lanes — consumer-verify 両 verify job への rf628 oracle lane 配線

## Summary

`.github/workflows/consumer-verify.yml` の両 verify job（`verify-audiolens`/`verify-synvelis`）に consumer oracle lane を追加した。consumer checkout 内の canonical test file を `oracle_probe` step が file 単位で存在確認し、存在すれば `git lfs pull` の後に canonical command（`mise run test <pkg> --test <file>`）を file 粒度の別 step で実行する。Record step は probe output の 3 分岐（`'1'`→run outcome / `'0'`→owner 付き `not_run` / else→probe outcome）で lane 行を記録し、`rec()` の case spec を field 6=`oracle`・field 7=`fixture_version` へ additive 拡張して `--oracle`/`--fixture-version`/`--resolved-ruviform-sha`（single-SHA guard 済み `$RUVIFORM_REV`）を記録する。`fixture_version` は consumer `Cargo.lock` の `ruviform-audio-test` version から `ruviform-audio-test-<version>@<RUVIFORM_REV>` を構成し、抽出失敗時は flag を省略する（値を捏造しない）。`evidence.mjs`・`mise.toml`・`ci/daily`・`ci/tracker`・他 workflow・consumer repo は変更していない。

## Context

- 実施タスク: `docs/tasks/issue-294-consumer-oracle-lanes.md`（`docs/plans/issue-294-consumer-oracle-lanes.md` 配下の単一 stream）
- 上流: `docs/specs/`・`docs/designs/`・`docs/test-specs/` の `issue-294-consumer-oracle-lanes.md`、正本契約 `docs/designs/issue-627-virtual-first-verification-contract-design.md` §2 Case schema・§4 独立 Oracle 規則
- 前提（実測確認済み）: schema v2 の optional field（`oracle`/`fixtureVersion`/`resolvedRuviformSha`）は `ci/consumer-verify/evidence.mjs:80-93` で受理済み — script 変更不要。consumer oracle test は両 repo に merge 済み（AL `crates/al-analysis/tests/rf628_oracle.rs`・SV `crates/synvelis-recording/tests/rf628_oracle.rs` + `rf628_oracle_rt_alloc.rs`）。single-SHA lockfile guard が全 ruviform source を候補 SHA へ pin 済み。
- 先例: gui_candidate の RF #390 probe→run→record 3 分岐（候補 task 不在→owner 付き not_run・dispatch ref 代用禁止）。

## Implementation

### Changes

- `.github/workflows/consumer-verify.yml` —
  - ヘッダ comment に consumer oracle lane 節を追加（canonical path・owner 付き not_run・oracle/fixture_version/resolved_ruviform_sha 記録・deterministic-backend 限定の明記）。
  - `verify-audiolens`: `test` lane 後・UI lane 群前に `oracle_probe`（`crates/al-analysis/tests/rf628_oracle.rs` → `oracle` output）と `oracle` run step（`if: gate=run && tuple_ok && fetch=success && oracle_probe.outputs.oracle=='1'`、`git lfs pull` + `mise run test al-analysis --test rf628_oracle`）を追加。
  - `verify-synvelis`: `headless_deps` 後・UI lane 群前に `oracle_probe`（`rf628_oracle.rs`→`oracle`・`rf628_oracle_rt_alloc.rs`→`oracle_rt`）と `oracle`/`oracle_rt` の 2 run step（`presence=='present'` 併記の同型 gate、`mise run test synvelis-recording --test <file>`）を追加。
  - 両 jobの `rec()`: spec を `"<case_id>|<level>|<environment_kind>|<requirement>[|<resolved_sha>[|<oracle>[|<fixture_version>]]]"` へ additive 拡張（`IFS='|' read` の field 6/7、非空時のみ `--oracle`/`--fixture-version` flag — 既存 5 field 呼出しは不変）。
  - 両 jobの Record step: `test-full`/`headless-deps` 行の後に `FIXTURE_VER` 抽出（`grep -A3 '^name = "ruviform-audio-test"$' "$CONSUMER_DIR/Cargo.lock"` → version → `ruviform-audio-test-<ver>@${RUVIFORM_REV}`、失敗時は空）と oracle lane 行を追加。AL 1 file・SV 2 file、各 3 分岐（probe='1'→`o(steps.oracle[_rt].outcome)` / '0'→owner 付き `not_run`（AL `scarecrow173/audio-lens#127`・SV `scarecrow173/synvelis#115` — 再実行条件・代用禁止を reason に明記）/ else→`o(steps.oracle_probe.outcome)`+未完了理由）。分類は既定 `OS-L1|deterministic-backend|Required`。
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs` — `CASE_SPEC_RE` を oracle+fixture ペア tail（`$RUVIFORM_REV` 同伴必須）へ拡張。`rshaRows` を job 別件数（AL 9=gui6+oracle3 / SV 12=gui6+oracle6）+ slug 許容集合（`gui-(boundary|semantic)`・`rf628-oracle(-rt-alloc)?`）へ更新。`$FIXTURE_VER` 行数（AL 3/SV 6）と `|rf628_oracle(_rt_alloc)?|$FIXTURE_VER` 末尾を pin。新規 test「consumer oracle lanes probe canonical files and record oracle fields」で step id・record 写像・canonical path・canonical command・run if 条件（fetch+probe のみ・check/test 非依存）・owner 付き not_run・spec tail・Cargo.lock 由来 fixture_version・`rec()` flag 写像を pin。
- `docs/development-environment.md` — consumer-verify.yml 節（:131）の lane 列挙に oracle lane を 1 段落追記（canonical path・probe/run・owner 付き not_run・oracle/resolved_sha/fixture_version 記録・deterministic-backend 限定）。他節は不変。

### Decisions During Implementation

- **file 粒度の別 step 実行**: SV の `rf628_oracle`（PCM/Project/fixture oracle）と `rf628_oracle_rt_alloc`（RT no-alloc 計測）は別 step・別 lane 行。`--test` 併記の単一実行は coverage 同等だが file 別 verdict を正直に attribut できないため却下（`metadata.experiments.rejected`）。
- **`fixture_version` 記録値**: `ruviform-audio-test-<lock version>@<RUVIFORM_REV>`。single-SHA guard が全 ruviform source を候補 SHA へ pin 済みのため SHA 部は宣言値=解決値。「同一 fixture version」AC の tuple 比較可能性を満たす可読形式を採用し、opaque hash 化は却下。抽出は `grep -A3` + `grep -m1` で Cargo.lock の `[[package]]` 形式に依存 — 失敗時は `FIXTURE_VER=""` → flag 省略（null）で outcome と切り離す。
- **`rec()` spec 拡張を採用**: oracle 行のみ `node "$EV" lane` 直接呼出しの代替案は optional flag を各分岐で手書きすることになり spec/flag 対応が分散するため却下。field 6/7 は `oracle`+`fixture_version` のペア必須（CASE_SPEC_RE の入れ子 optional で `$RUVIFORM_REV` 同伴時のみ受理）。
- **`--expected`/`--observed` は記録しない**: consumer test は機械可読の期待値/観測値を出力しない — 捏造禁止で null のまま。
- **probe の `if` は fetch/check/test 非依存**: file 存在判定は consumer checkout（resolution 後）で完結。run step 側で `fetch` 成功（toolchain+依存解決済み）を最小の正直な依存とし、`check`/`test` lane の成否に oracle verdict を連動させない。
- **`git lfs pull` を各 run step で実行**: clone は `GIT_LFS_SKIP_SMUDGE=1` — test-full lane と同じ materialize を行い欠落 asset による虚偽 fail を防ぐ（2 回目以降は no-op）。
- **exp 文書なし**: 採否は design doc + 本 IR の `metadata.experiments` に記録する水準（PoC 計測・代替実装の実測比較を要する性質ではない — 先行 slice `issue-294-virtual-first-evidence-schema` と同判断）。

### Known Limitations（正直記録）

- **実 runner 検証は未実施（NotRun）**: workflow_dispatch lane の実走は post-merge の Coordinator 管轄。本 slice は lane 配線 + 構造契約 pin のみ。
- **per-case (per-test fn) 粒度なし**: consumer test は機械可読 verdict を出力しない — file/domain 粒度 lane 行に留まる。per-case JSONL emitter + ingest 契約は consumer repo 側後続 slice（AL #127 / SV #115）。
- **`fixture_version` は crate version+SHA**: fixture 内部の scenario/seed 版は識別しない — 再現性は resolved SHA pin の範囲に留まる。
- **deterministic-backend のみ**: oracle lane は headless deterministic test であり、実 OS・実機・GUI・native plugin の証拠ではない（実 OS lane は RF #629/#630/#631・#313/#314、GUI fixture は AL #166/#127・SV #175-177 の別 gate）。
- **docs index 未同期**: `docs/**/README.md` は Coordinator が統合時に再生成（本 slice では未編集・未再生成）。

## Related Experiments

### Adopted

`metadata.experiments.adopted` 参照（fixture_version 記録値・rec() spec 拡張・SV file 別実行）。

### Rejected

`metadata.experiments.rejected` 参照（SV 単一併記実行・opaque hash・直接 lane 呼出し）。

## Validation

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| 証拠器 fixture + yml 契約 | `mise run test-ci-consumer-verify` | **84 pass / 0 fail**（node --test ci/consumer-verify/ — evidence.test.mjs + consumer-verify-yml-contract.test.mjs） |
| workflow 構文 | `mise exec -- actionlint .github/workflows/consumer-verify.yml` | exit 0・エラー 0 |
| downstream consumer 非破壊（補助・canonical=mise） | `mise exec -- node --test ci/` | **335 pass / 0 fail**（tracker/daily/daily-ci の `evidence.mjs` export consumer を含む横断退行確認） |
| hygiene | `mise exec -- prek run --all-files` | 全 hook pass（trim/eof/yaml/toml/line-ending/bom/private-key/actionlint） |
| 実 runner dispatch | — | **NotRun**（post-merge・Coordinator 管轄） |

## Risks

- **probe='1' かつ run step skipped（fetch 失敗等）**: record 分岐 1 は `o(steps.oracle.outcome)` を使うため `skipped` と正直記録され pass 化しない（`o()` の写像は既存契約）。
- **`rec()` spec field 増加の後方互換**: `IFS='|' read` の追加変数は field 6/7 省略時に空 → flag 非付与。既存 5 field 呼出し（gui 行含む）は不変 — CASE_SPEC_RE が oracle+fixture の `$RUVIFORM_REV` 同伴ペアのみ受理するため oracle 単独行の混入も構造的に排除。
- **`fixture_version` 抽出の脆さ**: `grep -A3` は Cargo.lock の `[[package]]` 形式（`name` 直後に `version`）に依存。形式変更で空になるが flag 省略（null）に留まり捏造しない — test file が無ければ lane 自体が not_run/fail で記録されるため outcome を汚染しない。
- **`git lfs pull` の繰り返し**: oracle run step ごとに実行するが 2 回目以降は no-op（既に materialize 済み）。

## Follow-ups

- per-case (per-test fn) JSONL emitter + ingest 契約（consumer repo 側 — AL #127 / SV #115 後続 slice）。
- 実 OS lane（RF #629/#630/#631・#313/#314）・consumer-side GUI contract fixture（AL #166/#127・SV #175-177）・`contract` v1→v2 bump・post-merge dispatch 実走（Coordinator 所有）。
- `gh issue view 294` / open PR 重複確認は探索時点で `gh` 不可のため未実施 — PR 作成前に再確認する。

## Links

- Issue: <https://github.com/scarecrow173/ruviform/issues/294>（OPEN 継続 — `Refs #294` のみ）
- 契約正本: `docs/designs/issue-627-virtual-first-verification-contract-design.md`（`scarecrow173/ruviform#627` Closed）
- 関連: `docs/impl/ir/issue-294-virtual-first-evidence-schema.md`（schema v2 の先行 slice）、`docs/impl/ir/issue-294-gui-contract-lane-tree-boundary-guard-fake-driver-semantic.md`（probe→run→record 先例）
