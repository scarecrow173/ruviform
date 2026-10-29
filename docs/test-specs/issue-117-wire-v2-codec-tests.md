---
id: "TSPEC-034NlQ76ZnpNmAnyT0HgY9"
type: "test-spec"
status: "approved"
title: "L1 wire v2 codec tests（Issue #117 — instance envelope・新動詞・scope 判定表・InstanceLimit）"
created: "2026-10-28"
updated: "2026-10-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
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
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 wire v2 codec tests（Issue #117 — instance envelope・新動詞・scope 判定表・InstanceLimit）

## Purpose

wire v2（`WIRE_VERSION` 1→2 一括 bump・Issue #119 の「単一最新版のみ」方針に従う wholesale 改訂）で導入される **増分**契約の oracle。envelope の `instance` field・新動詞 `CreateInstance`/`DestroyInstance`・verb↔scope 判定表・`HostError::InstanceLimit` wire tag 9・`WIRE_MAX_INSTANCES`/`WIRE_INSTANCE_NONE` 定数・adversarial 入力への bounded 拒否を pin する。frame ヘッダ・kind 表・shm レイアウト等の**引き継ぎ**契約は `plugin-l1-wire-codec-tests.md`（v2 修正済みの baseline oracle）が引き続き所有する — 本 spec は #117 が追加する面のみを対象とする。wire v2 が supersede された後は本 spec は引き継ぎ oracle へ統合され得る。

**re-engagement 注記（Issue #118・wire v3）**: Issue #118 で `WIRE_VERSION` は 2→3 へ再改訂された — video `Command` tag 12-22・`Reply` tag 4-7・`HostError` tag 10 `InvalidValue`・video 型 codec・`WIRE_VIDEO_MAX_*`・`wire::vshm` 定盤の additive 追加。本 spec が pin した v2 増分契約（`instance` envelope・scope 判定表・`WIRE_MAX_INSTANCES`/`WIRE_INSTANCE_NONE`・`InstanceLimit` tag 9・`CreateInstance`/`DestroyInstance` tag 10/11）は audio 経路で**不変のまま v3 下でも成立**する — 下記 `WIRE_VERSION == 2` のリテラル記述は #117 時点の pin として維持し、現行値への言及は 3 と読み替える。video 面の codec pin は `issue-118-wire-v3-codec-tests` が oracle。

## Feature

wire v2 codec は `Message`/`Command`/`Reply`/`BlockIn`/`BlockDone` を instance 付き envelope で符号化し、`instance` の値域・動詞との scope 整合・宣言長/count を `Malformed` で拒否する。`HostError::InstanceLimit` は encode/decode の両方向で tag 9 と round-trip する。

## Rules

- `WIRE_VERSION == 2`（**#117 時点の pin — Issue #118 で現行値は 3**。v2 で導入された下記定数・envelope 規則は v3 でも不変）・`WIRE_MAX_INSTANCES == 64`・`WIRE_INSTANCE_NONE == u32::MAX` が codec 定数として存在する
- `Command`/`Reply`/`BlockIn`/`BlockDone` payload は `[u64 seq][u32 instance][rest]` envelope を使う。`Hello`/`Shutdown` は `instance` を持たない
- `Command` に `CreateInstance{plugin_id: String, shm_key: String, in_depth: u32, out_depth: u32}`（tag 10）・`DestroyInstance`（tag 11）が追加される。既存 tag 0-9 の payload 意味論は変わらない。`plugin_id`/`shm_key` は `WIRE_MAX_STRING_BYTES`・`in_depth`/`out_depth` は `1..=WIRE_MAX_ELEMENTS` の bound 検査を受ける
- decode の scope 判定表: `Scan` は `instance == WIRE_INSTANCE_NONE` 必須。instance-scope 動詞（11 個）は non-NONE 必須。`BlockIn`/`BlockDone`/`Reply` の `instance` は動詞に応じた必須条件（`i < WIRE_MAX_INSTANCES` または `NONE` のみ）を満たす。違反は `WireError::Malformed`
- `encode_host_error(HostError::InstanceLimit) → tag 9`・`decode_host_error(9) → HostError::InstanceLimit` が両方向で round-trip する（wildcard arm が `UnsupportedVariant`/`Malformed` へ潰さない — 「定義済み拒否が子死亡に化ける」退行防止）
- decode は untrusted 入力の検証境界 — 範囲外 `instance`（`>63` かつ `!= NONE`）・scope 不整合・宣言長超過を panic・無制限 alloc なしで `Malformed` 拒否する
- 公開 enum（`Command`/`Reply`/`HostError`）は `#[non_exhaustive]` を維持する
- v1 Hello（`version != WIRE_VERSION` — #117 時点 `!= 2`・現行 `!= 3`）は handshake 失敗（version 一致は shm header `wire_version`・Hello `version` の双方で厳密一致 — #119 方針どおり交渉機構なし。`wire::vshm` header の `wire_version` も同規則）

