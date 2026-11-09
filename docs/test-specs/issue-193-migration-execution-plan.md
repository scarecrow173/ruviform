---
id: "TSPEC-034UCSrsj3GYcyfu8KYMB2"
type: "test-spec"
status: "approved"
title: "Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 設計正本"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/193"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-193-migration-execution-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-193-migration-execution-plan.md"
    - "https://github.com/scarecrow173/ruviform/issues/193"
  derived-by: []
  verifies:
    - "docs/designs/issue-193-migration-execution-plan.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/194"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/synvelis/issues/23"
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  defers: []
  deferred-by: []
---

# Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 設計正本

## Purpose

Issue #193 の設計正本（[design](../designs/issue-193-migration-execution-plan.md)）
が「Issue #193 本文（2026-11-09 時点）の Migration
Execution Plan v1.0 — M0〜M7・依存 DAG・受入 Gate G0〜G7・
Cross-Consumer 受入・Rollback RB0〜RB6・GUI lane・WebView
撤去 lane・進捗/Close 条件を、権威関係を保って構造化し、
未検証を合格に昇格させず、merge 済み正本のみを参照して
構成されている」ことを検証する。対象 spec は
[spec](../specs/issue-193-migration-execution-plan.md)、
実施記録は
[task](../tasks/issue-193-migration-execution-plan.md) /
[plan](../plans/issue-193-migration-execution-plan.md)
を参照。コード変更を伴わない docs 成果物のため、検証は
文書検査（link/relation 解決・参照先の実在確認・内容の
突合・lint）で行う。本 test-spec は Execution Plan
canonical 文書の正しさを対象とし、M0〜M7 の実装・gate
受入・Consumer 実測そのものの合否は対象外（#193・
#219/#281/#288/#294・各 Consumer Issue の要件）。

## Feature

Issue #193 Migration Execution Plan v1.0 設計正本
（M0〜M7 実行順・依存 DAG・G0〜G7 受入 Gate・Cross-
Consumer 受入・RB0〜RB6 Rollback・GUI/WebView lane・
進捗/Close 条件の canonical 記録）

## Rules

- **R1（権威関係の非侵害）**: 設計文書は umbrella 設計文書
  `docs/designs/synvelis-driven-target-architecture-and-migration.md`
  を設計権威、Issue #193 本文を実行権威と明記し、本書が
  それらを supersede しないことを relations・本文の両方
  で固定する。領域別詳細設計の再定義・新規 crate/API
  の確定を行わない。
- **R2（M0〜M7 の完全対応）**: M0〜M7 の全 8 段階が
  「区分/優先度・成果物/判断・対応 Issue・開始/完了
  境界」を持つ表として存在し、Issue #193 本文の対応先
  Issue（#194・#184・#195・#189/#190/#188/#186・#196・
  #187/#197・#191/#198・#181/#182/#185）と一致する。段階の
  脱落・統合・改名がない。
- **R3（依存の意味の保持）**: 依存 DAG の記述が「M0 契約
  /Fixture → M1 Graph IR → M2 Audio Runtime、M3
  Time/Span → M4 Plugin Event/Process、Media
  Reader/WAV・Mock Recording・UI は独立可能、M4 を M1〜
  M3 全面完了まで一律停止しないが未確定 ABI/Clock/State
  ownership の変更を公開しない」を保持する。
- **R4（G0〜G7 の完全対応）**: G0〜G7 の全 8 ゲートが
  判定条件を持つ表として存在し、「すべて証拠リンク必須」
  の要件が明記される。Synvelis EX/TA/PPQN/TT/MR の
  該当 test ID 範囲が本文どおり記録される。
- **R5（Cross-Consumer 対等性）**: Gate 別 Consumer 証拠
  表が G0〜G7 全行に Synvelis/AudioLens 両列を持ち、
  運用判定（`impacted / no impact / unknown` の独立分類・
  証拠記録項目・固定 SHA 凍結をしない方針・#219 Owner・
  未検証を完了済みと称さない）が本文どおり記録される。
- **R6（GUI lane の Start/Integration/Acceptance 分離）**:
  GUI lane が Start（#384/#262 並行・#385/#386/#388 調査/
  設計・#389 state/type 設計の同時開始可、#388 実
  NativeSlot PoC は #386 最小 Fixture 前提）、
  Integration（#385+#386→#387、#386+#388+#389→SV #176、
  #386/#387→SV #175/#112・AL #166、#390/#313/#314 並行
  接続）、Acceptance（#262 採否は #388/#389+#390 Evidence、
  SV #114、AL #166 独立、#219/#294 実 SHA tuple 判定）の
  3 区分で記録され、Issue 着手を #262 採用完了へ依存
  させる循環禁止が明記される。
- **R7（受入条件 4 項目 + DoD の対応）**: Issue #193 本文
  の受入条件（#384 後継 ADR の Evidence 承認・旧
  API/Frontend 削除の明示判断・WebView Code 一括破棄
  禁止 / 暗黙 Issue 依存の新設禁止 / Owner 分界・
  Driver/Component 二重実装・Cross Consumer CI 重複禁止
  / DAG と Owner 表の正本維持と両 Consumer 最新 Issue
  リンク）と GUI lane DoD が、設計文書に対応する記述
  として存在する。
- **R8（Rollback RB0〜RB6 の完全対応）**: RB0〜RB6 の全
  7 項目が領域と保護/復旧方針を持つ表として存在し、
  Git revert だけでデータ/公開 API が戻ると仮定しない
  等の本文の要点を保持する。
