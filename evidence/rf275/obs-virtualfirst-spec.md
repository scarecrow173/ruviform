# 内部 Spec — Issue #275 Virtual-first Owner/受入整合（Deferred Owner/Trigger 再指向・docs-only slice）

- 文書種別: 内部 spec（doc-driven-dev-graph フローの spec 相当。
  対象リポジトリ内の内部文書として `evidence/rf275/` に配置。
  canonical docs chain は編集範囲外のため本ファイルが内部正本）
- 日付: 2026-11-15
- 対象 Issue: https://github.com/scarecrow173/ruviform/issues/275
- 対象 Revision: `d5aa1b4b`（作業開始時の origin/main HEAD、
  clean tree。merge 後 SHA の統合検証は Daily/手動 Full CI の
  対象 SHA 証拠が正本）
- ブランチ: `scarecrow173/rf275-virtualfirst`

## What Should Be Built

Issue #275 の残る受入 item「現行 Virtual-first の Owner/受入整合」
を、docs-only の小 slice として処理する。canonical 設計正本
`docs/designs/issue-275-observability-integration-design.md` が
Deferred Owner/Trigger 表と Consumer Gate 割当で旧方針
（`#313/#314 系 real-window lane`）を参照したまま残っているため、
POLICIES.md §2.1（2026-11-15 決定）の Virtual-first 方針へ
再指向する。

1. **Deferred Owner/Trigger 表の再指向**: 実 OS GUI・実 Device
   系の行の再開 Owner を、RF #628〜631 共通 Harness
   （VirtualClock/PCM/Fault runtime・OS virtual endpoint・
   ABI test plugin・real-GUI runner）＋方針基盤 #627
   （CLOSED・policy 文書）へ更新する。各参照は issue 番号・
   役割・現行状態（open/closed）を明記する。
2. **Consumer Gate 割当の更新**: 「Consumer Gate 割当（要約）」
   の Windows 利用機能 L2/L3 行の RF 側 Owner・実行経路を
   旧 `#313/#314` 参照から #628〜631 Harness 系へ整合させる。
   `#281`・`#294`・`AL#127`・`SV#28` の既存委譲は維持する。
3. **dated 追記と front-matter**: 変更は `（2026-11-15 追記）`
   の dated note として入れ、front-matter の `updated` を
   当日日付に更新する。既存の status・承認記録は変更しない。
   全面書き換え・再構成はしない（追記・参照差し替えのみ）。
4. **Ledger への追記**: `evidence/rf275/obs-acceptance-ledger.md`
   へ追記専用セクションを追加し、本 slice の照合結果
   （充足済み item の根拠＋Virtual-first 再指向の内容）を記録
   する。§6 の precedent どおり既存の監査記録は改変しない。
5. **内部 doc chain**: 監査 slice の先例に倣い、spec→design→
   plan→task→ir/exp 相当の内部文書を `evidence/rf275/` 配下に
   新規追加する。新規 canonical doc は作らない。
6. **Evidence**: `evidence/rf275/` に verification 記録
   （実行コマンド・exit code・参照 Issue の実在照合結果・
   変更差分要約）を残す。

## Why Now

- Issue #275 本文「2026-11-15 Virtual-first正式受入Owner整理」
  が、OBS 設計正本における Windows 提供 GUI/実 Device
  API/Plugin Host の必要 L2/L3 を「Ruviform #628〜631 共通
  Harness と RF #281/#294・製品独立 Owner へ委譲」と明示した。
- POLICIES.md §2.1（2026-11-15 有効）は、実 OS GUI・実
  Device API・Plugin Host の L2/L3 検証を #628〜631 共通
  Harness（VirtualClock/PCM/Fault・OS virtual endpoint・
  ABI test plugin・real-GUI runner）＋ RF #281/#294＋
  product owner（AL#127・SV#28）へ委譲し、未検証は `NotRun`
  と正直に記録する方針を定めた。
- 旧参照先の Issue #313/#314 は CLOSED（2026-11-13/11-11）
  であり、real-window lane 設計書への参照は陳腐化している。

## Scope

### In Scope

- `docs/designs/issue-275-observability-integration-design.md`
  の参照差し替え＋dated note＋front-matter `updated` 更新
- `evidence/rf275/obs-acceptance-ledger.md` への追記専用
  セクション追加
- `evidence/rf275/obs-virtualfirst-{spec,design,plan,task,
  evidence}.md`（本 doc chain・内部文書）
- 参照 Issue の実在照合（`gh issue view`）と証拠記録

### Out of Scope

- `crates/`・`Cargo.toml`・`Cargo.lock`・`mise.toml`・
  `docs/development-environment.md`・`.github/`・`ci/`・
  `POLICIES.md`・`AGENTS.md` の変更
- canonical docs（`docs/specs|designs|plans|tasks|test-specs|
  impl`）への新規文書追加・index 再生成（編集範囲外。
  doc chain は `evidence/rf275/` 内部文書で代替）
- `*-msrv`・`*-full` 系タスクの実行（daily CI の管轄）
- cargo 系検証（docs-only slice — POLICIES §5 比例原則）
- F: ドライブへのビルド/中間生成物の配置
- Consumer repo（audio-lens/synvelis）での実測・Issue 更新
- GitHub blockedBy の登録、Issue 起票/Close、PR マージ
- 他 Worker の既存作業（未統合 branch・Draft PR・証拠）の
  保持対象への変更

## Requirements

- 参照する Issue 番号（RF #627〜631・#281・#294・#313・
  #314・AL #127・SV #28）は全て `gh issue view` で実在と
  open/closed 状態を照合し、証拠に記録する。実在しない
  番号・方針本文と矛盾する記述を黙って書かない。
- 「未検証は `NotRun` と正直に記録」「物理機材・商用
  plugin の一括要求は課さない」「design-Close と impl 受入を
  分離」の 3 原則を dated note に反映する。
- 既存の status・承認記録・監査記録（ledger §1〜§6・
  owner-map audit）を書き換えない。変更は dated note と
  参照差し替えに限定する。
- 結果語彙は POLICIES §3 に従う（要件状態 Required/
  Deferred/NotApplicable × 実行結果 Pass/Fail/NotRun/
  Unsupported）。`NotRun` を `Pass` に置き換えない。

## Acceptance Criteria（本 slice）

- [ ] Deferred Owner/Trigger 表の実 OS GUI・実 Device 系行が
      #628〜631 Harness＋#627 方針基盤を番号・役割・状態付きで
      参照し、`#313/#314` への stale 参照が解消されている。
- [ ] Consumer Gate 割当の Windows 利用機能 L2/L3 行が
      #628〜631 Harness 系へ整合し、#281/#294/AL#127/SV#28 の
      委譲が維持されている。
- [ ] front-matter `updated` が 2026-11-15、変更が dated note
      として記録されている。
- [ ] `obs-acceptance-ledger.md` に追記専用セクションがあり、
      既存 §1〜§6 が改変されていない。
- [ ] `evidence/rf275/` に内部 doc chain と verification 記録が
      ある。
- [ ] `mise run fmt-check`・`mise exec -- prek run --all-files`
      が成功する。
