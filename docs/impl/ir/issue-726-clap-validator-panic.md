---
id: "IMPL-034Z1KOt5WW4W743J3SiQh"
type: "impl"
status: "completed"
title: "Issue #726 clap validator panic"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "docs/tasks/issue-726-clap-validator-panic.md"
    - "docs/plans/issue-726-clap-validator-panic.md"
    - "docs/designs/issue-726-clap-validator-panic.md"
    - "evidence/rf726/investigation.md"
    - "evidence/rf726/upstream-report-draft.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-726-clap-validator-panic.md"
      - type: "file"
        path: "docs/impl/exp/issue-726-clap-validator-panic-exp.jsonl"
      - type: "file"
        path: "evidence/rf726/investigation.md"
      - type: "file"
        path: "evidence/rf726/upstream-report-draft.md"
    modified:
      - type: "file"
        path: "ci/plugin-abi-validator/run.mjs"
      - type: "file"
        path: "ci/plugin-abi-validator/tests/run.test.mjs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-726-clap-validator-panic.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-726-clap-validator-panic.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: ["docs/impl/exp/issue-726-clap-validator-panic-exp.jsonl"]
    rejected: []
---

# Issue #726 clap validator panic

## Summary

clap-validator 0.4.1 の `param-fuzz-sample-accurate` panic（exit
0xc0000409 → scoped thread panic → validator exit 101・JSON 未出力）
の最小再現を特定し、根因が upstream validator の堅牢性欠陥
（sandboxed child が status 0 で結果未出力の場合に sandbox.rs:98 の
`.expect()` が panic し scoped thread 経由で validator 全体が死亡）
であることを確定した。最小再現は `test_fault` + `param-fuzz-basic`
（`param-fuzz-modulation` も同様）で、Issue タイトルの
`param-fuzz-sample-accurate` 帰属は誤り（同 test は非 0 終了を
`crashed` として正常記録し JSON を出力する）。upstream 報告は
「報告する」と判断しドラフトを `evidence/rf726/upstream-report-draft.md`
に記録（実起票は worker 権限外・Coordinator hand-off）。leg 扱いは
現行維持（canonical exclude + informational Unsupported）で、
無フィルタ run 再開条件を design doc に明文化した。

加えて lane 実走で runner の stdio 捕捉 bug を発見・修正した:
`runClapLeg` の `spawnSync` pipe stdio が、orphan 化した sandboxed
grandchild（preset-discovery deadlock で wedge）の pipe 保持により
validator 完走・JSON 出力済みにもかかわらず 300s `ETIMEDOUT` を返し、
`proc.error` 優先評価で完全な JSON を捨てて Failed を誤記録していた。
`spawnCapture`（file-backed stdio）を追加して `runClapLeg` に適用し、
contract test 2 件を追加した。leg 定義・exclude 条件・verdict 語彙は
無変更。

## Context

`docs/tasks/issue-726-clap-validator-panic.md`（T1-T9）に基づく。
rf#630 の ABI-validator lane と rf#732 の `--clap-production` leg
（merge commit `e4835019`）の上に積む直列作業。fixture
`crates/ruviform-plugin-test-clap` は rf#630 占有のため read-only
参照のみ。upstream `free-audio/clap-validator`（local checkout master
`5bc2ef3`）も read-only 調査のみ。

## Implementation

### Changes

- `ci/plugin-abi-validator/run.mjs`: `spawnCapture(exe, args,
  {timeout})` を追加。stdout/stderr を `mkdtemp` 配下の一時ファイルに
  `openSync` した fd で `spawnSync` の `stdio` に渡し、終了後に
  `readFileSync` で回収して `proc.stdout`/`proc.stderr` に設定する。
  `runClapLeg` の `spawnSync` 呼び出しを `spawnCapture` に差し替え
  （timeout 300s は維持）。`runVst3Leg` は無変更（Steinberg
  validator は sandbox grandchild を spawn しない）。
- `ci/plugin-abi-validator/tests/run.test.mjs`: `spawnCapture` の
  import 追加 + contract test 2 件 — (1) stdio を継承する
  grandchild が残っても child exit で復帰し stdout を回収すること、
  (2) 存在しない executable・hang child が `proc.error` を返し
  fail-closed であること。
- `evidence/rf726/`: investigation.md・upstream-report-draft.md・
  filter-runs/（絞り込み run の raw stdout/stderr）・
  abi-validator/（修正前 runner の実走 — ETIMEDOUT 誤記録の証拠）・
  abi-validator-fixed/（修正後 runner の実走）。
