---
id: "EXP-issue-193-virtual-first-owner-remap"
title: "Issue #193 Virtual-first owner remap 監査メモ — Issue 状態照合と Snapshot 節構造照合"
created: "2026-11-15"
updated: "2026-11-15"
---

# Issue #193 Virtual-first owner remap 監査メモ(2026-11-15)

- 計測契約: `docs/specs/issue-193-virtual-first-owner-
  remap.md` + `docs/designs/issue-193-virtual-first-
  owner-remap-design.md`
- 検証記録の所在: 本 slice の `evidence/` は lease 外
  のため、本メモと `docs/impl/ir/issue-193-virtual-
  first-owner-remap.md`・各 Verification Notes が正本。

## 1. 問い

Issue #193 の 2026-11-15「Virtual-first正式受入
Owner整理」を canonical 計画文書へ記録するにあたり、
(a) 参照 Issue の実在と状態、(b) 両 canonical の
`Snapshot-2026-11-14` 節の同期構造、(c) 新規 dated 節の
語彙・対応付け規則 — をどう固定するか。

## 2. 調査と実測

### Issue 状態（`gh issue view`、2026-11-15）

- RF #627 CLOSED — [design/test][RF-VTEST-00]
  仮想優先の3repo検証契約・物理Gate置換・POLICIES
  整合と独立Oracle（POLICIES §2.1 の契約正本）。
- RF #628 OPEN — [impl/test][RF-VTEST-01]
  Ruviform製Deterministic Audio/MIDI Test Runtime・
  Virtual Clock・Fault/Oracle。
- RF #629 OPEN — [test/device][RF-VTEST-02] Windows
  実OS仮想Audio/MIDI Endpoint・Provisioning・本番API
  通過Gate。
- RF #630 OPEN — [test/plugin][RF-VTEST-03]
  Ruviform製VST3/CLAP Test Plugin Suite・実ABI/
  State/RT/Native Editor。
- RF #631 OPEN — [test/gui][RF-VTEST-04] Ruviform
  GPUI実Window Scenario Runner・Test GUI Dogfooding・
  Pixel/Receipt/Evidence。
- RF #294 OPEN（candidate tuple gate）、RF #275 OPEN
  （OBS）、RF #251/#253/#388/#389/#532 OPEN（実装・
  mechanism owner）、AL #127・SV #28 OPEN（Consumer
  Oracle）、RF #193 OPEN（perpetual tracker）。
- 結論: #628〜631 は全て OPEN の未実装 Harness で
  あり、対応表は受入 owner の対応付けのみを記録し、
  Case 実走・Pass を意味しない（`NotRun` 維持）。

### Snapshot 節構造（両 canonical の diff 実測）

- `docs/designs/issue-193-migration-execution-plan.md`
  の `## Snapshot-2026-11-14` 節（`## Deferred Design
  Concerns` 直前まで）と `docs/plans/issue-193-
  migration-execution-plan.md` の同名節（`##
  Implementation Handoff` 直前まで）を抽出して diff
  したところ、差異は版識別ヘッダの設計根拠リンク部
  のみ（designs は `issue-193-migration-plan-update-
  design.md` への相対リンク 1 行、plans は設計正本側
  2 文書への `../designs/` リンクと言及を含む 4 行）。
  本文（Issue 状態照合・M0〜M7 表・並行 Lane・
  Deferred/Activation 表・AL/SV Oracle・DAG 表・
  Verification Notes）は一致 — 「同名 Snapshot 節
  （内容は一致させる）」の約束が守られていることを
  実測確認した。
- 帰結: 新規 dated 節は両ファイルで**バイト一致**が
  必須。相対 markdown リンクはファイル間で解決が
  変わるため使わず、Issue 参照は GitHub URL、文書
  参照は repo 相対パスの inline code とした。

### 挿入位置と append-only

- designs: `## Deferred Design Concerns` 直前、
  plans: `## Implementation Handoff` 直前 — 前回の
  Snapshot 追加と同じ配置慣行（版付き節を時系列で
  積む）。
- `git diff` で両ファイルとも +142 行・削除 0 の
  append-only であることを確認。`Snapshot-2026-11-14`
  節を含む既存本文に差分なし。

## 3. 採用した判定

- 対応表は per-milestone 4 列（Milestone/領域・
  OS-L1・GUI-L2/EXT-L3 受入 owner・残存 owner）。
  OS-L1 は変更なし（既存 Daily/手動 CI・#294 tuple・
  AL #127/SV #28）。L2/L3 対象のない M0/M1/M2/M6 は
  「なし/変更なし」と正直に記した。
- GUI-L2 → #631、Audio/MIDI 実 OS 仮想 Endpoint
  EXT-L3 → #629、実 Plugin ABI → #630、共通基盤
  #628・方針基盤 #627（CLOSED）を併記。M3 は #253
  の OS MIDI・VST3 翻訳を #629+#630 へ、M4 は #388/
  #389/#532 の Native Dock・実 ABI を #631+#630 へ、
  M5 は #251 の実 Audio Device を #629 へ、M7/GUI
  lane を #631+#630 へ写像した。
- 旧 owner 記述との関係は dated 上書き宣言で処理:
  `Snapshot-2026-11-14` 節の Deferred/Activation 表・
  Integration Gate 行・GUI lane・DAG 表が指す
  #251/#253/#388/#389/#532 を名指しで列挙し、実装・
  mechanism・製品 Owner として残存（削除・格下げ
  しない）としつつ、「2026-11-15 以降の受入 owner
  正本は本節」と宣言した（issue-275 の 2026-11-15
  追記と同型）。
- 非昇格を維持: Required→Deferred 格下げなし、
  `NotRun`→`Pass` 置換なし、新規 `Pass` 付与なし、
  DComp 既知 Fail 維持、物理機材・市販 Plugin の
  一括 Required 化なし、Headless 実装 Slice と製品
  最終受入の分離を明記。

## 4. 棄却した選択肢

- 旧 Snapshot 節の Owner 列を直接書き換える案 →
  「内容は一致させる」約束と履歴 snapshot 保持の
  原則に反するため棄却。dated 節の追加で更新を
  表現する append-only 方針を採用。
- `evidence/rf193/` への監査証拠ファイル作成案 →
  本 slice の lease 外のため棄却。検証記録は doc
  chain（本メモ・IR・Verification Notes）に集約。
- 新規節で相対 markdown リンクを使う案 → 両ファイ
  ルでリンク解決が変わり完全一致を壊すため棄却。
  GitHub URL / repo 相対パスの inline code を採用。
