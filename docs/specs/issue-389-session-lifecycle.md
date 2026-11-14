---
id: "SPEC-034XGr9WuDhnEjKHHcee0S"
type: "spec"
status: "approved"
title: "Issue #389 dock-test session-lifecycle cases（ruviform-plugin-dock-test に session lifecycle/host-terminate/open-or-degrade case 追加・issue-389 設計書 verification-status reconcile）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "docs/designs/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "docs/plans/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "docs/tasks/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "docs/test-specs/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "docs/impl/ir/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "docs/impl/exp/issue-389-session-lifecycle.md"
      - type: "doc"
        path: "evidence/rf389-dock/verification.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-389-plugin-editor-session.md"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/README.md"
  deleted: []
  renamed: []
  moved: []
  generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-631-plugin-dock-scenario.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-session-lifecycle.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #389 dock-test session-lifecycle cases（ruviform-plugin-dock-test に session lifecycle/host-terminate/open-or-degrade case 追加・issue-389 設計書 verification-status reconcile）

## What Should Be Built

`crates/ruviform-plugin-dock-test` の `rf631.plugin-dock` scenario に、実 `test_gain` VST3 cdylib を `Vst3Library` → `InProcessHost` → `PluginEditorSessions`/`InProcessEditorOps` の production 経路で実走させる session-lifecycle 系 case を追加する。あわせて `docs/designs/issue-389-plugin-editor-session.md` の stale な verification-status 記述（R5 matrix・D8 移行表・`GuiParentToken`/`WIRE_VERSION` 関連記述）を現行コードの事実に reconcile する。

追加する case:

- `session-lifecycle`: `begin_session` 成功 → 同一 instance への 2 回目 `begin_session` が `SessionBusy` で拒否 → fake dock slot lease 経由の `session.open(Docked)` が `Ops(Unsupported)`（test_gain `createView` null）→ `Failed{Unsupported}` + `editor_live=false` → `close` が adapter を呼ばず `Closed`+`concluded` へ収束 → `end_session` → instance binding 解放 → slot/generation bump 付き再 `begin_session` → teardown。
- `host-terminate`: live `Failed{Unsupported}` session 保持中に `HostCommand::Destroy` → `PluginEditorSessions::notify_terminated(HostLost)` → `Terminated{cause:HostLost}` がちょうど 1 回 → 再 notify は no-op → `Closed` → `end_session` → binding 解放。
- `open-or-degrade`: `open_or_degrade(Docked → Floating)` が両 mode 拒否で `Err(Ops(Unsupported))`・`Failed{Unsupported}`・`Degraded`/`Attached` event 0 件（double-refusal honesty）。

## Why Now

Issue #389 の session 状態機械・deadline・degrade 契約は plugin-host の unit/integration test で検証済みだが、実 VST3 バイナリを通した session registry の lifecycle 観測（begin/busy/open-refusal/close-converge/end/re-begin、host 破棄時の `notify_terminated` exactly-once、degrade の正直な非発行）は未記録。#631 で作った dock-test harness の fail-closed・正直 verdict 規律を維持したまま、この実バイナリ経路の証拠を追加する。また設計書には VST3/CLAP L0 attach merge・`GuiParentToken` 完全削除・`WIRE_VERSION=5` 反映前の stale 記述が残っており、実装事実との乖離を是正する必要がある。

## Users and Value

- Coordinator/Issue #389 tracker: 実バイナリ経路の session lifecycle 証拠と、設計書 verification-status の正確な reconcile により、後続 slice（実 editor attach・L2/L3 GUI）が依拠する baseline を確定できる。
- 将来の consumer（AudioLens/Synvelis 等）: session 契約の実 ABI 挙動が fake/ScriptedOps だけでなく実 plugin binary で観測済みであることが分かる。

## Scope

### In Scope

- `crates/ruviform-plugin-dock-test/` の scenario case 追加・test 断言更新・README verdict 表更新
- `docs/designs/issue-389-plugin-editor-session.md` の verification-status reconcile（R5 matrix・D8・D5/R2/R4/R6・`WIRE_VERSION`）
- 本 slice の doc chain（spec/design/plan/task/test-spec/IR/exp）と `evidence/rf389-dock/` 証拠
- 触れた docs dir の index 再生成（doc-maintenance ツール）

