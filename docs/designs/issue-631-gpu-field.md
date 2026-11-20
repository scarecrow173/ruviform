---
id: "DSGN-b4Xc8vLn2Kp9wQm6rTs3Jh"
type: "design"
status: "approved"
title: "Issue #631 AC4 'OS and GPU' display-adapter field — 設計"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-gpu-field.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 AC4 'OS and GPU' display-adapter field — 設計

## 概要

real lane session `environment` に `display_adapters` を追加する
設計。`EnumDisplayDevicesW` による display-adapter census を
session 開始時に一度だけ採取し、既存の session extras 経路
（`session_scale_factor`・`window_identities` と同じ
`finalize_case` merged extras）で全 real case に刻む。

## モジュール構成

### `src/osgpu.rs`（新規）

`ospix.rs` / `ospres.rs` / `osoccl.rs` / `osmut.rs` と同じ OS
観測補助 module の配置規約に従う。test-only・非 RT・読み取り
専用（window 生成・mutation・message 送信なし）。

公開面（driver-neutral — Win32 型は境界を越えない）:

- `DisplayAdapters { supported, adapters, truncated, error }`
- `DisplayAdapter { index, device_name, device_string, device_id,
  device_key, state_flags, attached_to_desktop, primary, monitors }`
- `DisplayMonitor { index, device_name, device_string, device_id,
  device_key, state_flags }`
- `display_adapters() -> DisplayAdapters`

Windows 実装:

- 第1段: `EnumDisplayDevicesW(null, idevnum, &mut dd, 0)` を
  `idevnum = 0..` で列挙し adapter identity（`DeviceName`・
  `DeviceString`・`DeviceID`・`DeviceKey`・`StateFlags`）を取得。
- 第2段: adapter の `DeviceName` を device 引数に
  `EnumDisplayDevicesW(name, jdevnum, &mut dd,
  EDD_GET_DEVICE_INTERFACE_NAME)` で monitor entry を列挙し、
  device interface 名を `DeviceID` に得る。
- `StateFlags` から `DISPLAY_DEVICE_ATTACHED_TO_DESKTOP`（bit 0）
  と `DISPLAY_DEVICE_PRIMARY_DEVICE`（bit 2）を bool に decode。
- UTF-16 固定配列は `String::from_utf16_lossy` で終端までを Rust
  文字列化（値の捏造なし）。
- `idevnum = 0` の失敗のみ error 記録。以降の `FALSE` 戻り値は
  列挙終端（正常）として扱う。
- safety cap 1024 で打ち切り、到達時 `truncated = true`。

非 Windows stub: `supported: false`・空 `adapters`・
`error: None`・`truncated: false` を返す。

### `src/real_dock.rs` 配線

- `RealDockContext` に `display_adapters: Value` を追加。
- live session 開始時に
  `display_adapters_json(&osgpu::display_adapters())` で一度だけ
  census を採取し context に保持（`session_scale_factor`・
  `window_identities` と同じ位置）。
- `finalize_case` で全 real case の `environment` に
  `display_adapters` を insert。
- `blocked_reports` には触れない — session 不在経路には刻まない
  （fail-closed 契約維持）。

### `src/lib.rs`

- `#[cfg(feature = "real-dock")] mod osgpu;` を追加。
- AC1 残件の stale コメントを現状追従（command-receipt 実装済・
  DPI は live 観測・window-close は armed lifecycle・
  `Unsupported` は `real-window-ime` のみ）。

## 設計上の判断

- **field 名 `display_adapters`**: 記録対象が「GPU の renderer
  情報」ではなく「OS の display-adapter census」であることを
  名前で正直に表す（`gpu` という仮名は renderer 特定と誤読
  され得るため不採用）。
- **session 開始時の単回採取**: 列挙は冪等で高頻度変化しない
  OS 情報のため、case ごとではなく session 単位で採取する
  （`window_identities` と同じ粒度）。
- **`truncated`/`error` の別軸記録**: 空配列を成功と誤認させず、
  失敗を失敗として記録する fail-closed の原原則に従う。
- **renderer adapter 非特定の明記**: `source` field と README/
  ADR に GDI view の限界を記録し、DXGI 等による実 adapter 特定
  を本 field が主張しないことを明示する。

## テスト方針

- `display_adapters_json_shape`: 合成 inventory → JSON shape
  （source/supported/truncated/error/adapter identity/flag
  decode/monitor fields）。
- 失敗 shape: error 記録で空 `adapters`。
- Windows live 列挙: `supported`・非空時の identity 存在・
  `index` が `idevnum` 順・flag decode 整合・空なら `error` 必須。
- `blocked_probe_rows_carry_no_session_scoped_fields`
  （feature-gated integration test）: blocked 22 行で
  `window_identities` / `display_adapters` 不在・
  `scale_factor: null`。
