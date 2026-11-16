---
id: "SPEC-MIToHOPFHSbRZ0H5kHB121"
type: "spec"
status: "approved"
title: "Issue #294 consumer oracle lanes — consumer-verify 両 verify job への rf628 oracle lane 配線"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/discovery/issue-294-consumer-oracle-lanes.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-294-consumer-oracle-lanes.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-294-consumer-oracle-lanes.md"
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
    - "docs/specs/issue-294-gui-contract-lane-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/impl/ir/issue-294-virtual-first-evidence-schema.md"
    - "POLICIES.md"
  refines:
    - "docs/specs/issue-294-virtual-first-evidence-schema.md"
  refined-by: []
  derives-from:
    - "docs/discovery/issue-294-consumer-oracle-lanes.md"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "ci/consumer-verify/evidence.mjs"
    - ".github/workflows/consumer-verify.yml"
    - "ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs"
  defers: []
  deferred-by: []
---

# Issue #294 consumer oracle lanes — consumer-verify 両 verify job への rf628 oracle lane 配線

## What Should Be Built

`.github/workflows/consumer-verify.yml` の両 verify job（`verify-audiolens`/`verify-synvelis`）に、consumer repo 側で merge 済みの独立 Oracle test（`rf628_oracle` 系）を実行・記録する oracle lane を追加する slice。schema v2（`ci/consumer-verify/evidence.mjs`）が既に受理する optional field（`oracle`/`fixture_version`/`resolved_ruviform_sha`）へ実値を流し、file 粒度の lane 行として evidence JSON / Job Summary に記録する。gui_candidate の RF #390 先例（probe→run→record 3 分岐）を踏襲する。

1. **probe step（両 job）**: consumer checkout（`$CONSUMER_DIR`）内の canonical test file 存在を file 単位で判定し `$GITHUB_OUTPUT` へ出す。AL は `crates/al-analysis/tests/rf628_oracle.rs` → `oracle`、SV は `crates/synvelis-recording/tests/rf628_oracle.rs` → `oracle`・`crates/synvelis-recording/tests/rf628_oracle_rt_alloc.rs` → `oracle_rt`。crate 配下の予期しない移動は canonical 実行不可として `0`。
2. **run step（両 job・file 粒度）**: `git lfs pull`（clone は LFS smudge skip — test-full lane と同じ materialize）の後、canonical command を実行。AL: `mise run test al-analysis --test rf628_oracle`。SV: `mise run test synvelis-recording --test rf628_oracle` と `mise run test synvelis-recording --test rf628_oracle_rt_alloc` を別 step で実行（file 別 verdict attribution のため単一併記にしない）。`check`/`test` lane の成否には依存せず、`fetch` 成功（toolchain+依存解決済み）を最小の正直な依存とする。
3. **record 行（file 粒度・3 分岐）**: probe output `'1'` → run step outcome の `o()` 写像、`'0'` → owner 付き `not_run`（AL `scarecrow173/audio-lens#127` / SV `scarecrow173/synvelis#115` — oracle test を含む consumer ref で再実行・dispatch ref/別 crate 代用の偽 pass 禁止）、それ以外 → probe outcome をそのまま写す。全 oracle 行は `resolved_ruviform_sha=$RUVIFORM_REV`（single-SHA guard 済み解決値）・`oracle=<test target 名>`・`fixture_version=<consumer Cargo.lock の ruviform-audio-test version>@<RUVIFORM_REV>` を記録する。`--expected`/`--observed` は consumer test が機械可読値を出さないため記録しない（捏造禁止）。
4. **contract test 更新**: `consumer-verify-yml-contract.test.mjs` の `CASE_SPEC_RE` を拡張 spec tail へ更新し、`rshaRows` 件数/slug 絞込み・新規 pin（canonical file path・canonical command・owner 付き not_run・oracle/fixture/resolved sha 記録）を追加する。
5. **doc chain + 記録**: slug `issue-294-consumer-oracle-lanes` の discovery/spec/design/plan/task/test-spec/impl ir 文書を作成し、`docs/development-environment.md` の consumer-verify lane 列挙へ oracle lane を追記、`evidence/rf294/` に検証記録を残す。

## Why Now

