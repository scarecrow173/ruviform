---
id: "EXP-630abiValidatorRerun08"
type: "impl"
status: "completed"
title: "Issue #630 外部 ABI validator 再実走 slice — 実験記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/impl/ir/issue-630-abi-validator-rerun.md"
    - "docs/impl/exp/issue-630-abi-validator-rerun-exp.jsonl"
  changes:
    added:
      - type: "file"
        path: "docs/impl/exp/issue-630-abi-validator-rerun.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-abi-validator-rerun-exp.jsonl"
  implements:
    - "docs/impl/ir/issue-630-abi-validator-rerun.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-630-abi-validator-rerun.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 外部 ABI validator 再実走 slice — 実験記録

詳細な時系列イベントは companion JSONL
`docs/impl/exp/issue-630-abi-validator-rerun-exp.jsonl`
（`experiment_event.v1` schema）を参照。

## 要約

1. PR#735 merge 後の post-#735 ABI surface（CLAP preset-discovery/
   preset-load・VST3 IUnitInfo 系・`test_spawn_fail`）に対し、
   旧 runner（schema /1）のまま実走プローブを実施。
2. `test_spawn_fail` が enumerable だが create を意図的に拒否する
   契約により、clap-validator が plugin-instance case を
   create_plugin-null signature の failed として大量報告
   （spawn_fail leg で 35 件）— 旧 runner では leg Failed。
3. library-level の `preset-discovery-crawl`/`load` case が
   clap-validator 0.4.1 内部で 45 秒 timeout + JSON 無し終了
   （upstream issue #41 の MetadataReceiver self-deadlock）。
   orphan 化した sandboxed child が DLL をロックする副作用を
   確認し手動終了。
4. VST3 suite validator は `test_spawn_fail` の create 拒否を
   `Failed to create component instance of test_spawn_fail!` /
   `Failed to setup test!` として反復出力するが `Result:` 集計
   （235 passed, 0 failed）には含めない — decline が run.json で
   不可視になることを確認。
5. test_fault の param 除外と同型の宣言的機構として
   `expectCreateDecline`（CLAP）・`CLAP_FIXTURE_LIBRARY_EXCLUDES`
   + `clapLegExclusions`（library 除外合成）・
   `VST3_DECLARED_UNCREATABLE` + `applyVst3DeclaredUncreatable`
   （decline 証拠の記録と 0 件時の Passed→Failed）を設計。
6. contract test を 18→32 test に拡張し全 pass。canonical 再実走で
   verdict Passed / exit 0、run.json schema /2 に全証拠を記録。
7. 非回帰一式（fmt/check/lint/test 両 fixture crate・L1 lane・
   test-rt・prek）を実 exit code つきで記録。`test-l1-real-plugin`
   の instrument 系 e2e test に高負荷時 flaky を観測したが最終
   実行は全緑 — `crates/` zero-diff のため本 slice 起因でない
   既存 lane の flaky として記録。
