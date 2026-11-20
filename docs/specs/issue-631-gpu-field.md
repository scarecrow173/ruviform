---
id: "SPEC-r7Tn2mQv9Kw4xLb8cJd5Hs"
type: "spec"
status: "approved"
title: "Issue #631 AC4 'OS and GPU' display-adapter field"
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
    - "docs/discovery/issue-631-gpu-field.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 AC4 'OS and GPU' display-adapter field

## What Should Be Built

Issue #631 AC4 の 'OS and GPU' field のうち GPU 側を、real lane
の session `environment` に `EnumDisplayDevicesW` ベースの
display-adapter census として記録する。

### 記録 field（session-scoped）

全 real case の `environment.display_adapters` に以下の JSON
object を刻む（session 開始時に一度だけ採取した値を全 case に
同値で付与）:

| key | 内容 |
| --- | --- |
| `source` | 固定値 `"EnumDisplayDevicesW"` |
| `supported` | OS が列挙 API を提供するか（Windows: true、非 Windows stub: false） |
| `truncated` | adapter 列挙の safety cap（64）到達で打ち切ったか |
| `error` | 初回列挙失敗時のエラー文字列。成功時 `null` |
| `adapters` | adapter 配列（後述） |

各 adapter entry:

| key | 内容 |
| --- | --- |
| `index` | `EnumDisplayDevicesW` の `idevnum`（0 起算の列挙順） |
| `device_name` | `DISPLAY_DEVICEW.DeviceName`（`\\.\DISPLAYn`） |
| `device_string` | `DeviceString`（例: GPU 製品名） |
| `device_id` | `DeviceID`（PnP ID。`EDD_GET_DEVICE_INTERFACE_NAME` 時は interface 名） |
| `device_key` | `DeviceKey`（registry key） |
| `state_flags` | `StateFlags` 生値（数値） |
| `attached_to_desktop` | `DISPLAY_DEVICE_ATTACHED_TO_DESKTOP` bit の decode |
| `primary` | `DISPLAY_DEVICE_PRIMARY_DEVICE` bit の decode |
| `monitors` | 第2段列挙の monitor entry 配列（`index` / `device_name` / `device_string` / `device_id` / `device_key` / `state_flags`） |

### fail-closed 契約（不変）

- blocked-probe / headless 行には session が存在しないため
  `display_adapters` を刻まない（`window_identities`・
  `scale_factor` と同じ session-scoped 規約）。
- 列挙失敗時は空配列を返すのではなく `error` に理由を記録する。
- 非 Windows では stub が `supported: false`・空 `adapters` を
  正直に返す（verdict には使用しない）。

### 識別の限界

`display_adapters` は GDI の display-adapter view の記録であり、
GPUI / compositor が選択した実 renderer adapter の証明ではない。
この限界は README・設計・ADR に明記する。

### 同梱の軽微残件

`src/lib.rs` の stale コメント（DPI / window-close を
`Unsupported` と記した旧表現）を現状（実 verdict 化済み、
`Unsupported` は `real-window-ime` のみ）に追従させる。

## Out of Scope

- catalog 件数変更（headless 9 + real 22 を維持）
- verdict ロジック変更（本 field は記録のみで判定に使わない）
- `real-dcomp-pixel` 既知 Fail の修正（`ruviform#532` gate）
- 新規依存・feature・manifest 変更・新規 CI lane
- DXGI / GPUI renderer-info による実 renderer adapter 特定
  （Coordinator 決定により本 slice 対象外）
- `plugin-dock-console.rs` の stale コメント（lease 外の既知残件）
- consumer 投影 gate（`synvelis#114`/`synvelis#176`/`audio-lens#127`）

## Acceptance Criteria

- [ ] `test-ui-plugin-dock-real`（interactive desktop）実走で全
  real case の `environment.display_adapters` に adapter census
  が記録される（`plugin-dock-real-scenario.jsonl` 再生成）。
- [ ] `test-ui-plugin-dock-real-headless`（blocked-probe）で全
  22 行が `not_run` + `environment_blocked` のまま、
  `display_adapters` を含まない（fail-closed）。
- [ ] `tests/plugin_dock.rs` の feature-gated テストで blocked
  行の session-scoped field 不在（`window_identities`・
  `display_adapters` 不在・`scale_factor: null`）を検証。
- [ ] `real_dock.rs` unit test で JSON projection shape・error
  記録・live 列挙の誠実性（supported・identity・index 順・
  flag decode）を検証。
- [ ] 指定の check / lint / test / UI lane 全て green（real
  lane は既知 `real-dcomp-pixel` Fail=exit 1 を正直に保持）。
