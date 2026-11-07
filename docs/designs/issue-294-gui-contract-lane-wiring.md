---
id: "DESIGN-RVQaQUltza40Kb1HlAt48X"
type: "design"
status: "approved"
title: "Issue #294 GUI-CONTRACT lane 接続設計 — 候補 tree 取得・task probe・step/lane 分離"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "docs/specs/issue-294-gui-contract-lane-wiring.md"
  changes:
    added: []
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
    - "docs/specs/issue-294-gui-contract-lane-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-294-gui-contract-lane-wiring.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-390-driver-contract-lab.md"
    - "docs/test-specs/issue-294-gui-contract-lane-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-294-gui-contract-lane-wiring.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-294-gui-contract-lane-wiring.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/390"
  defers: []
  deferred-by: []
---

# Issue #294 GUI-CONTRACT lane 接続設計

## 概要

両 verify job（`verify-audiolens` / `verify-synvelis`）の Consumer lane 群と
`extra tasks` step の間に、**候補 tree 評価の 3 step**（checkout+probe、
boundary guard、fake driver contract）を追加し、Record step の固定
`not_run` 行を step outcome 由来の実 lane 行へ置き換える。

## 候補 tree 取得（step `gui_candidate`）

```text
cand = $RUNNER_TEMP/ruviform-candidate    # 常に専用 scratch dir
git init / remote add https://github.com/scarecrow173/ruviform.git
git fetch --depth 1 origin $RUVIFORM_REV / checkout --detach FETCH_HEAD
rev-parse HEAD == $RUVIFORM_REV を確認（不一致は step fail）
```

- **常に専用 dir へ clone する**（dispatch ref == candidate の場合でも
  workspace を再利用しない）。`working-directory:` は step outputs を参照
  できないため、条件分岐で candidate dir が変わると後続 lane step の
  working-directory を静的に書けない。一律 clone にすることで
  `working-directory: ${{ runner.temp }}/ruviform-candidate` を固定でき、
  全経路で `rev-parse` 照合も統一される（depth-1 fetch の追加コストのみ）。
- Consumer clone と同じ統一 fetch 経路で、GitHub の unadvertised SHA fetch
  受理に依存する（既存 comment と同根拠）。
- deploy_key mode では `dep` step が `url.insteadOf` を global 設定済みのため、
  HTTPS URL が SSH へ rewrite される。anonymous mode では public repo の匿名
  fetch。**step は `dep` より後に置く**（cleanup 前であることも自明）。
- `RUVIFORM_REV` は preflight で 40-hex 検証済み（run script への直接
  interpoltion なし — env 経由は既存契約どおり）。

## task probe（同 step 内）

```text
boundary = grep -qE '^\[tasks\.check-ui-driver-boundary\]' mise.toml
contract = grep -qE '^\[tasks\.test-ui-driver-contract\]' mise.toml
```

- task が 1 つでもあれば `mise install --locked rust`（lane が使う tool のみ —
  node / git-lfs は両 task とも不要）。
- `boundary` / `contract` を step output として出力するが、**install 成功後に
  限る**。toolchain 解決の失敗を output 空 → record の else 分岐
  （`gui_candidate.outcome` = fail）へ落とし、実行 lane が `skipped` のまま
  pass-with-coverage 化する経路を塞ぐ。

## lane step

- `gui_boundary`: `if` = 既存 lane 条件 + `boundary == '1'`、
  `working-directory: ${{ runner.temp }}/ruviform-candidate` で
  `mise run check-ui-driver-boundary`。
- `gui_semantic`: 同条件 + `contract == '1'` で
  `mise run test-ui-driver-contract`。`RUVIFORM_DRIVER_LAB_EVIDENCE` を
  `$RUNNER_TEMP` 配下に設定し、実行後に JSONL を step summary へ追記する
  （候補 tree 評価の実行証拠）。
- `if` 条件は Consumer lane と同じ `gate == 'run' && tuple_ok == 'true'`
  （synvelis は `presence == 'present'` も）。tuple 未確立の run で候補側だけ
  pass を記録しない。

## Record step の lane 行

`boundary`/`contract` output で 3 分岐:

1. `"1"` → `o('<実行 step>.outcome')`（実行結果を正直記録）。
2. `"0"` → `not_run` + 理由「候補 SHA に task 未定義・RF #390 Owner・
   task を含む候補で再実行（dispatch ref 代用なし）」。
3. 上記以外（checkout/probe/toolchain 解決の skipped/failure）→
   `o('gui_candidate.outcome')` + 理由「candidate checkout/probe/toolchain
   未完了」。

Consumer 側通常 View/Domain headless は別 `not_run` 行として保持
（Owner: AL #166 / SV #175）。`unsupported` 固定行は無変更。

## 判定分離の保持

- guard 失敗 → GUI-CONTRACT 行のみ `fail`。semantic 失敗 → GUI-SEMANTIC 行のみ
  `fail`。どちらも status 導出の既存 fail-closed 規則に従う
  （`deriveStatus` 変更不要 — `evidence.mjs` は無変更）。
- 実 Native/OS/plugin は `unsupported` 固定行のまま — Fake pass を
  Native pass に読み替えない（Issue #390 manifest の layer 分離と一致）。

## contract test（`consumer-verify-yml-contract.test.mjs`）更新

- 旧「GUI-CONTRACT/GUI semantic は not_run 固定」assertion を、
  「候補 tree 評価へ接続され、task 不在時のみ owner 付き not_run」に更新。
- `gui_candidate`/`gui_boundary`/`gui_semantic` の各 id が rec 行で
  outcome 参照されること（既存の全 id→lane 行ルールに自動適合）。
- 候補 checkout が `$RUVIFORM_REV` を fetch し `rev-parse` 照合すること、
  lane step が候補 dir で実行されることを pin する。
- `evidence.mjs checkout is the dispatch ref` 契約は維持
  （actions/checkout の `ref:` に候補 SHA を渡さない不変条件は無変更）。

## 限界・スコープ外

- `ci/daily` scheduler・`ci.yml`・Consumer repo 側 mise.toml・`evidence.mjs`
  本体・`crates/` 配下の fixture/test 本体は変更しない。
- macOS / 実 OS / 実 plugin / GPUI driver semantic（#386）・Consumer 側
  View/Domain fixture（AL #166 / SV #175）は本変更の対象外で、
  既存 not_run/unsupported/owner 記録を維持する。