- **R9（WebView 撤去 lane の独立性）**: #465 を撤去
  Owner とする独立 lane が Start（即時・#455 Close・新機能
  停止・棚卸し）/Integration（共通契約の WebView/JS/DOM
  分離・semantics/fixtures の GPUI 経路移行・parity を
  必須 Acceptance にしない）/Acceptance（#419・SV #114/#176・
  AL #166+#127・#219/#294・終了条件を満たした範囲のみ撤去）
  の 3 区分で記録され、旧実装 Closed 履歴の保持対象
  （SV #110/#111、AL #105/#131/#132、RF #156/#310〜312/
  #423/#431）が列挙される。
- **R10（状態分離と未検証の非昇格）**: Design Approved /
  PoC Verified / Implementation Complete / CI Verified /
  Platform Verified / Migration Accepted の状態分離、
  「Closed な旧 Issue は実装完了証拠ではない」「Issue
  更新は実装・テスト PASS を意味しない」「未実測事項を
  完了に昇格させない」が明記され、実装ステータス同期
  （2026-11-04）の M0〜M7 進捗表と横断受入 Owner が
  snapshot として記録される。
- **R11（参照の実在性と merge 済み限定）**: 設計文書が
  参照する docs パス（umbrella 設計文書・#194 discovery・
  #219 spec・#257/#384 設計文書・関連 ADR 各件・領域別
  issue-* 文書）と Issue/PR 番号が作成時点で実在し、
  別 Worker が並行作業中の未 merge 文書（#192 設計
  チェーンの `issue-192-*` 文書等）を参照していない。
- **R12（docs のみの変更）**: 本チェーンの変更が
  `docs/` の新規 `issue-193-*` 文書と生成 index のみで
  あり、`crates/`・`Cargo.toml`・`Cargo.lock`・
  `mise.toml`・`.github/` に差分がない。

## Examples

- **EX-1（権威確認）**: 設計文書 Context を読むと、
  umbrella 設計文書（approved・設計権威）と Issue #193
  本文（実行権威・本書が canonical 記録）と領域別詳細
  設計の 3 層の権威関係が分かり、umbrella 本文の
  crate 配置・概念 API を本書が再定義していない。
- **EX-2（gate 解決）**: G4 を引くと ProcessConfig
  min/max 厳守・256/100/128 Frame の具体例・EngineFrame
  offset 対応・TT-025〜030 が辿れ、Cross-Consumer 表で
  Synvelis（Plugin min/max process・state/RT lifecycle）
  と AudioLens（共有 crate/API/feature 非退行）の両契約
  が対応付けられる。
- **EX-3（GUI lane 分離）**: GUI lane 節を読むと、#388
  の実 NativeSlot PoC が #386 最小 Fixture のみを前提と
  し全 Host 完成を待たないこと、#262 採否が #388/#389/
  #390 の Evidence を参照する最終判定であること、Issue
  着手が #262 完了へ循環依存しないことが確認できる。
- **EX-4（受入条件対応）**: Issue #193 本文の受入条件
  4 項目を設計文書で引くと、GUI lane 制約節（#384 ADR
  Evidence 承認・削除の明示判断・WebView 一括破棄禁止）、
  暗黙依存禁止、Owner 表（Ruviform 共通/Synvelis 固有/
  AudioLens 固有・二重実装/CI 重複禁止）、DAG と Owner
  の表（DoD）がそれぞれ対応行として存在する。

## Guarantees

- 設計文書は spec の Acceptance Criteria（5 文書チェーン・
  受入条件対応・M0〜M7/G0〜G7/RB0〜RB6/GUI lane/WebView
  lane/Cross-Consumer 表の存在・docs のみ変更・audit
  error 0）を全て満たす構成を持つ。
- 本 test-spec の R1〜R12 が文書検査により検証可能で
  あり、各 Rule が失敗時に具体的な欠陥（権威関係の曖昧
  さ・段階/ゲート脱落・依存意味の改変・GUI lane 区分の
  混同・受入条件未対応・Rollback 脱落・参照切れ・未検証
  の実施済み主張・範囲外差分）を検出できる。
- 参照先が全て merge 済み正本に実在し、snapshot 日付と
  権威関係が明示されているため、後続の担当者が正本へ
  安全に遷移できる。

## Non-goals

- G0〜G7・TA-001〜TA-012・consumer-verify の実際の合否
  判定（#193・#219/#281/#288/#294・Consumer repo の
  要件）。
- M0〜M7・GUI lane・WebView 撤去の実装内容の正しさの
  再検証（各正本の test-spec が所有）。
- umbrella 設計文書・Issue #193 本文の改訂。
- 実装コード・テスト・CI の検証（本 slice は docs のみ）。

## Risk

- Issue #193 本文への追記で snapshot が古くなると計画
  正本の信頼性が低下する → 日付明示と「本文更新時は
  docs 正本の同期更新」の義務付けで緩和。
- 「merge 済み PoC = 受入済み」の誤読が Consumer へ
  誤った完了シグナルを与える → R10 で状態分離と未昇格
  原則を強制。
- GUI lane の Start/Integration/Acceptance 混同や暗黙
  依存の混入が着手判定を誤らせる → R6/R7 で 3 区分と
  受入条件対応を固定。
- 未 merge 文書への参照が正本の安定性を損なう → R11 で
  merge 済み限定を要求。