### Out of Scope

- `test_editor` 実 editor fixture・実 Dock PASS・実画素検証（`dock-pixels` は `NotRun` 維持）
- 実 Window/実 OS GUI L2/L3、Native Dock 画素（`scarecrow173/ruviform#388`・`#532` 領域）
- plugin-host/plugin-core/plugin-clap/plugin-vst3/ui-test のソース変更
- `mise.toml` 変更（新 lane 追加禁止 — 既存 `test-ui-plugin-dock` が全 case を一括実行）
- Issue #389 の close（tracker — L2/L3・consumer gates 残存）

## Requirements

- 新 case は artifact 解決に成功し実バイナリ経路を実走した場合のみ `Pass`。artifact 不在では既存 case と同じ fail-closed `NotRun`。
- 既存 5 case の verdict を回帰させない（`view-mount`=pass・`artifact`=pass・`host-lifecycle`=pass・`editor-attach`=unsupported・`dock-pixels`=not_run）。
- `editor-attach`=`unsupported`、`dock-pixels`=`not_run` を維持 — 本 slice は実 attach 成功経路を持たない。
- `environment_kind`=`real-abi-test-plugin` は実走 case のみが名乗る。fake slot 上の契約適合を real-window/dock 証拠と主張しない。
- session の各 step・event 列・state 遷移を `observed` に記録し、setup 段階の失敗は黙った skip ではなく `Fail` に分類する。
- 設計書 reconcile は「改訂」履歴を残す形式で行い、未実施項目を検証済みと書かない。

## Acceptance Criteria

- [ ] `run_all` が 8 case を返し、`session-lifecycle`/`host-terminate`/`open-or-degrade` が実バイナリ実走で `Pass`（artifact 実走時）・artifact 不在で `NotRun` になる
- [ ] `tests/plugin_dock.rs` が新 case 数・fail-closed NotRun・実 verdict を断言する
- [ ] `README.md` verdict 表に新 case 行が正直な verdict で追加される
- [ ] `mise run test-ui-plugin-dock` 実走の JSONL が `evidence/rf389-dock/` に記録され、全 case verdict が確認できる
- [ ] `docs/designs/issue-389-plugin-editor-session.md` の R5/D8/D5/R2/R4/R6/`WIRE_VERSION` 記述が現行コード事実と一致する
- [ ] review-protocol 1 cycle 以上実施・blocker なし

## Deferred Design Concerns

- 実 editor attach（`test_editor` fixture）・実 Dock/Native Dock 画素・実 OS GUI L2/L3 — 後続 slice。本 slice の refused-open 経路は `Failed{Unsupported}` の観測であり attach 成功の証拠ではない | reason: fixture 未整備・`scarecrow173/ruviform#630` 残存 scope | re-engagement trigger: `test_editor` 到着・L2/L3 lane 整備 | risk if ignored: Headless 成功を実 Plugin Pass と誤認する危険（本 spec の正直 verdict 規律で抑止）

## Sources and Relations

- Issue `scarecrow173/ruviform#389`（tracker — auto-close しない）
- `docs/designs/issue-389-plugin-editor-session.md`（reconcile 対象の正本設計書）
- `docs/specs/issue-631-plugin-dock-scenario.md`・`docs/tasks/issue-631-plugin-dock-scenario.md`（先行 harness slice）
- `docs/tasks/issue-389-vst3-l0-editor-attach.md`・`docs/tasks/issue-389-clap-l0-editor-attach.md`（L0 attach merge 済み slice — 設計書 reconcile の根拠）
- `docs/impl/exp/issue-389-parent-lease.md`（`GuiParentToken` 削除・`ParentLease`/`WIRE_VERSION=5` の記録）
- `POLICIES.md`（OS-L1/GUI-L2/EXT-L3・Pass/Fail/NotRun/Unsupported 規範）
