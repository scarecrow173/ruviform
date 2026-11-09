---
id: "SPEC-034UCI2A3NOXf96MhNKdmi"
type: "spec"
status: "approved"
title: "Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/synvelis/issues/22"
    - "https://github.com/scarecrow173/synvelis/issues/23"
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
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/issue-257-driver-neutral-ui-contract.md"
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "docs/designs/issue-384-driver-neutral-gui-contract.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "docs/adr/issue-465-legacy-webview-retirement-status.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/adr/media-io-backend-abstraction.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/synvelis/issues/22"
    - "https://github.com/scarecrow173/synvelis/issues/23"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/181"
    - "https://github.com/scarecrow173/ruviform/issues/182"
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/189"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/191"
    - "https://github.com/scarecrow173/ruviform/issues/194"
    - "https://github.com/scarecrow173/ruviform/issues/195"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/197"
    - "https://github.com/scarecrow173/ruviform/issues/198"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/308"
    - "https://github.com/scarecrow173/ruviform/issues/309"
    - "https://github.com/scarecrow173/ruviform/issues/310"
    - "https://github.com/scarecrow173/ruviform/issues/311"
    - "https://github.com/scarecrow173/ruviform/issues/312"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/409"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/synvelis/issues/83"
    - "https://github.com/scarecrow173/synvelis/issues/104"
    - "https://github.com/scarecrow173/synvelis/issues/106"
    - "https://github.com/scarecrow173/synvelis/issues/107"
    - "https://github.com/scarecrow173/synvelis/issues/109"
    - "https://github.com/scarecrow173/synvelis/issues/112"
    - "https://github.com/scarecrow173/synvelis/issues/113"
    - "https://github.com/scarecrow173/synvelis/issues/114"
    - "https://github.com/scarecrow173/synvelis/issues/175"
    - "https://github.com/scarecrow173/synvelis/issues/176"
    - "https://github.com/scarecrow173/synvelis/issues/177"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/audio-lens/issues/141"
    - "https://github.com/scarecrow173/audio-lens/issues/142"
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  defers: []
  deferred-by: []
---

# Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 仕様

## What Should Be Built

Issue #193 本文で合意済みの「Migration Execution Plan v1.0 —
M0–M7・依存DAG・受入Gate・互換移行」を、`docs/` チェーンの
正本文書として記録する。対象は、既存 28 Workspace Crate と
Accepted ADR・既存 Consumer（Synvelis・AudioLens）の互換性を
維持しながら、Synvelis が必要とする汎用 Time / Typed Graph /
RT Ownership / Event / Plugin / Device / Media / Offline / UI /
Probe 機能を**検証可能な順序で**導入するための実行計画である。

具体的には次を canonical 文書化する。

- M0〜M7 の実行順・成果物・対応 Issue・開始/完了境界の表。
- 依存の意味（M0 契約/Fixture → M1 Graph IR → M2 Audio
  Runtime、M3 Time/Span → M4 Plugin Event/Process、独立可能な
  Media Reader/WAV・Mock Recording・UI）。
- 共通受入ゲート G0〜G7 と、全項目に必須の証拠リンク要件。
- Cross-Consumer（Synvelis / AudioLens）受入ゲート追加条件と
  Gate 別 Consumer 証拠表。
- 実装者向け前提判断 6 項目、Rollback / 保護対象 RB0〜RB6。
- Synvelis 側 E2E-L / E2E-R / E2E-X の受入接続。
- GUI Driver 移行 Lane（#384/#257/#262/#385〜#390 と両
  Consumer Issue）の Start / Integration / Acceptance DAG。
- Legacy WebView 撤去の独立 Lane（撤去 Owner = #465）。
- 進捗・Close 条件と、Issue #193 を計画対象・依存追跡の
  tracking Issue として維持する境界。

本 slice は docs のみの文書化であり、M0〜M7 の実装・PoC・
CI 変更・Consumer 実測・gate 受入そのものは対象外とする。

## Why Now

- Issue #193 は「実行計画として承認済み」の状態で Open を
  継続しており、M0〜M7 の計画対象と依存追跡の正本として
  機能している。実装・追跡の双方が参照する安定した
  `docs/` 内正本が必要。
- Target Architecture v0.7.1（umbrella 設計文書、approved）
  は設計権威であり、実行順・Gate 判定・Rollback・Consumer
  受入接続は #193 が実行権威である。両者の権威関係を
  文書上で固定する必要がある。
