---
id: "DESIGN-034V2ho95TYYvIaJsGmRSx"
type: "design"
status: "approved"
title: "Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures の設計"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/426"
    - "https://github.com/scarecrow173/ruviform/issues/464"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-314-semantic-oracle.md"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-audiolens.json"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-synvelis.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-314-semantic-oracle.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-314-golden-evidence.md"
    - "docs/designs/issue-426-gpui-lab-binding.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-314-semantic-oracle.md"
    - "docs/designs/issue-314-golden-evidence.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-314-semantic-oracle.md"
    - "docs/specs/issue-314-golden-evidence.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-314-semantic-oracle.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures の設計

## Context

Issue #314 は semantic oracle（Source/Viewport/時間軸/Selection/
NaN gap/partial/stale/LOD/Revision・Geometry/annotation 位置）を
「実装と独立した Fixture から検証」することを要求する。slice 1〜5
で Golden Utility・Scenario 接続・実 Window capture・証拠 lane は
揃ったが、consumer 形状の source 値を固定する semantic oracle
fixture は driver-lab parity set に存在しなかった。

本 slice は新規の検査機構を作らず、既存の 2 つの seam に fixture
を接続するだけで成立する:

- Fake lane: `ruviform-ui-test/tests/driver_parity.rs` —
  `ViewSpec` JSON を `view_spec()` で parse し、`ViewHost::mount`
  前後の payload と mount 受理 key を検査する既存 parity 構造。
- GPUI lane: `ruviform-ui-gpui/tests/consumer_observation.rs` —
  `GpuiObserver::observe`/`drawn`（公開 seam、gpui type 非露出）
  で mounted projection と `DrawnPlan` を検査する既存構造。

## Scope

### In Scope

- `fixtures/driver-lab/parity/viz-semantic-{audiolens,synvelis}.json`
  新設（`rvf.driver-lab.parity/v1` manifest）。
- `driver_parity.rs` への Fake payload 保持 test 2 件追加。
- `consumer_observation.rs` への drawn-plan 一致 test 2 件追加。
- 本 slice の文書チェーン。

### Out of Scope

- spec の Out of Scope と同一（pixel golden 接続・実 capture・
  OS input・stale/LOD/Revision oracle・NaN 個別値・consumer theme
  infra・新規依存・公開 API 変更）。

## Components and Boundaries

```text
crates/ruviform-ui-test/fixtures/driver-lab/parity/
    viz-semantic-audiolens.json   — AudioLens 形状 oracle（新規）
    viz-semantic-synvelis.json    — Synvelis 形状 oracle（新規）
crates/ruviform-ui-test/tests/driver_parity.rs
    parity_viz_semantic_audiolens_payload_retained_fake    （新規）
    parity_viz_semantic_synvelis_payload_retained_fake     （新規）
crates/ruviform-ui-gpui/tests/consumer_observation.rs
    observes_viz_semantic_audiolens_oracle_through_seam    （新規）
    observes_viz_semantic_synvelis_oracle_through_seam     （新規）
```

依存方向: fixture/test → `ruviform-ui`（`ViewSpec`/`ComponentSpec`/
`ViewHost`）と `ruviform-ui-gpui`（`GpuiObserver`）。共有層に
consumer 固有コードは追加しない — fixture は JSON データであり、
判定ロジックは既存 test helper（`view_spec`/`fixture`/
`mount_parity`/`drawn_plan`/`assert_near`）を再利用する。

### Fixture schema（`rvf.driver-lab.parity/v1`）

既存 parity fixture（`new-kinds.json` 等）と同じ envelope:

| field | 内容 |
| --- | --- |
| `schemaVersion` | `1` |
| `manifest` | `"rvf.driver-lab.parity/v1"` |
| `issue` | Issue #314 URL |
| `description` | lane 境界・NaN 表現・serde_json 制約・非対象領域の記述（日本語） |
| `view` | mount 可能な `ViewSpec` JSON（`view_spec()` が parse） |
| `expect` | 検査期待値（下表） |

`expect` の構造は fixture ごとに component 構成へ合わせる:

| key | Fake 側の検査 | GPUI 側の検査 |
| --- | --- | --- |
| `mountedKeys` | `host.mount` の `ProjectionOp::Mounted` に全 key が含まれる | `observe().components` に全 key が `mounted` で存在 |
| `waveform.declaredBins` / `gapBins` / `drawnChannels` | channel_bins 総数・`count==0` 数・paint 対象 bin の `(start,end)` 列 | `DrawnPlan::Waveform.channels[].bins` が `drawnChannels` と一致 |
| `timeAxis.declaredTicks` / `drawnMarks` | ticks 総数・viewport 外 tick の温存 | `DrawnPlan::TimeAxis.marks` が `drawnMarks`（x/major/label）と一致 |
| `seek.committedPlain` / `committedNormalized` | `host.observe("seek")` snapshot の committed 値が一致 | `observe` snapshot の committed 値が一致 |
| `meter.declaredBins` / `gapBins` / `drawnBars` | channel_levels 総数・`count==0` 数・over-range/partial 値の verbatim 保持 | `DrawnPlan::Meter.bars` が `drawnBars`（channel/x0/x1/peak/rms）と一致 |
| `spectrogram.declaredColumns` / `declaredCells` / `gapColumns` / `drawnCells` | columns/cells 総数・`count==0` column 数・`magnitude_range` 外値の verbatim 保持 | `DrawnPlan::Spectrogram.cells` が `drawnCells`（x0/x1/y0/y1/intensity）と一致 |

### Payload 保持 vs 描画計画の非対称（重要）

Fake lane が検査するのは **mount 前 payload の verbatim 保持** で
あり、描画計画ではない。renderer の clamp/cull は payload を
書き換えない — over-range peak/rms・`magnitude_range` 外
magnitude・gap bin・viewport 外 tick は source truth として残り、
clamp/cull は `DrawnPlan` 生成時の責務である。

`DrawnPlan` の viewport 挙動は component ごとに非対称であり、
fixture の `expect.*.drawn*` は現行 contract の正直な記述である:

- **Waveform**: paint される bin の frame range を全て報告し
  viewport cull を行わない（viewport 外 bin は bounds 外 quad
  として clip される）。
- **Meter/Spectrogram/TimeAxis**: viewport 外・非重なりの
  bin/column/tick を cull し、部分重なりは viewport 端に clamp
  する。over-range amplitude/magnitude は `[0,1]`/`magnitude_range`
  へ clamp して plan に載せる。

この非対称を fixture の `description` に明記し、「plan が
source を再解釈しない」ことと「plan が描画可能な範囲に正規化
する」ことを混同しない。

### serde_json 丸め制約（実測済み）

serde_json 1.0.154 は一部の 17 桁小数を `str::parse::<f64>`
（正確丸め）と異なる bit pattern に parse する（1 ulp 誤丸め、
実測）。bit-exact oracle の小数は **両 parser が一致する値**
に限定する — `-0.1234567890123456`/`0.9876543210987654`/
`0.1111111111111111` 等は一致を確認済み。fixture description と
test コメントにこの制約を記録し、将来 fixture を拡張する者が
「JSON の文字列表現 = f64 bit」という誤前提を置かないように
する。

### NaN の表現

JSON は NaN/Infinity を表現できない。NaN-gap/NaN-statistics の
oracle は `count==0` の gap bin/column で表し、個別 non-finite
値の描画挙動（non-finite peak skip・non-finite rms → `rms=None`・
non-finite magnitude skip）は `render.rs`/`view.rs` の unit test
が担う — 本 fixture は JSON で表現可能な Source/Viewport/
時間軸/partial/Geometry の drawn-plan oracle に限定する。

### Lane 分離の維持

- semantic oracle（payload 保持）: Fake lane の `driver_parity`
  test が担う。
- draw plan（`DrawnPlan` 一致）: GPUI lane の
  `consumer_observation` test が担う。plan pass は pixel pass
  ではない（`DrawnPlan` は `#[non_exhaustive]` の plan データで
  あり screenshot ではない）。
- pixel golden: slice 3/4/5 の `golden_capture.rs`/`BaselineStore`
  lane が担う。本 slice は触れない。
- OS input: #313 lane が担う。本 slice は触れない。

## Data and Control Flow

```text
fixture JSON (view + expect)
  │
  ├─ Fake lane: view_spec(&fx) → ViewSpec
  │     ├─ payload 検査（parse 直後、mount 前）
  │     │     f64 to_bits / gap・partial 集計 / over-range 値
  │     │     / viewport 外 tick 温存
  │     └─ ViewHost::mount(spec) → ProjectionOp::Mounted keys
  │           + observe(Seek).committed plain/normalized
  │
  └─ GPUI lane: fixture() → GpuiObserver::new + mount_parity
        ├─ observe(window) → components[].key/snapshot.mounted
        │                    + Seek committed 値
        └─ drawn(window) → DrawnPlan::{Waveform,Meter,
              Spectrogram,TimeAxis} が expect.*.drawn* と一致
```

