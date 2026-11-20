---
id: "ADR-q6Mv3tYn8Lb5xKp2cJd9Rw"
type: "adr"
status: "accepted"
title: "issue-631-gpu-field EnumDisplayDevicesW 採用・DXGI 不採用の根拠"
created: "2026-11-20"
updated: "2026-11-20"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-631-gpu-field.md"
    - "../designs/issue-631-gpu-field.md"
    - "../discovery/issue-631-gpu-field.md"
    - "../adr/issue-631-evidence-fields.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-gpu-field.md"
    - "docs/designs/issue-631-gpu-field.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# issue-631-gpu-field EnumDisplayDevicesW 採用・DXGI 不採用の根拠

## Status

accepted（2026-11-20）

## Context

Issue #631 AC4 'OS and GPU' の GPU 側記録をどの API で充足するか
が判断点だった。前 slice（PR #764）の Spec は「GPU 情報は
manifest/API gate で別件残件」と記していた。候補は:

1. **GDI `EnumDisplayDevicesW`** — `idevnum` 列挙で
   `DISPLAY_DEVICEW` の `DeviceString`/`DeviceID`/`DeviceKey`
   を取得し、`EDD_GET_DEVICE_INTERFACE_NAME` 第2段で monitor
   対応付け。既存有効 feature（`Win32_Graphics_Gdi` /
   `Win32_UI_WindowsAndMessaging`）のみで実現可能。
2. **DXGI adapter 列挙** — `IDXGIFactory::EnumAdapters` で実
   renderer adapter を特定できるが、`windows-sys` の
   `Win32_Graphics_Dxgi` feature 追加（manifest 変更）が必要。
3. **GPUI renderer-info API** — GPUI 内部の renderer/adapter
   情報を production crate 側に新規公開する案。API 追加が必要。

## Decision

**`EnumDisplayDevicesW` を採用する**（Coordinator 決定に従う）。

- AC4 の要求は 'OS and GPU' の環境 evidence 記録であり、OS が
  認識する display-adapter census（`DeviceString` = GPU 製品名・
  `DeviceID` = PnP ID・`DeviceKey` = registry key・StateFlags・
  monitor 対応付け）で要件を満たせる。
- manifest / 依存 / feature 変更なし — windows-sys 0.61.2 の
  既存有効 feature のみで実装でき、本 slice の禁止範囲
  （`Cargo.toml` / `Cargo.lock`）に触れない。
- field 名は `display_adapters` とし、`source` に
  `"EnumDisplayDevicesW"` を刻んで provenance を明示する。

**DXGI / GPUI renderer-info は不採用**:

- 実 renderer adapter の特定は AC4 の環境記録要件を超える精度で、
  feature 追加または production API 新設のコストに見合わない。
- 代わりに限界を正直に記録する: `display_adapters` は GDI の
  display-adapter view であり、GPUI/compositor が選択した
  adapter の証明ではない旨を field・README・本 ADR に明記する。

## Consequences

- real lane 全 case の `environment` に `display_adapters` が
  記録され、AC4 'OS and GPU' が実装面で充足する。
- blocked-probe / headless 行には session 不在のため刻まれない
  （fail-closed 契約の維持 — session-scoped field は session
  存在時のみ）。
- 将来 GPU renderer 特定が必要になった場合は DXGI feature 追加
  を別件で検討する（本 ADR で「環境 census ≠ renderer 証明」の
  区別は記録済み）。
- `src/lib.rs` の stale コメントを現状追従（DPI/window-close
  実 verdict 化済み、IME のみ Unsupported）— 前 slice の残件
  を本 slice で解消。