- GUI lane（#384 以降）と Legacy WebView 撤去 lane（#465）
  が 2026-11-07〜11-09 に Issue #193 本文へ追記され、
  M0〜M7 とは独立した先行関係の正本が本文中に存在する。
  これを構造化して記録しないと、Start/Integration/
  Acceptance の区別が失われる。
- Cross-Consumer 対等原則（Synvelis と AudioLens を対等な
  Consumer とする 2026-11-02 追補）と横断依存正規化 v1.1
  （Synvelis #23 正本）が本文に存在し、実装 Issue の着手
  判定に使われる。docs 正本化により参照が安定する。

## Users and Value

- **Ruviform maintainer / 将来の実装担当者**: M1〜M7 の各
  実装 Issue（#184〜#198 等）の着手条件・依存・Gate 証拠
  要件を、Issue #193 本文を都度読み直さずに docs 正本から
  参照できる。
- **Synvelis / AudioLens の Consumer 担当者**: Cross-
  Consumer 受入ゲートの Gate 別証拠要件と、Consumer 側
  Issue との接続面（E2E-L/E2E-R/E2E-X、GUI lane）を
  同一の計画正本から確認できる。
- **doc-driven 開発を行うエージェント**: spec → design →
  plan → task → test-spec の canonical チェーンとして
  追跡可能になり、後続の docs メンテナンス（index 再生成・
  audit）に載る。

## Scope

### In Scope

- Issue #193 本文（2026-11-09 時点の全文）のうち、
  Migration Execution Plan v1.0・M0〜M7・依存 DAG・
  受入 Gate G0〜G7・Cross-Consumer 受入追加条件・
  Rollback RB0〜RB6・GUI lane・WebView 撤去 lane・
  横断依存正規化・Signal Flow 所有境界・進捗/Close 条件・
  受入条件を docs チェーンへ構造化して記録すること。
- 参照正本（umbrella 設計文書・#194 M0 audit・#219 spec・
  領域別 issue-* 文書・関連 ADR）への relations による
  接続。
- 「設計合意・実装完了・CI 検証・Platform 検証・移行受入」
  の状態分離と、未検証を合格に昇格させない記録方針の明記。

### Out of Scope

- M0〜M7 の実装・PoC・CI 変更・Consumer repo の変更
  （#194〜#198・#219・#294・各 Consumer Issue の Owner
  領域）。
- Issue #193 本文の改訂、umbrella 設計文書
  （`synvelis-driven-target-architecture-and-migration.md`）
  本文の改訂。
- GPUI 採否の最終判定（#262/#419）、Legacy WebView の
  実撤去（#465 の実施）、各 gate の正式 Close 判定。
- 実装コード・テスト・`Cargo.*`・`mise.toml`・`.github/`
  の変更（本 slice は docs のみ）。

## Requirements

- R-1: Issue #193 本文の受入条件（GUI lane の 4 項目と
  DoD）を 1 行も欠落なく設計文書へ対応付ける。
- R-2: M0〜M7 の実行順表（区分・成果物/判断・対応先・
  開始/完了境界）と依存の意味を、本文の区分・優先度
  （P0/P1/P2）・Issue 対応を保ったまま記録する。
- R-3: G0〜G7 の各ゲートについて、判定条件と「すべて証拠
  リンク必須」の要件を維持する。
- R-4: GUI lane・WebView 撤去 lane について、Start /
  Integration / Acceptance の 3 区分を分離して記録し、
  Issue 着手を #262 採用完了へ依存させる循環を禁止事項
  として明記する。
- R-5: 暗黙の Issue 依存を新設しないこと、Owner 表を
  実装順序の正本として維持すること、Driver/Component
  二重実装と Cross Consumer CI 重複作成の禁止を、
  受入条件どおり記録する。
- R-6: Cross-Consumer 受入ゲートの Gate 別証拠表
  （Synvelis / AudioLens の守る契約）と運用判定
  （impacted / no impact / unknown、証拠記録、固定 SHA
  凍結をしない方針、#219 Owner 境界）を記録する。
- R-7: 実装者向け前提判断 6 項目・Rollback RB0〜RB6・
  Synvelis E2E-L/E2E-R/E2E-X 接続・進捗/Close 条件・
  確定 Issue traceability を記録する。
