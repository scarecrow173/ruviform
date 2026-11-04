---
id: "SPEC-P7W4Dz6p2EQIQNPclAQG2U"
type: "spec"
status: "approved"
title: "Issue #307 PluginNode attach 時 frame_base の非 RT 型付き検証（verified clock context）"
created: "2026-11-04"
updated: "2026-11-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/307"
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
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/designs/issue-186-param-control-events.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-186-param-control-events.md"
    - "docs/specs/issue-188-timed-midi.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #307 PluginNode attach 時 frame_base の非 RT 型付き検証

## Summary

`attach()` で確定した `AttachConfig.frame_base: TimeBase` を、非 RT 側
（`PluginNodeControl`）から**同一の検証済み context** として取得・消費できる
ようにする。外部 `frame_base` を併記して「AttachConfig と等しいはず」と
前提する consumer 契約を型付き検証へ置き換え、異値で構築された
`TransportPlan` / Dispatcher が node へ届く前に `FrameBaseMismatch` 系の
typed error を非 RT 側へ返す。変更は `ruviform-audio-plugin-bridge` 内で
完結し、RT callback 経路・media-core scheduler・既存の
Epoch/Generation 契約には触れない。

## Why Now

- Synvelis PC-C4（synvelis#101）が `ParamDispatch::new(control, node,
  frame_base)` で外部 `frame_base` を受け取る設計を進めており、現行
  bridge には照合アクセサがない（consumer 側コメントに「等値必須だが
  bridge に照合手段なし」と明記済み）。
- `TransportScheduler::install_plan` は `plan.frame_base` と scheduler の
  `frame_base` を比較しない。`TransportPlanBuilder::new` は公開 API で、
  consumer は任意 base で plan をコンパイルできる — 別 base の plan が
  install されると `position_ticks` が scheduler の frame 空間で誤読され、
  TimedParamEvent・Seek・Plan/Receipt 整合・Offline 精度を破る。
- P0: 型付き Plugin Binding 導入前に Ruviform 側の時刻 context 正本を
  確定させる必要がある（Issue #307 ラベル BRIDGE-CLOCK-01）。

## Users and Value

- **Synvelis（DAW consumer）:** `ParamDispatch` 等の構築時に bridge が
  発行した検証済み `TimeBase` と照合でき、誤値を型付き拒否できる。
- **AudioLens / generic consumer:** 同じ control handle 契約を共有し、
  attach 時 context の単一起源を信頼できる。
- **bridge 保守者:** 「frame_base の正本は AttachConfig」の不変条件が
  API レベルで強制され、下流での暗黙補正が不要になる。

## Scope

### In Scope

- `PluginNodeControl` への attach 時 `frame_base` 保持と読み取り専用
  アクセサ（`frame_base()`）＋ 異値検証 API（`check_frame_base`）。
- `PluginNode` 側の同一値アクセサ（`frame_base()` — scheduler 委譲）で、
  両半分の公開 context が同一 `AttachConfig` 起源であることを保証。
- `install_plan` の事前検証: `plan.frame_base` が attach 時 base と
  異なる plan を stage 前に typed error で拒否（非破壊）。戻り値型は
  `InstallPlanError`（`QueueFull`/`FrameBaseMismatch`）へ明示移行。
- 新規公開型 `FrameBaseMismatch` / `InstallPlanError`（`#[non_exhaustive]`
  規約準拠）。
- `attach`/`PluginNodeControl` 既存利用箇所の調査と移行（本 repo 内）。
- 検証: attach base の往復一致・異値拒否の非破壊性・
  Epoch/DeviceRestart/Seek/Loop/PlanGeneration と frame_base の非混同・
  既存 RT/Offline 絶対 frame 配送の非回帰。

### Out of Scope

- 実 VST3 backend（Issue #253）。Synvelis 側 Consumer 導入（synvelis#101）。
  DAW 固有 Automation/`ProjectRevision`/`BeatPoint`/`AutomationMode`。
- `TransportScheduler::install_plan` 内部への base 照合（media-core 変更は
  本 Issue の「plugin-bridge 内で完結」境界の外。bridge の control seam で
  全経路を塞ぐため不要 — design に記録）。
- `AttachConfig.frame_base` と `process_config.sample_rate` の整合検証
  （engine tick の意味論は consumer 宣言;新規ポリシーは別 Issue）。