## Examples

- 全動詞 + 新 2 動詞の round-trip: `CreateInstance{plugin_id, shm_key, in_depth:3, out_depth:3}`（tag 10・payload bounds）・`DestroyInstance`（tag 11）を含む `Command{seq, instance, cmd}` が encode→decode で同一値を返す
- scope 違反行列: `Scan` に `instance=0` → `Malformed`・instance 動詞に `instance=NONE` → `Malformed`・`BlockIn` に `instance=NONE` → `Malformed`・`i=64`（`=WIRE_MAX_INSTANCES`）→ `Malformed`・`i=u32::MAX-1` → `Malformed`
- `InstanceLimit` 両方向 pin: `Reply{seq, instance, Err(InstanceLimit)}` の encode→decode が `InstanceLimit` を返す（wildcard arm に落ちない）。decode 側の tag 9 → `InstanceLimit`・encode 側の `InstanceLimit` → 9 を個別 assert
- adversarial `instance` field: 宣言 `instance=63`（上限内）で decode 成功・`instance=64` で `Malformed`・`instance=u32::MAX`（`NONE`）が instance 動詞で `Malformed` — いずれも panic/無制限 alloc なし
- v1 Hello: `Hello{version: 1, capabilities: 0}` は `version != WIRE_VERSION` で handshake 失敗（host 側 proxy 経由 — cross-ref `plugin-l1-proxy-contract-tests`）

## Guarantees

- codec は wire v2 契約の authoritative 境界 — envelope・scope 判定・`InstanceLimit` tag の正本を本 spec が pin する
- `InstanceLimit` は encode/decode の両方向で明示 round-trip され、wildcard arm による黙認退行を防ぐ
- decode は adversarial `instance` field・scope 不整合・宣言長超過を panic/無制限 alloc なしで `Malformed` 拒否する
- 新動詞は `#[non_exhaustive]` を維持し、未知 variant は `UnsupportedVariant`/defined 拒否経路へ留まる

## Non-goals

- host/runner の demux・registry・converge 意味論（`issue-117-bundle-failed-granularity-tests`・`issue-117-bundle-e2e-tests` が対象）
- frame ヘッダ・kind 表・shm レイアウト・`Hello`/`Shutdown` 等の引き継ぎ契約（`plugin-l1-wire-codec-tests.md` が所有）
- handshake の spawn/argv 契約（`plugin-l1-proxy-contract-tests`・`plugin-l1-runner-e2e-tests` が対象）
- `InstanceLimit` の発生条件（pool/registry 上限 — `issue-117-bundle-e2e-tests` が対象）
- video 系統の codec（`Command` tag 12-22・`Reply` tag 4-7・`HostError` tag 10・`VideoFrameDesc`/`PlainParamInfoOwned`・`WIRE_VIDEO_MAX_*`・`wire::vshm`）— Issue #118 で追加・`issue-118-wire-v3-codec-tests` が oracle

## Risk

- scope 判定表の行漏れ → 正当 frame を `Malformed` 拒否 or 違反 frame を受理 → 子 scope 誤収束 or 未定義 demux 動作
- `InstanceLimit` の wildcard arm 見落とし → defined 拒否が `Malformed`/`UnsupportedVariant` へ化け、子が誤収束して bundle 内全 instance が `Failed` 化する
- envelope field 順序・byte 長の誤り → `seq`/`instance` の parse ずれで全 frame が壊れる
- `u32::MAX`/`NONE` の境界処理ミス → 正当な `Scan` が拒否される or `NONE` が instance 動詞へ混入する