- R-8: 実装ステータス同期（2026-11-04）の状態分類
  （Implemented / Tested headless / Cross-Consumer
  Verified / Platform Verified / Migration Accepted）と
  M0〜M7 進捗表を snapshot として記録し、Closed な旧
  Issue を新能力の実装完了証拠としない原則を維持する。
- R-9: 参照は merge 済みの正本・文書のみに限定し、
  別 Worker が並行作業中の未 merge 文書を参照しない。
- R-10: 本 slice の変更は `docs/` の新規 `issue-193-*`
  文書と生成 index のみとする。

## Acceptance Criteria

- [ ] spec / design / plan / task / test-spec の
  `issue-193-migration-execution-plan.md` 5 文書が
  `docs/` チェーンに存在し、Issue #193 本文の内容を
  権威関係を保って構造化している。
- [ ] Issue #193 本文の受入条件 4 項目（#384 後継 ADR の
  Evidence 承認・旧 WebView/Frontend 削除の明示判断・
  WebView Code の一括破棄禁止 / 暗黙 Issue 依存の新設
  禁止 / Owner 分界と二重実装・CI 重複禁止 / DAG と
  Owner 表の正本維持と両 Consumer 最新 Issue への
  リンク）が設計文書に対応行として存在する。
- [ ] M0〜M7 表・G0〜G7・RB0〜RB6・GUI lane・WebView
  撤去 lane・Cross-Consumer 受入表が設計文書に存在し、
  各項目が Issue 番号または正本 docs パスへ辿れる。
- [ ] `docs/` 以外（`crates/`・`Cargo.*`・`mise.toml`・
  `.github/`・既存他 Issue 文書）に差分がない。
- [ ] audit_docs で新規文書の error が 0、docs index が
  再生成済み、`mise run fmt-check` が pass する。

## Deferred Design Concerns

- M0〜M7 の実装・各 gate の正式受入・Consumer 実測は
  #193・#219/#281/#288/#294・各 Consumer Issue の Owner
  領域であり、本 slice では `not run`/`NotRun`/
  `Unsupported` として記録する（意図的な分離であり、
  解消しない gap ではない）。
- GPUI 採否の最終判定（#262 の Evidence による #419）と
  Legacy WebView の実撤去（#465）は将来の受入段階の
  要件。再着手トリガーは対応 Issue の Gate 証拠。
- umbrella 設計文書本文への GUI lane・ADR 変更の反映は
  本 slice の対象外（approved の設計権威として維持。
  改訂要否は別 Issue/Owner 判断）。
- Issue #193 本文は進行中の tracking 正本であり今後も
  追記され得る。本書の記述は 2026-11-09 時点の snapshot
  であり、本文更新時は docs 正本の同期更新が必要。

## Sources and Relations

- 受入正本: [Issue #193](https://github.com/scarecrow173/ruviform/issues/193)
  本文（2026-11-09 時点）。
- 設計権威: [umbrella 設計文書](../designs/synvelis-driven-target-architecture-and-migration.md)
  （`docs/designs/synvelis-driven-target-architecture-and-
  migration.md`、approved、Issue #192 由来 Target
  Architecture v0.7.1）。
- Consumer 実装依存の正本: Synvelis #23 正規化 DAG、
  Synvelis #22 全体設計。
- M0 監査正本: [docs/discovery/issue-194-m0-baseline-audit.md](../discovery/issue-194-m0-baseline-audit.md)。
- Cross-Consumer 互換契約: [docs/specs/issue-219-consumer-compat-contract.md](../specs/issue-219-consumer-compat-contract.md)
  （proposed）。
- GUI lane 関連正本: [docs/specs/issue-384-driver-neutral-gui-architecture.md](../specs/issue-384-driver-neutral-gui-architecture.md)、
  [docs/designs/issue-384-driver-neutral-gui-contract.md](../designs/issue-384-driver-neutral-gui-contract.md)、
  [docs/designs/issue-257-driver-neutral-ui-contract.md](../designs/issue-257-driver-neutral-ui-contract.md)、
  ADR `driver-neutral-gui-gpui-driver-webview-first-issue-384`、
  `issue-465-legacy-webview-retirement-status`。
- 領域別実装 Issue: #181/#182/#184〜#198・#219・#294・
  #308/#309・#310〜#314・#384〜#390・#409・#419・#465、
  Synvelis #22/#23/#28/#83/#104/#106/#107/#109/#112〜#114/
  #175〜#177、AudioLens #127/#141/#142/#166。