- `ParamDispatchLedger` 等非 RT 投影 helper の base 照合（install 境界で
  混入経路が閉じるため不要。詳細は design の境界決定に記録）。

## Requirements

- R1: `PluginNodeControl::frame_base() -> TimeBase` は attach 時に
  `AttachConfig.frame_base` へ渡した値と**同一**を返す（Copy、読み取り
  専用、非 RT）。
- R2: `PluginNode::frame_base() -> TimeBase` は内蔵 scheduler の
  `frame_base()` を返す（RT-suitable な read-only）。node 構築は
  `attach` 経由のみのため、R1 と常に一致する。
- R3: `PluginNodeControl::check_frame_base(base) -> Result<(),
  FrameBaseMismatch>` は `base == frame_base()` を検証し、異値では
  `{expected, actual}` を保持した typed error を返す。Dispatcher 構築等の
  consumer 側コンストラクタがそのまま `?` で伝播できる形。
- R4: `install_plan` は `plan.frame_base != frame_base()` の plan を
  `InstallPlanError::FrameBaseMismatch` で stage 前に拒否する。queue push
  未実施・`control_overflows` 非加算・旧 plan/graph/node 状態は非破壊。
  `TimeBase` の比較は gcd 正規化済み実質同値（`PartialEq`）。
- R5: `install_plan` の queue 満杯失敗は `InstallPlanError::QueueFull`
  として従来意味（drop + `control_overflows` bump）を維持する。
- R6: `frame_base` は `SetClock`/`DeviceRestart`/`Seek`/loop 更新/
  `PlanGeneration` 進行で変化しない。Clock Epoch の変化と frame base を
  同一視しない（別 identity）。
- R7: RT callback 経路に Mutex/Allocation/Blocking を追加しない。
  metadata 保持・事前検証は attach/非 RT のみ。
- R8: 暗黙の rate conversion・時刻 scale 変更を導入しない。異値は成功に
  変換せず必ず明示拒否。

## Acceptance Criteria

- AC-1（正常）: `attach` 時 `TimeBase`（例: 1/48000）を
  `control.frame_base()` / `node.frame_base()` から取得でき、同値で構築した
  TimedParamEvent plan が RT process で絶対 frame に一致して届く。
- AC-2（異値）: 別 `TimeBase` で構築した plan の `install_plan` が
  `InstallPlanError::FrameBaseMismatch{expected,actual}` で拒否され、
  report queue に何も流れず、後続の正しい plan install が通常通り適用される
  （旧 Plan/Graph/Node 非破壊）。
- AC-3（Dispatcher 構築拒否）: `check_frame_base(foreign)` が
  `Err(FrameBaseMismatch)` を返し consumer 側 ctor の型付き拒否に使える。
- AC-4（非混同）: `device_restart` 後も `frame_base()` は attach 値を維持し、
  epoch 変化と混同しない。未知/不一致を成功に変えない。
- AC-5（互換調査）: repo 内の `attach`/`PluginNodeControl` 全利用箇所が
  新シグネチャでビルド・テストを通過する。`install_plan` 戻り値型の変更は
  明示移行として報告。
- AC-6（Cross-Consumer 材料）: AudioLens/Synvelis への影響分類材料を
  記録（実機/実 Plugin 未実測の部分は Not Run と明記）。Consumer 側
  ビルド/テストの実施は #219/#294 lane 運用側。

## Dependencies

- `docs/specs/issue-190-transport-span-scheduler.md`（
  `frame_base`/`PlanGeneration`/receipt 契約の正本）
- `docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md`
  （bridge/node/control の構造と RT 契約）
- `docs/designs/issue-186-param-control-events.md`（TimedParamEvent →
  plan 構築の consumer 経路）

## Risks

- `install_plan` の戻り値型変更は `publish = false` 下での明示移行。
  in-repo 呼出は全て `.unwrap()` 経由で破壊的影響は限定されるが、
  Synvelis 側が仮に `QueueFull` を match していた場合の移行コストを
  consumer impact として報告する。
- `#[non_exhaustive]` enum 追加で将来 variant 追加時に downstream match が
  default arm を要求される（規約どおり; 仕様化済み）。

## Open Questions

- なし（API 形態の候補比較と採用理由は design doc 側で確定・記録する）。