## Non-Functional Constraints

- 非 RT only（既存 parity/observation test と同じ）。
- 新規依存なし — `serde_json`/`std` のみ（両 crate の dev 依存に
  既存）。
- `unsafe` なし。
- 公開 signature 変更なし — test/fixture のみの additive 変更。
- fixture は consumer-*shaped* であり consumer 製品コード・
  theme infra の複製を含まない。

## Risks and Trade-offs

| リスク | 影響 | 対策 |
| --- | --- | --- |
| `expect.*.drawn*` が実装詳細（plan 生成手順）に密結合し、正当な plan 変更で test が壊れる | plan contract 変更のたびに fixture 更新が必要 | `DrawnPlan` は `#[non_exhaustive]` の公開 contract であり、fixture はその contract 面（bin range・mark 座標・bar 座標・cell 座標・intensity）のみを固定する。plan 内部実装の変更は test を壊さない |
| serde_json の丸め差で bit-exact 検査が環境依存になる | 別 serde_json 版で test が fail | 制約を fixture description/test コメントに明記。依存版更新時は両 parser の一致を再実測する（`tmp` 検証で実施済みの手順をコメントに残す） |
| Waveform の viewport 非 cull を「正しい挙動」として固定してしまう | 現行の非対称が将来の意図的変更を阻害 | fixture description に「現行 contract の正直な記述」と明記。contract 変更時は fixture 更新が review 対象になる |
| fixture が consumer 製品 fixture の複製と見なされる | scope 越境・二重管理 | fixture は `ViewSpec` JSON のみで theme/product/RTC 接続を持たず、consumer-*shaped*（形状のみ再現）であることを description に明記 |
| NaN を `count==0` で代用することの限界 | NaN 個別値の oracle 欠落 | description に「JSON 表現不能・unit test が担う」と明記し、coverage 境界を固定 |

## D1: fixture は `rvf.driver-lab.parity/v1` の additive 追加

新規 manifest・新規 schema は作らず、既存 parity fixture envelope
（`schemaVersion`/`manifest`/`issue`/`description`/`view`/`expect`）
を再利用する。`expect` の key 名は lane 別検査を素直に表す
（`drawnChannels`/`drawnMarks`/`drawnBars`/`drawnCells` は
`DrawnPlan` variant 名に対応）。

## D2: 検査は payload 保持と plan 一致の 2 段

Fake lane は **mount 前 payload**（parse 直後の `ComponentSpec`）
を検査し、mount 後の semantic 投影ではなく source truth の保持を
固定する。GPUI lane は **公開 seam の観測結果**（`observe` +
`drawn`）を検査する。両者が同一 `expect` を読むことで
「payload が真実を保持し、plan がそれを正しく描画計画化する」
2 段の oracle になる — Fake の payload 検査を plan 比較と混同
しない。

## D3: bit-exact は `to_bits` 比較

f64 の等価判定は `==` ではなく `to_bits()` で行う
（`clippy::float_cmp` 回避であると同時に、-0.0/NaN bit 差を
排除しない正直な bit 一致を固定する）。fixture の小数選定は
serde_json/`str::parse` の一致を実測して決める。

## D4: coverage 境界は fixture description に固定

stale（spec 更新後の再投影）・selection・LOD・Revision・
NativeSurface・NaN 個別値は既存 lane の責務であり、本 fixture
が担わないことを `description` に明記する。fixture が
「全 oracle を網羅した」と誤読されない構造にする。

## Verification Notes

- [ ] `mise run fmt-check` / `check` / `lint` / `test`
- [ ] `mise run test-ui-parity`（Fake 側新規 test 含む）
- [ ] `mise run test-ui-gpui`（gpui-test、`consumer_observation`
      含む）
- [ ] `mise run test-ui-scenario-contract` / `test-ui-golden` /
      `test-ui-driver-contract` / `check-ui-driver-boundary`
      （非回帰）
- [ ] `mise run test-rt`（非回帰）
- [ ] docs index 再生成（`doc_maintenance`）
