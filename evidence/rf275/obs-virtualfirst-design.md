# 内部 Design — Issue #275 Virtual-first Owner/受入整合（Deferred Owner/Trigger 再指向）

- 文書種別: 内部 design（`obs-virtualfirst-spec.md` の設計相当）
- 日付: 2026-11-15
- 対象 Revision: `d5aa1b4b`（origin/main HEAD、clean tree）

## Context

`docs/designs/issue-275-observability-integration-design.md`
（canonical 設計正本、status: approved）は 2026-11-14 時点の
snapshot を含む。「Deferred 範囲の Owner/Trigger」節の実 OS GUI
行は real-window 系 lane の設計正本として
`docs/designs/issue-313-windows-realos-smoke.md`・
`docs/designs/issue-314-real-scenario-lane.md` を参照し、
「Consumer Gate 割当（要約）」節の Windows 利用機能 L2/L3 行は
RF 側 Owner に `#313/#314 系 lane`・実行経路に「RF real-window
lanes」を記す。

しかし POLICIES.md §2.1（2026-11-15 決定「仮想優先の受入と
共通テスト基盤」）と Issue #275 本文「2026-11-15
Virtual-first正式受入Owner整理」は、実 OS GUI・実 Device
API・Plugin Host の L2/L3 検証を RF #628〜631 共通 Harness
＋ RF #281/#294＋ product owner（AL#127・SV#28）へ委譲する
方針を定めた。旧参照先の #313/#314 は CLOSED であり、
本書の当該参照は陳腐化している — Issue #275 受入 item
「現行 Virtual-first の Owner/受入整合」が未達である直接の
原因。

## Scope

### In Scope

- canonical 設計正本への参照差し替え＋dated note＋
  front-matter `updated` 更新
- `evidence/rf275/obs-acceptance-ledger.md` への追記専用
  セクション（§7）
- `evidence/rf275/obs-virtualfirst-*` 内部 doc chain と
  verification 記録

### Out of Scope

- spec 記載の Out of Scope 全般（コード・CI・canonical docs
  新規追加・index 再生成・msrv/full 系実行・Consumer repo
  実測・Issue 操作）

## Components and Boundaries

### 変更対象 1: Deferred 範囲の Owner/Trigger 表

対象節: `## Deferred 範囲の Owner/Trigger`（現行 L539-560）。
表は 4 行（実外部 Provider/実 OS GUI/実 Device/長時間
Profiler）。変更は「実 OS GUI」「実 Device」の 2 行の
再開 Owner 列と Activation Trigger 列、および表の直後への
dated note 追加に限定する。

- **実 OS GUI 行**: 再開 Owner を `RF #293（API 境界）+
  利用機能の Owner Issue（AL #127/SV #28 の Windows L2
  Case）` に、Virtual-first Harness として `RF #631`
  （real-GUI/plugin-dock runner — OPEN）と `RF #630`
  （ABI Test Plugin — OPEN、Native Editor/Plugin Dock の
  L3 に必要）を追加し、方針基盤 `RF #627`（CLOSED・
  POLICIES §2.1 仮想優先契約の設計正本）を併記する。
  旧 `#313/#314` 系 lane 正本参照は「Issue CLOSED・本行の
  検証経路は #628〜631 共通 Harness へ再指向」と明記して
  置き換える。
- **実 Device 行**: 再開 Owner を `RF #281（OBS-06 gate）+
  device 利用機能の Owner Issue` に、`RF #629`（OS virtual
  endpoint survey/probe — OPEN、本番 Audio/MIDI API を通す
  仮想 Endpoint での L3 検証）と `RF #628`（deterministic
  test runtime — OPEN、VirtualClock/PCM/Fault の OS-L1 契約
  試験基盤）を追加する。物理機材は POLICIES §2.1 どおり
  原則必須 Gate から外し、物理 Case は正式サポート宣言・
  個別不具合調査時のみ定義する旨を Trigger に反映する。
- **実外部 Provider 行・長時間 Profiler 行**: 本 slice の
  対象外（#628〜631 Harness の守備範囲外。変更しない）。

### 変更対象 2: Consumer Gate 割当（要約）表

対象節: `## OBS-01〜11 受入照合台帳と Consumer Gate 割当`
内の「Consumer Gate 割当（要約）」表（現行 L585-595）。
「Windows 利用機能 L2/L3 故障・RT/Privacy」行のみ変更:

- RF 側 Owner: `#281・#294・#293・#313/#314 系 lane` →
  `#281・#294・#293・#628〜631 共通 Harness（方針基盤
  #627）`
- 実行経路: 「RF real-window lanes」→「RF Virtual-first
  Harness lanes（OS virtual endpoint・ABI Test Plugin・
  real-GUI runner）」
