---
id: "TASK-034IRZB7D249qUO1bLldy4"
type: "task"
status: "todo"
title: "AGENTS.md 暫定原則の確定契約化"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  implemented-by: []
  depends-on:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
    - "apply-publish-false-policy-to-workspace.md"
  blocks:
    - "verify-issue-1-repo-reflection.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# AGENTS.md 暫定原則の確定契約化

## Work

plan `PLAN-034IR3qOr15LGNY9mEindJ` のスライス T-AGENTS（Phase G-1〜G-7）。`AGENTS.md` の暫定原則節を approved 済み design ベースラインの確定契約（RT 契約・SIMD 構造仮説・依存方向 DAG 引用）へ差し替え、ワークスペース構成節を `publish = false` 適用後の現状に整合させる。最終文案は design が逐語確定していないため、plan Design Inputs が示す design 引用元節の範囲で執筆する（plan Open Questions / Gaps）。

- [ ] G-1. 「### 暫定原則（詳細契約は #1 で確定）」（L143–L147）を確定契約節へ差し替える: RT/オフラインの設計意図を design の確定契約（API・ライフサイクルで分離・`Processor`/`RtContext`・`prepare`/`process` 分離・RT パスは alloc/dealloc 禁止・保証範囲は API 単位で明示・未確定 API を RT-safe と断定しない・CI 成功は完全証明ではない・OS ハード RT 保証なし）へ置き換え、詳細は design「RT／非 RT の分離方式」「RT 契約」節を正本として参照する旨を記す
- [ ] G-2. 同節の SIMD 暫定原則を design の構造仮説へ更新する: スカラー参照実装必須・等価性契約（ビット一致または ulp 上限）・stable-only・`unsafe` 局所許可は現行規約と整合・SIMD 経路の採否と dispatch 層の `ruviform-math` 集約は要 PoC である旨（design「DSP／SIMD 設計」採否）
- [ ] G-3. 同節のクレート依存方向を design の DAG ルールへの引用へ更新する: 上位→下位のみ・循環依存禁止・基礎クレートから GUI・オーディオデバイス・GPU への強制依存を作らない。詳細な層内順序・adapter ルール・契約型所有は design「レイヤー構造と依存方向ルール」節が正本である旨を記す
- [ ] G-4. 「### ワークスペース構成」（L114–L115）を整合させる: `[workspace.package]` の列挙に `publish = false`（未決定クレート原則・licensing ADR）を加え、「最終的なクレート分割と公開 API は Issue #1 の詳細設計で確定する」を「クレート DAG・分割案は `docs/designs/ruviform-architecture-design-baseline.md` に記録され、`members` は実装 Phase に応じて段階追加される」現状に更新する（`members` 段階追加方針の根拠: design A.8「クレート追加計画」L533）
- [ ] G-5. 「### 暫定規約」の feature/unsafe 規約が現行のまま design（DSP/SIMD の unsafe 方針・feature 外部バックエンド分離クレート方針）と整合することを確認し、結果（変更/無変更と理由）を実施記録に残す
- [ ] G-6. 「### ライセンス注記」（L149–L151）が無変更であることを確認する（design: 維持・仮置き・未決定）
- [ ] G-7. `#1 で確定`・`#1 で決定`・`後続タスク` 等の陳腐化した参照が README/AGENTS に残っていないことを `git grep -n "#1 で確定\|#1 で決定\|後続タスク" -- README.md AGENTS.md` で確認する（S-1 ベースラインと比較）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/apply-issue-1-architecture-baseline-to-repo.md](../plans/apply-issue-1-architecture-baseline-to-repo.md) — `PLAN-034IR3qOr15LGNY9mEindJ`（approved）。Phase T-AGENTS / V-4・V-5 の正本
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。「レイヤー構造と依存方向ルール」（L111–L173）・「RT／非 RT の分離方式」（L272–L281）・「RT 契約」（L420–L425）・「DSP／SIMD 設計」採否（L355–L369）の引用元
- [../specs/architecture-design-baseline-deliverables.md](../specs/architecture-design-baseline-deliverables.md) — `SPEC-034INYy2aDLxse9dy8MgGZ`（approved）。Requirements B の確定制約と禁止記載の正本（V-5）
- [../adr/licensing-and-distribution-policy.md](../adr/licensing-and-distribution-policy.md) — `ADR-034IO02s3n3KHsxnHckpRn`（accepted）。`publish = false` 原則の根拠
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — `ADR-034IO00JjhxeTXUbCZtcDX`（accepted）。RT 契約の根拠

## Dependencies

- 前提タスク: `apply-publish-false-policy-to-workspace.md`（`TASK-034IRYXA0oq14nlgmC4Bch`）— G-4 のワークスペース構成節記述（`publish = false` の存在）が T-PUB の適用結果を前提とするため、本タスクは T-PUB 後に実行する
- 共有前提: plan S-1（変更前ベースライン記録）は pre-dispatch ステップとして実施済みであること
- 後続をブロック: `verify-issue-1-repo-reflection.md`（V-4・V-5 の内容監査）
- T-README とは相互独立だが、両方の完了が T-VERIFY の前提
- design の確定契約を超えた主張をしない（未確定 API を RT-safe と断定しない・CI 成功を完全証明扱いしない・OS ハード RT を保証しない）

## Affected Files

- `AGENTS.md` — 「暫定原則（詳細契約は #1 で確定）」節の確定契約化・「ワークスペース構成」節の publish/分割記述整合（section-edit。feature/unsafe 規約・ライセンス注記は現行維持）

## Done When

- [ ] 暫定原則節が design の確定契約（RT 契約・SIMD 構造仮説・DAG ルール引用）に差し替わっている
- [ ] ワークスペース構成節が `publish = false`・design ベースラインへの参照・`members` 段階追加の現状を記述している
- [ ] feature/unsafe 規約・ライセンス注記が現行のまま維持され、G-5/G-6 の確認結果が実施記録にある
- [ ] `#1 で確定`/`#1 で決定`/`後続タスク` の参照が README/AGENTS に残っていない（G-7）

## Verification

- [ ] `git grep -n "#1 で確定\|#1 で決定\|後続タスク" -- README.md AGENTS.md` が一致を返さない（G-7・S-1 ベースラインと比較）
- [ ] `grep -n "publish = false" AGENTS.md` と `grep -n "ruviform-architecture-design-baseline" AGENTS.md` が各々一致を返す
- [ ] Manual: design の RT 契約・SIMD 採否・DAG 節との突合（V-4）および spec Requirements B との整合・禁止記載なし（V-5）の目視監査
