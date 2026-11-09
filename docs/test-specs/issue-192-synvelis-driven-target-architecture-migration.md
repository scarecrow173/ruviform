---
id: "TSPEC-034UC88veX939aXOGHd2sw"
type: "test-spec"
status: "approved"
title: "Issue #192 Synvelis-driven Target Architecture & Migration — umbrella migration 設計正本"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
      - type: "doc"
        path: "docs/specs/issue-192-synvelis-driven-target-architecture-migration.md"
      - type: "doc"
        path: "docs/plans/issue-192-synvelis-driven-target-architecture-migration.md"
      - type: "doc"
        path: "docs/tasks/issue-192-synvelis-driven-target-architecture-migration.md"
      - type: "doc"
        path: "docs/test-specs/issue-192-synvelis-driven-target-architecture-migration.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #192 Synvelis-driven Target Architecture & Migration — umbrella migration 設計正本

## Purpose

Issue #192 の設計正本（[design](../designs/issue-192-synvelis-driven-target-architecture-migration.md)）
が「umbrella 設計文書と #193 を権威とし、TA-001〜TA-012・
M0〜M7・G0〜G7・GUI lane・Consumer 接続・ADR/互換
policy の対応索引を、実在する正本への参照で構成し、
設計合意と実装/検証の完了を混同していない」ことを検証
する。対象 spec は
[spec](../specs/issue-192-synvelis-driven-target-architecture-migration.md)、
実施記録は
[task](../tasks/issue-192-synvelis-driven-target-architecture-migration.md) /
[plan](../plans/issue-192-synvelis-driven-target-architecture-migration.md)
を参照。コード変更を伴わない docs 成果物のため、検証は
文書検査（link/relation 解決・参照先の実在確認・内容の
突合・lint）で行う。本 test-spec は umbrella tracking
canonical 文書の正しさを対象とし、M0〜M7 の実装・gate
受入・Consumer 実測そのものの合否は対象外（#193・
#219/#281/#288/#294・各 Consumer Issue の要件）。

## Feature

Issue #192 umbrella migration tracking canonical 設計文書
（TA gate 対応・capability→正本 map・Consumer 接続面・
ADR/互換 policy・設計/実装分離）

## Rules

- **R1（権威の非侵害）**: 設計文書は umbrella 設計文書
  `docs/designs/synvelis-driven-target-architecture-and-migration.md`
  と Issue #193 を設計・実行の権威と明記し、本書が
  それらを supersede/replaces しないことを relations・
  本文の両方で固定する。領域別詳細設計の再定義・新規
  crate/API の確定を行わない。
- **R2（TA gate の完全対応）**: TA-001〜TA-012 の全 12
  項目が「要求要約・担当正本/Issue・証拠レーン・現状」
  の 4 要素を持つ表として存在し、各項目が umbrella
  Issue #192 本文の受入条件と 1:1 で対応する。項目の
  脱落・統合・改名がない。
- **R3（capability map の網羅性）**: M0〜M7 の全段階・
  GUI Driver lane・IO 共通基盤・Observability・
  Cross-Consumer gate の各領域が、担当 Issue・正本
  docs パス・merge 証拠・未解決事項を持つ行として存在
  する。merge 証拠は #193 進捗表または git log の merge
  commit に基づく記述であり、未検証の merge を主張しない。
- **R4（参照の実在性）**: 設計文書・spec・plan・task が
  参照する Issue 番号（#181〜#198・#219・#243・#251・
  #253・#262・#275〜#306・#384〜#390・#409・#464・
  #465・Synvelis #17〜#28/#82〜#84/#106/#109/#175〜
  #177・AudioLens #95/#127/#139〜#143/#166）・PR 番号
  （#200・#224・#225〜#240・#250・#252・#274・#519〜
  #522）・docs パス（umbrella 設計・discovery #194・
  spec #219・領域別 issue-* 文書・ADR 各件）が作成時点で
  実在する。存在しない Issue/文書/PR/commit を根拠と
  して記載しない。
