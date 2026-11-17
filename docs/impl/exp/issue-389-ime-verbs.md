# Issue #389 rf389-ime-verbs — 実装実験記録（2026-11-18）

`docs/tasks/issue-389-ime-verbs.md` の実装で試行・観測した
事項の記録。実装結果の正本は
[`docs/impl/ir/issue-389-ime-verbs.md`](../ir/issue-389-ime-verbs.md)、
イベントログは
[`issue-389-ime-verbs-exp.jsonl`](issue-389-ime-verbs-exp.jsonl)
を参照。

## 試行 1: `InProcessEditorOps` verb の lease redeem 経路

- **目的**: `reattach` は新しい `ParentSlotLease` を受け取るが、
  plugin へ渡す `ParentLease` は issuer 側 seam でしか作れない。
- **観測**: `InProcessEditorOps` は `open` で使った `LeaseRedeemer`
  を保持しており、`reattach` も同じ redeemer を呼べば
  stale/forged lease を op 発行前に拒否できる。redeem を
  `host.execute` の後に置くと拒否済み lease が plugin へ
  到達しうるため、redeem-first の順序を採用。
- **結論**: `reattach` = `(self.redeemer)(lease)?` →
  `HostCommand::SetGuiParent`。headless test
  `l0_redeemer_refusal_blocks_reattach_before_setguiparent` で
  発行前拒否を検証。

## 試行 2: composite の L0-only verb guard

- **観測**: `to_wire` は `Scan`/`Instantiate`/`Destroy` を
  `unreachable!` で弾く構造で、新 verb を追加すると
  `OutOfProcess` slot dispatch が panic へ到達しうる。
- **結論**: `l0_only_gui_verb` predicate を `Slot::OutOfProcess`
  分岐の先頭に置き、`to_wire` 到達前に defined
  `HostError::Unsupported` を返す。`to_wire`/`command_handle`/
  `retarget` の match は機械追従（unreachable arm に verb を
  明記）。

## 試行 3: VST3 hide/show の format 意味差

- **観測**: `IPlugView` に hide/show verb は存在しない。
  hide を `removed()`（detach）で実現すると plugin UI state が
  失われるが、show は `setFrame`+`attached` で同じ parent/frame
  に再構成できる。これは format 制約であり fake hide ではない
  — 実際に view が parent から外れ、再 attach で新 generation
  の HWND が生まれる。
- **結論**: hide=detach・show=再構成を採用し、design doc と
  adapter コメントに「hide は UI state 喪失の detach 同義」と
  明記。CLAP は `hide`/`show` の真の verb があるため意味差を
  維持したまま同じ session 遷移（`Hidden`/`Shown` event）へ
  写像する。

## 試行 4: lane effect oracle の識別方法

- **観測**: HWND 値は destroy 後に OS が再利用しうるため、
  「旧 child が死んだ・新 child が生まれた」を HWND 値だけでは
  証明できない。fixture の attach ごとに window title へ
  `rvf-test-editor-attach-{n}` を刻印する方式を採用 —
  generation 前進を title 比較で観測できる。
- **結論**: `ChildWindowFacts` に `width`/`height`/`title` を
  追加し、oschild Windows impl が `GetWindowRect`/
  `GetWindowTextW` を query-only で観測。`is_alive` helper で
  `IsWindow` による teardown 証明を行う。resize oracle は全
  child の rect が `800×600` と一致することを要求（1 child でも
  不一致なら `Fail`）。

## 試行 5: verdict 写像の `effect_confirmed` 導入

- **観測**: 従来の accepted op は effect oracle が無く
  `NotRun` 固定だった。`Pass` を出すには「op 受理 + lifecycle
  verified + 効果確認」の 3 条件が必要。
- **結論**: `EditorOpVerification.effect_confirmed` +
  `EditorOpIdentity`（case 別 pass detail/evidence tag）を追加し
  `editor_op_body` に `Pass` arm を実装。refused → `Unsupported`、
  accepted+未確認 → `NotRun`、契約不一致 → `Fail` の既存写像は
  維持。実走で 3 row とも `pass` を確認（`evidence/rf631/
  plugin-dock-real-scenario.jsonl`）。

## 試行 6: scope reconcile（lease 外ファイルの差し戻し）

- **観測**: 初回 commit で `docs/**/README.md`（生成 index）・
  `crates/ruviform-plugin-host/src/lib.rs`・
  `crates/ruviform-plugin-dock-test/src/lib.rs` を変更していたが、
  task spec の編集可能範囲外だった。`plugin-host/src/lib.rs` の
  `GuiSize` re-export は `host.rs` が `ruviform_plugin_core`
  から直接 import する形へ変えれば不要になる。
- **結論**: lease 外 4 file を revert し、docs を spec 指定の
  `issue-389-ime-verbs` 命名へ rename、verification 記録を
  `evidence/rf389/verification.md` への追記へ統合。