- POLICIES.md（Effective 2026-11-15 仮想優先受入）と #627 design §2/§4 は、AudioLens の f64/長尺/CLI oracle と Synvelis の PCM/RT/Project oracle を**独立 Oracle**として候補 SHA tuple に結合することを要求する。schema v2 は記録器を着地済みだが、consumer oracle test を実際に実行して `oracle`/`fixture_version`/`resolved_ruviform_sha` を記録する lane はまだ無い。
- 両 consumer の oracle test は merge 済み（AL `scarecrow173/audio-lens#127`・SV `scarecrow173/synvelis#115`）で canonical 実行経路が確立しており、ruviform 側の lane wiring のみが残っている（探索 JSON verdict DISPATCHABLE）。
- Issue #294 の受入は file/domain 粒度の独立判定を要求し、per-case emitter・実 OS lane は別 gate — 本 slice はそのうち consumer oracle lane wiring に限定する。

## Users and Value

- **Issue #294 / #627 の受入 Owner・Coordinator**: 候補 tuple `(ruviform SHA, consumer SHA)` に対し、consumer 独立 Oracle の実行結果が `oracle` 識別子・`resolved_ruviform_sha`・`fixture_version` 付きで file 粒度に記録され、Pass/Fail/NotRun を独立判定できる。
- **`ci/daily` tuple scheduler**: `contract` v1・status 語彙不変のため既存の証拠回復経路をそのまま使い続けられる。
- **後続 slice（per-case emitter・実 OS lane）**: file 粒度 lane 行が確立されることで、consumer 側 JSONL emitter の ingest 契約の土台になる。

## Scope

### In Scope

- `.github/workflows/consumer-verify.yml`: 両 verify job への `oracle_probe`/run step 追加と Record step への file 粒度 lane 行（3 分岐）追加、`rec()` spec の additive 拡張（oracle/fixture_version field）、workflow ヘッダ comment の lane 説明追記。
- `ci/consumer-verify/tests/consumer-verify-yml-contract.test.mjs`: `CASE_SPEC_RE` grammar 更新、`rshaRows` 件数/slug 更新、新規 oracle lane pin。
- `docs/development-environment.md`: consumer-verify.yml 節（:131）の lane 列挙への oracle lane 追記のみ。
- doc chain（slug `issue-294-consumer-oracle-lanes`）と `evidence/rf294/` の検証記録。

### Out of Scope

- `ci/consumer-verify/evidence.mjs` 本体（schema v2 optional field は受理済み — 変更は原則不要、必要時は範囲拡張を申請）。
- `mise.toml`（新規 task 追加なし — consumer 側既存 `mise run test <pkg>` 経路を使用）。
- `ci/daily/`・`ci/tracker/`・`ci/summary/`・他 workflow・`POLICIES.md`・`AGENTS.md`・`Cargo.toml`/`Cargo.lock`・`crates/`。
- consumer repo 側の全ファイル（`audio-lens/`・`synvelis/` は読み取りのみ）。
- per-case (per-test fn) JSONL emitter + ingest 契約（consumer repo gate — AL #127 / SV #115 後続 slice）。
- 実 OS lane（RF #629/#630/#631・#313/#314）、consumer-side GUI contract fixture（AL #166/#127・SV #175-177）、`contract` v1→v2 bump、post-merge dispatch 実走（Coordinator 所有）。
- docs index（`docs/**/README.md`）の編集・再生成 — Coordinator が統合時に再生成。

## Requirements

- **canonical 実行経路**: oracle test は consumer checkout 内の canonical path に存在する file のみを対象とし、`mise run test <pkg> --test <file>` で実行する（AL `al-analysis`/`rf628_oracle`、SV `synvelis-recording`/`rf628_oracle`・`rf628_oracle_rt_alloc`）。予期しない crate 配下への移動・task 未定義は canonical 実行不可として正直に扱う。
- **file 粒度 verdict**: SV の 2 test file は別 step・別 lane 行とし、一方の失敗を他方へ埋没させない。`--test` 併記の単一実行は採用しない。
- **正直な 3 分岐**: probe='1' → run step outcome の写像、probe='0' → owner 付き `not_run`（再実行条件・代用禁止を reason に明記）、probe/step 未完了 → その outcome（skipped/fail 含む）をそのまま写す。偽 pass・pass 水増しを構造的に排除する。
- **Case schema v2 記録**: oracle 行は `resolved_ruviform_sha=$RUVIFORM_REV`（guard 済み解決値）・`oracle=<test target 名>`・`fixture_version=ruviform-audio-test-<lock version>@<RUVIFORM_REV>` を記録する。`expected`/`observed` は記録しない。`fixture_version` 抽出失敗時は flag を省略し値を捏造しない。
- **既存契約の不変**: `workflow_dispatch` 専用 trigger・concurrency tuple key・`cancel-in-progress: false`・expected SHA fail-closed 照合・extra_tasks 検証・全終端経路の証拠 artifact・not-run-is-not-pass 集約・`contract: consumer-verify-tuple/v1`・schemaVersion 2・outcome 語彙は変更しない。新規 Pass の付与なし。
- **検証入口**: `mise run test-ci-consumer-verify`（更新した contract test 込み）・`mise exec -- actionlint .github/workflows/consumer-verify.yml`・`mise exec -- prek run --all-files` が green。補助証拠として `mise exec -- node --test ci/`（`evidence.mjs` export を import する他 ci/ consumer の横断退行確認）。