- **R5（ADR status の正確性）**: ADR 対応表の status が
  各 ADR front matter の実値と一致する。特に
  `ui-architecture-webview-hybrid` = superseded（
  `driver-neutral-gui-gpui-driver-webview-first-issue-384`
  への superseded-by）、`driver-neutral-gui-…-issue-384`
  = proposed（仮決定）、`configurable-ppqn-…` =
  accepted かつ #189 で実装済み、`audio-engine-snapshot-
  swap` = accepted かつ存続、を正しく反映する。
- **R6（3 者 Ownership の両 Consumer 対等性）**: Ownership
  boundary 表が Ruviform/Synvelis/AudioLens の 3 列を持ち、
  AudioLens の f64 精度・native PCM/PTS・Dataset
  provenance・GUI/CLI headless 分離が Synvelis の DAW
  意味論と同列に記述される。片方の Consumer を基盤へ
  従属させる記述がない（#192 2026-11-02 追補準拠）。
- **R7（Consumer 接続面の責務分界）**: Synvelis/AudioLens
  各 Issue の表が「Consumer が所有する schema/
  revision/precision/DAW・解析意味論」と「Ruviform が
  提供する汎用能力」の分界を明記し、schema/version を
  Ruviform へ移す記述がない。
- **R8（設計/実装/受入の分離）**: 「merge 済み
  （headless/CI/PoC）」と「Consumer 受入・実機検証・
  gate 正式 Close」を別状態として記述し、「設計
  approved ≠ 実装完了 ≠ Consumer 受入」の判定原則と
  `not run`/`NotRun`/`Unsupported` の記録方針が明記され
  る。未実施の検証を実施済みと記載しない。
- **R9（snapshot 日付管理）**: Issue/PR/status の記述が
  「2026-11-09 時点の snapshot」として日付付きで記録
  され、変動する状態を恒久的事実として扱う記述がない。
- **R10（docs のみの変更）**: 本チェーンの変更が
  `docs/` の新規 `issue-192-*` 文書と生成 index のみで
  あり、`crates/`・`Cargo.toml`・`Cargo.lock`・
  `mise.toml`・`.github/` に差分がない。

## Examples

- **EX-1（権威確認）**: 設計文書 Context を読むと、
  umbrella 設計文書（approved）と #193 が設計・実行の
  権威であり、本書が tracking canonical であることが
  分かる。umbrella 文書の crate 配置・概念 API・Rollback
  本文を本書が再掲・改訂していない。
- **EX-2（gate 解決）**: TA-010 を引くと #191・#198・
  IO #287/#305/#306・SV #25 へ辿れ、「#191/#198 merge
  済み・SV #25 E2E-L2 完了・IO E2E #305/#306 OPEN」と
  証拠状態が分離されている。
- **EX-3（ADR 現状）**: ADR 表で `ui-architecture-
  webview-hybrid` が superseded、後継 #384 ADR が
  proposed（仮決定・#262 evidence 待ち）と読め、
  umbrella §8 の「維持」記述が 2026-11-01 時点のもので
  あることと矛盾なく現状が把握できる。
- **EX-4（consumer 分界）**: AudioLens #139/#143 の行が
  Manifest/Dataset schema を AL が所有し Ruviform は
  `StagedFile`/Structured の汎用能力のみ提供する分界を
  示し、Synvelis #84 が IO-05 breaking change の consumer
  側対応として OPEN であることが対応付けられる。

## Guarantees

- 設計文書は spec の Acceptance Criteria（権威非侵害・
  TA 対応・capability map・3 者 boundary・ADR 現状・
  設計/実装分離・docs のみ変更）を全て満たす構成を持つ。
- 本 test-spec の R1〜R10 が文書検査により検証可能で
  あり、各 Rule が失敗時に具体的な欠陥（参照切れ・
  gate 脱落・status 誤記・分界違反・未検証の実施済み
  主張）を検出できる。
- 参照先が全て実在し、snapshot 日付と権威関係が明示
  されているため、後続の担当者が正本へ安全に遷移できる。

## Non-goals

- TA-001〜TA-012・G0〜G7・consumer-verify の実際の
  合否判定（#193・#219/#281/#288/#294・Consumer repo
  の要件）。
- 領域別設計（#184〜#198・IO・Observability・GUI）の
  内容の正しさの再検証（各正本の test-spec が所有）。
- umbrella 設計文書・#193 本文の改訂。
- 実装コード・テスト・CI の検証（本 slice は docs のみ）。

## Risk

- 対応表の snapshot が古くなると gate 追跡の信頼性が
  低下する → 日付明示と参照正本への誘導で緩和。
- 権威関係の記述が曖昧だと umbrella 設計文書との二重
  権威が生じる → R1 で relations と本文の両面を固定。
- Issue 状態の誤記（OPEN/CLOSED の取り違え）が consumer
  接続判断を誤らせる → R4/R9 で作成時点の実測との
  突合を要求。
- 「merge 済み」を「受入済み」と読み違える記述が
  Consumer へ誤った完了シグナルを与える → R8 で状態
  分離を強制。
