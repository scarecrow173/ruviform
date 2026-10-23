---
id: "ADR-R2GFzIbod68oHGdQZTuyJW"
type: "adr"
status: "accepted"
title: "第 2 ターゲット NLE 編集モデル"
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
    - "docs/designs/ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/second-target-design.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/second-target-design.md"
    - "docs/specs/edit-data-model.md"
    - "docs/designs/edit-data-model.md"
    - "docs/specs/composite-transition-kernels.md"
    - "docs/designs/composite-transition-kernels.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 第 2 ターゲット NLE 編集モデル

## Context and Problem Statement

ベースラインは第 2 ターゲットを「NLE／映像制作級（動画再生・同期 → 編集・コンポジット）」と定義するが、編集範囲の具体化は保留だった。基盤として何を持つか — 破壊編集 vs 非破壊編集、タイムラインデータモデルの責務所在、既存エンジンとの関係 — を確定する。

## Considered Options

### D1. 編集モデル

- D1-a 非破壊編集モデル（タイムライン/クリップ/トラック + ソース参照 + in/out — 評価時にエンジンへ射影）
- D1-b 破壊レンダリングモデル（編集決定を逐次バッファへ焼き付け）
- D1-c 両対応抽象（edit decision list + 直接描画の二系統）

### D2. コンポジット/遷移の責務所在

- D2-a `ruviform-video-proc` に RT カーネルとして追加（blend/transform/transition は既存色変換カーネルと同層）
- D2-b 専用 compositor クレート新設（即時）
- D2-c アプリ層任せ（基盤は提供しない）

### D3. 編集評価の実行モデル

- D3-a 既存 pull グラフ/pipeline へタイムライン位置から射影（engine 再利用）
- D3-b 専用編集実行エンジン新設

## Decision Outcome

- **D1 — D1-a 採用。** NLE 級の本質要件は非破壊性（再編集・undo・メタ編集）であり、クリップ列 = ソース参照 + in/out + トラック属性のデータモデルを基盤の責務とする。破壊レンダリングは「オフライン書き出し」として既存 `ruviform-exec` で表現可能であり、モデルとして持たない（YAGNI — D1-c）。
- **D2 — D2-a 採用。** blend（alpha/mix）・アフィン変換・遷移はピクセルカーネルであり `ruviform-video-proc` の RT カーネル群と同層・同 RT 契約。専用クレートはカーネル種別数が増えた段階で分割判断（後続 Issue へ延期）。
- **D3 — D3-a 採用。** 編集評価 = タイムライン位置 → 各トラックのソース pull → 合成の射影であり、audio グラフ/video pipeline の既存 pull モデルと同型。専用エンジンは不要。

### Consequences

- 新設責務: 非破壊編集データモデル + 評価射影（非 RT クレート `ruviform-edit` 相当 — 命名は実装 Issue で確定）→ media-core/audio-core/video-core 依存。
- タイムライン再生は既存 engine のまま（engine 非侵襲）。
- コンポジットカーネルは video-proc へ段階追加。

## Verification

- design が ADR の責務分割を具体化し、baseline の 3 分類（決定）へ記録。