## Acceptance Criteria

- [ ] 両 verify job が consumer oracle lane を持つ: probe（canonical file 存在確認）→ run（canonical `mise run test`）→ file 粒度 lane 行（AL 1 行・SV 2 行、各 3 分岐）に `--oracle`・`--resolved-ruviform-sha $RUVIFORM_REV`・`--fixture-version`（`ruviform-audio-test-<version>@<RUVIFORM_REV>`）を記録する。
- [ ] target 不在時は owner 付き `not_run` 固定行（AL `scarecrow173/audio-lens#127` / SV `scarecrow173/synvelis#115`）。probe・上流 step 未完了はその outcome（skipped/fail 含む）をそのまま写し、偽 pass しない。
- [ ] `mise run test-ci-consumer-verify`（更新した contract test 込み）・`mise exec -- actionlint .github/workflows/consumer-verify.yml`・`mise exec -- prek run --all-files` が green。補助: `mise exec -- node --test ci/`。
- [ ] workflow 実走（workflow_dispatch）は本 slice のローカル検証では NotRun — post-merge dispatch は Coordinator 所有として記録する。
- [ ] `docs/development-environment.md` の consumer-verify lane 列挙へ oracle lane を追記（他節は不変）。
- [ ] doc chain（slug `issue-294-consumer-oracle-lanes`）と `evidence/rf294/` 検証記録が整合。docs index は未同期のまま（Coordinator が統合時に再生成）。

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| per-case (per-test fn) JSONL emitter + ingest 契約 | consumer test が機械可読 verdict を出力しない — consumer repo 側の後続 slice | `scarecrow173/audio-lens#127` / `scarecrow173/synvelis#115` 後続 | file 粒度以上の case 別判定が得られない |
| 実 OS lane（Windows L2/L3・macOS/Linux L1・OS 仮想 Endpoint・実 ABI Test Plugin・GUI Runner） | RF #629/#630/#631・#313/#314 の実装・環境 gate | Issue #294 本体・`scarecrow173/ruviform#629`-`#631` | oracle lane は deterministic-backend L1 のみ — 実 OS 受入は別途 |
| consumer-side GUI contract fixture | AL #166/#127・SV #175-177 が Owner | 各 consumer Issue | GUI-CONTRACT consumer-side は not_run のまま |
| `contract` v1→v2 bump | `ci/daily`・`daily-ci.yml`・docs 連動の後続 slice | Issue #294 後続 slice | schemaVersion 2 と contract v1 の版ずれが残る（語彙上分離済みで実害なし） |
| post-merge dispatch 実走 | workflow_dispatch は merge 後の対象 SHA でのみ意味を持つ — Coordinator 所有 | Coordinator | lane の実走証拠は PR 本文で NotRun と明記 |

## Sources and Relations

- Issue #294（OPEN 継続 — 本 slice は oracle lane wiring、`Refs #294` のみ）
- `docs/discovery/issue-294-consumer-oracle-lanes.md`（現状監査・実測根拠）
- `docs/specs/issue-294-virtual-first-evidence-schema.md`（schema v2 の先行 spec — `refines`）
- `docs/specs/issue-294-gui-contract-lane-wiring.md`（probe→run→record 先例 spec）
- `docs/designs/issue-627-virtual-first-verification-contract-design.md` §2 Case schema・§4 独立 Oracle 規則（正本）
- `docs/impl/ir/issue-294-virtual-first-evidence-schema.md`（rec() spec 方式・Decisions 先例）
- `POLICIES.md`（2026-11-15 仮想優先受入 — OS-L1/GUI-L2/EXT-L3・Required/Deferred/NotApplicable）