- `#281`・`#294`・`AL #127`・`SV #28` の委譲と「未利用機能
  は Required にしない」の原則は維持する。

### dated note の配置

- Deferred 節: 表の直後（「なお **feature compile matrix**」
  段落の前）に `（2026-11-15 追記）` の段落を追加し、
  POLICIES §2.1 への再指向・#627〜631 の役割と状態・
  #313/#314 CLOSED の事実・3 原則（NotRun 正直記録・
  物理機材/市販 Plugin の一括 Required 化なし・design-Close
  と impl 受入の分離）を 1-2 文で記す。
- Consumer Gate 節: 割当表の直後の箇条書き末尾に同趣旨の
  短い dated note を追加する（表の要約性を保つため
  詳細は Deferred 節の note に集約）。
- front-matter: `updated: "2026-11-14"` → `"2026-11-15"`。
  `status: "approved"`・relations・他フィールドは不変。

### Ledger への追記（`obs-acceptance-ledger.md` §7）

§6 の precedent（追記専用・既存監査記録は改変しない）に
倣い、`## 7. 追記: Virtual-first Owner/受入整合の再指向
（Issue #275 slice、2026-11-15）` を追加する。内容:

- 本 slice の対象・方法（`gh issue view` 照合・文書差分）
- 参照 Issue の実在照合結果表（番号・状態・役割）
- 充足済み item の根拠: OBS-01〜11 API/Feature/Owner/
  OS-L1 fixture matching は §1〜§4.2 で充足済み、
  AL/SV consumer-gate assignment は §3 で充足済み —
  本 slice は ledger の追記で照合根拠を記録するのみで
  checkbox 更新は Coordinator 側 bookkeeping
- Virtual-first 再指向の内容要約（design doc の差分概要）
- §3「Windows 利用機能 L2/L3」行・§4.2 の `RF #293/#313/
  #314` 記述は監査時点（2026-11-14）の記録として保持し、
  現行 Owner は本節と設計正本の dated note が正本である旨

### 内部 doc chain と evidence

`evidence/rf275/` に `obs-virtualfirst-{spec,design,plan,
task,evidence}.md` を新規作成。`evidence` は verification
記録（実行コマンド・exit code・Issue 実在照合結果・変更
差分要約）を所有し、ir/exp 相当の内部記録を兼ねる。

## Data and Control Flow

文書のみ。流れは:

1. `gh issue view` で参照 Issue の実在・状態を照合。
2. canonical 設計正本の 2 箇所の参照を差し替え、dated
   note を追加。
3. ledger §7・内部 doc chain・evidence を `evidence/rf275/`
   に記録。
4. `mise run fmt-check`・`prek run --all-files`・
   `git diff` 範囲確認で検証。

## Non-Functional Constraints

- **正確性**: Issue 状態・役割は `gh issue view` 実測。
  存在しない Issue・役割を根拠にしない。
- **非改変**: 既存の status・承認記録・ledger §1〜§6・
  owner-map audit・「実装の現状」snapshot を改変しない。
- **非昇格**: `NotRun`/`Deferred` を `Pass` に置き換え
  ない。lane/Issue の存在を実行済みの根拠にしない。
- **保護対象**: spec の Out of Scope 全般を変更しない。

## Risks and Trade-offs

- **参照差し替えの drift**: dated note で旧参照の失効理由
  （#313/#314 CLOSED・POLICIES §2.1）を明記し、ledger §7
  が現行 Owner の索引となる構造にする。
- **表の肥大化**: Owner 列は番号・役割・状態の最小表記に
  留め、詳細は dated note と ledger §7 に集約する。
- **#630/#631 の役割誤認**: #630 は ABI Test Plugin
  fixture、#631 は real-GUI/plugin-dock runner であり、
  役割を取り違えないよう issue 題名と設計正本
  （`docs/designs/issue-630-test-plugin-suite.md`・
  `issue-631-plugin-dock-scenario.md`）で確認済み。

## Deferred Design Concerns

- #628〜631 の実装進捗と lane 整備 | 各 Issue・#281/#294
  gate | 本 slice は参照の整合のみ。Harness 未実装分は
  `NotRun` のまま。
- design-Close 判定 | Coordinator/Owner | 本 slice の
  merge は Issue #275 全体の完了を宣言しない（PR 本文は
  `Refs #275`、`Closes` 禁止）。

## Verification Notes

- [ ] `mise run fmt-check` exit 0
- [ ] `mise exec -- prek run --all-files` exit 0
- [ ] `git diff` で変更が指定 2 ファイル＋`evidence/rf275/`
      新規ファイルに限定されること
- [ ] 参照 Issue 全件の `gh issue view` 結果が evidence に
      記録されていること
- [ ] review-protocol: docs-only 小 slice — proportionate
      な self-review（quick cycle または triviality の正直な
      記録）で blocker 0
