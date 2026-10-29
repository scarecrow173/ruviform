---
id: "ADR-Lhe1Psi2bhfA35B3FABi6q"
type: "adr"
status: "accepted"
title: "OFX 等映像プラグイン規格の対応範囲"
created: "2026-11-03"
updated: "2026-11-03"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/second-target-design.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/adr/plugin-isolation-strategy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by:
    - "ofx-host-video-contract-and-c-abi.md"
  derives-from:
    - "docs/specs/second-target-design.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "ofx-host-video-contract-and-c-abi.md"
    - "../specs/ofx-host-adapter.md"
    - "../discovery/issue-118-video-l1.md"
    - "../designs/issue-118-video-l1.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# OFX 等映像プラグイン規格の対応範囲

## Context and Problem Statement

ベースラインは映像プラグイン規格として「OFX が有力候補。範囲・採否は別途設計（保留）」と記録。プラグイン基盤は規格中立コア + 規格アダプタ分離（plugin ADR・#29 実装済み）、分離は L0 インプロセス既定（#31 確定）が既決。OFX 採否と対応範囲を確定する。

## Considered Options

### D1. 採用規格

- D1-a OFX（OpenFX — C API・image effect 中心の業界標準）
- D1-b 独自規格のみ（外部規格非対応）
- D1-c OFX + 他映像規格（Fuse/EDIUS 系等）の複数対応

### D2. OFX 対応範囲

- D2-a Image Effect API のホスト側のみ（suite: imageEffect/property/parameter）
- D2-b ホスト+authoring 両側・全 suite（Generator/Transition/Retimer 等）

### D3. 実装方式

- D3-a plugin-core 境界に揃えた規格アダプタクレート（`ruviform-plugin-ofx` 相当）— CLAP/VST3 と同構造
- D3-b 独立した映像専用プラグイン基盤の別設計

## Decision Outcome

- **D1 — D1-a 採用。** OFX は NLE/映像制作系ホスト間で実装上の標準であり、第 2 ターゲットの相互運用要件に整合。独自規格のみでは外部資産を取り込めない。複数規格対応は需要未確定（YAGNI — 追加規格は構造上追加可能）。
- **D2 — D2-a 採用。** 初期範囲 = Image Effect API のホスト側（imageEffect/property/parameter suite の実用最小）。authoring 側・追加 suite・GPU suite（OpenCL/CUDA/Metal 系）は需要発生時の後続 Issue。
- **D3 — D3-a 採用。** plugin-core の `PluginProcessor`/ライフサイクル/分離設計（L0 既定）に揃える規格アダプタとして実装。bases: `clack`/`vst3` と同一の「規格中立コア + 薄アダプタ」パターン。binding 方式（自前 C FFI vs 既存 crate）は実装 Issue の依存追加規約に委任。

### Consequences

- `ruviform-plugin-ofx`（仮）が plugin 拡張層の第 3 アダプタとして後続 Issue で実装される。
- OFX のピクセル平面/時間/色モデルは video-core `VideoFrame`/`ColorInfo` へ写像 — 写像規則は design で定義。
- GUI（OFX は overlay/parameter UI の規約が弱い）: 汎用パラメータ UI 生成で代替（既存方針と同一）。

## Verification

- 実装 Issue 起票時に plugin-core 境界への写像テスト（#74 同型）を受入条件へ。