- docs chain: discovery/specs/designs/plans/tasks/test-specs/impl
  を新規作成し、実測結果に合わせて更新。

### Decisions During Implementation

- **panic 帰属の訂正**: `param-fuzz-sample-accurate` 単独では panic
  せず `crashed: exit code: 0xc0000409` の JSON を出力して exit 1。
  panic するのは `param-fuzz-basic`/`param-fuzz-modulation` で、
  stepped Fault 値 `2.0` が fixture の `exit` mode（`exit(0)`）を
  踏み、sandboxed child が status 0・結果未出力で終了する経路。
  `-j1` でも再現（並列依存ではない）。対照 `test_gain` +
  `param-fuzz-basic` は完走。
- **runner 修正の境界**: verdict ロジック（`normalizeClapLeg`/
  `evaluate`）は契約適合だったが、stdio 捕捉層が「validator が
  JSON を出して正常終了したのに ETIMEDOUT で Failed」という
  誤記録を生じさせていた。file-backed stdio への変更は child exit
  で復帰するため orphan の影響を受けない。`proc.error` による
  fail-closed 評価・300s timeout・JSON parse 経路は不変。
- **preset-discovery crashed を exclude しない判断**: rf630/rf732
  では skipped だった `preset-discovery-crawl`/`load` が現在は
  upstream #41 の deadlock で crashed ×2/leg となり fixture leg を
  Failed にする。param-fuzz と異なり fixture の意図的 fatal surface
  ではなく validator 側 bug のため、exclude せず Failed を正直に
  記録する（upstream release での pin 更新が解消条件）。
  `descriptor-consistency` は success を維持。
- **upstream 報告**: free-audio/clap-validator の issue list
  （open+closed）に同件なし。master `5bc2ef3` に修正なし。
  報告文ドラフトを evidence に記録し、実起票は Coordinator
  hand-off（worker 権限外）。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-726-clap-validator-panic-exp.jsonl`:
  `--include param-fuzz-basic` が panic を単独再現すること、
  `param-fuzz-sample-accurate` が clean `crashed` 記録に留まる
  こと（帰属訂正の根拠）を採用。

### Rejected

- `param-fuzz-sample-accurate` が panic を起こすという当初仮説は
  実測で棄却（JSON 出力 + exit 1 で validator 自体は正常終了）。

## Validation

- `node --test ci/plugin-abi-validator/tests/`: 23 tests pass
  （exit 0。baseline 21 + spawnCapture 2 件）。
- opt-in lane 実走（修正後）: `node
  ci/plugin-abi-validator/run.mjs --clap-validator
  "$LOCALAPPDATA/ruviform/plugin-abi-validator/clap-validator.exe"
  --clap-production --no-build --evidence-dir
  evidence/rf726/abi-validator-fixed` — 結果は
  `evidence/rf726/abi-validator-fixed/run.json` 参照。
- opt-in lane 実走（修正前・証拠保存）: 同コマンドで
  `--evidence-dir evidence/rf726/abi-validator` — 全 fixture leg が
  `Failed — spawn error ETIMEDOUT` と誤記録されることを確認
  （修正の必要性の実測根拠）。
- panic 絞り込み: `filter-runs/` の raw 出力で exit 101・JSON 未出力・
  stderr panic text を記録。`-j1` でも再現。
- `mise exec -- prek run --all-files`: 結果は PR 本文に記録。

## Risks

- fixture leg は preset-discovery crashed ×2 を含むため Failed が
  正直な verdict（0.4.1 の upstream #41 deadlock 未収録が原因）。
  upstream fix 入り release への pin 更新まで fixture leg の Passed
  は期待できない。
- orphan sandbox child（preset-discovery deadlock で wedge）は
  validator timeout 後も kill されず残存し DLL を lock し得る。
  lane 実行前に残存 `clap-validator.exe`（sandbox mode）を確認・
  kill する運用注意が必要（本調査で 14+ 個を実測・除去）。
- `runVst3Leg` は pipe stdio のまま — Steinberg validator が
  stdio 継承 grandchild を spawn する場合は同型問題があり得るが、
  現状そのような spawn は確認されていない（follow-up 候補）。
- upstream 報告の実起票は未実施（Coordinator hand-off）。

## Follow-ups

- Coordinator: `evidence/rf726/upstream-report-draft.md` を確認のうえ
  free-audio/clap-validator へ Issue 起票し、URL を docs/PR に追記。
- preset-discovery deadlock（upstream #41）と panic 伝播の upstream
  fix を含む release が公開されたら pin 更新を検討。
  `runVst3Leg` への `spawnCapture` 適用は同型リスクの有無を確認のうえ
  判断。
