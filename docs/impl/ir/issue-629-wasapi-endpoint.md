---
id: "IMPL-034YbQpZeewSk6QoG6ahZJ"
type: "impl"
status: "completed"
title: "Issue #629 WASAPI endpoint streaming verifier"
created: "2026-11-15"
updated: "2026-11-16"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/tasks/issue-629-wasapi-endpoint.md"
    - "docs/plans/issue-629-wasapi-endpoint.md"
  changes:
    added:
      - type: "code"
        path: "crates/ruviform-audio-device/tests/virtual_endpoint.rs"
      - type: "doc"
        path: "docs/specs/issue-629-wasapi-endpoint.md"
      - type: "doc"
        path: "docs/designs/issue-629-wasapi-endpoint.md"
      - type: "doc"
        path: "docs/plans/issue-629-wasapi-endpoint.md"
      - type: "doc"
        path: "docs/tasks/issue-629-wasapi-endpoint.md"
      - type: "doc"
        path: "docs/test-specs/issue-629-wasapi-endpoint.md"
      - type: "doc"
        path: "docs/impl/exp/issue-629-wasapi-endpoint.md"
      - type: "doc"
        path: "docs/impl/exp/issue-629-wasapi-endpoint-exp.jsonl"
      - type: "evidence"
        path: "evidence/rf629/virtual_endpoint_slice4_verification-windows-local.txt"
    modified:
      - type: "code"
        path: "crates/ruviform-audio-device/examples/endpoint_probe.rs"
      - type: "doc"
        path: "docs/discovery/issue-629-virtual-endpoint-survey.md"
      - type: "evidence"
        path: "evidence/rf629/endpoint_probe-windows-local.txt"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-629-wasapi-endpoint.md"
    - "docs/plans/issue-629-wasapi-endpoint.md"
    - "docs/specs/issue-629-wasapi-endpoint.md"
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-wasapi-endpoint.md"
    - "docs/impl/exp/issue-629-wasapi-endpoint.md"
    - "docs/impl/ir/issue-629-virtual-endpoint-survey.md"
    - "docs/impl/ir/issue-629-midi-loopback.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-629-wasapi-endpoint.md"
    - "docs/plans/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-629-wasapi-endpoint.md"
  verified-by: []
  references:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-629-wasapi-endpoint.md"
    rejected: []
---

# Issue #629 WASAPI endpoint streaming verifier

## Summary

Issue #629 4th slice。Windows 実 OS 仮想 Audio endpoint に対する
WASAPI streaming verifier（`tests/virtual_endpoint.rs`）を新設し、
`examples/endpoint_probe.rs` を pair-candidate 表・driver 署名/
権限の best-effort 観測・cleanup 記録で拡張した。verifier は
`real-device` feature + `#[ignore]` の manual lane で、production
`DeviceEnumerator`/`Device`/stream trait のみを使い、独立 oracle
（test 内 deterministic generator）で render→capture の
round-trip を照合する。未 provisioning 環境では fail-closed の
`NotRun — NOT_PROVISIONED` を返し、`virtual-os-endpoint` や
EXT-L3 を主張しない。

## Context

`docs/tasks/issue-629-wasapi-endpoint.md`（plan
`docs/plans/issue-629-wasapi-endpoint.md`、spec
`docs/specs/issue-629-wasapi-endpoint.md`、design
`docs/designs/issue-629-wasapi-endpoint.md`）に従う。本 slice は
AC1 の WASAPI render→capture streaming verifier の headless 実装
単位と、AC3 Test Runtime 結線・AC4 記録拡張の headless 範囲を担当
する。Coordinator 決定 (A) により `ruviform-audio-test` への
dev-dep 追加は行わず、oracle は test 内に自足させた。production
契約（`ruviform-audio-core`/`ruviform-audio-device` の src）は
無変更。

## Implementation

### Changes

- `crates/ruviform-audio-device/tests/virtual_endpoint.rs`（新設、
  約 1250 行）:
  - `#![cfg(feature = "real-device")]` + `#[test] #[ignore]` —
    `real_input_e2e.rs` と同型の manual lane。canonical 起動は
    `cargo test -p ruviform-audio-device --features real-device
    --test virtual_endpoint -- --ignored --test-threads=1`
    （evidence 採取時は `--nocapture` を付記）。
  - `default_host()` → `output_devices()`/`input_devices()` 列挙 →
    `is_virtual_name` + direction-token 除去 stem による pair
    candidate 選定（`endpoint_probe` と同一規則を複写・sync
    コメント付き）。pair なし → `verdict: NotRun —
    NOT_PROVISIONED` で return（assert/panic なし）。
  - format 交渉は `real_input_e2e.rs` の規則を両方向に適用
    （default F32 優先、supported space から F32 range 選択、
    48 kHz clamp、既知 channel 数→layout）。
  - `open_*_stream` は `BufferSize::Fixed(480)` 優先、
    `UnsupportedConfig` で `Default` に fallback。
  - RT callback は `real_input_e2e.rs` の観測 discipline を両方向
    に拡張: frame 数/min/max、timestamp monotonic、clock domain
    一致、`capture <= callback` 順序、`frame_position` 連続性
    （gap > 通知済み discontinuity で Fail）、non-finite/peak scan、
    latency 記録、error/xrun/discontinuity/invalidated counter。
    capture 側は channel 0 を pre-allocated `CaptureBuf` に
    `try_lock` で bounded copy（溢れ・lock 失敗は counter 可視化）。
  - 出力 stimulus は独立 oracle `signal_sample`（440 Hz sine
    0.20 + 8 frame 0.70 marker/100 ms）— 全 channel 同一、
    絶対 frame index で callback サイズ非依存。
  - oracle は marker 検出（`|x|>0.5` + 20 ms refractory、≥3 個・
    平均間隔 100 ms ±10%）、RMS ratio [0.2, 3.0]、Goertzel
    440 Hz tone share ≥0.25 の 3 系統。閾値は provisional。
  - lifecycle: start → 3 s 観測 → stop → post-stop query 記録 →
    restart → re-stop → drop（API に close はなく Drop が配信を
    止める）。60 s watchdog（超過で exit 3）。
  - verdict: 全 pair 失敗で `panic!("verdict: Fail — …")`、
    いずれか Pass で `verdict: Pass — …`（name match は
    candidate であり `virtual-os-endpoint` 証明ではない旨を
    明記）。
- `crates/ruviform-audio-device/examples/endpoint_probe.rs`:
  - module doc を 4th slice 内容に更新（pair 表・OS 観測・
    cleanup・verifier 参照）。
  - `PortDevice`/`DIRECTION_TOKENS`/`virtual_stem`/
    `pair_candidates`/`print_pair_candidates` を追加 —
    port view の (name, direction) から verifier と同一規則で
    pair 表を印字。unpaired virtual candidate も列挙。
  - `print_driver_signature`: `Win32_PnPSignedDriver` を
    PowerShell 経由で name-hint regex 照合し、staged catalog
    （`C:\Windows\INF\<inf>.cat`）に `Get-AuthenticodeSignature`
    を best-effort 適用。失敗は `NotRun` note、exit 0 維持。
  - `print_capture_permission`: HKCU privacy keys
    （`DeviceAccess\Global\{2EEF81BE-…}`、
    `ConsentStore\microphone`、`…\NonPackaged`）を
    `Get-ItemProperty` で照会。absent/失敗も正直に記録。
  - `print_verdict` を拡張: pair 数・driver/permission status・
    cleanup（persistent state なし）・verifier 起動方法を記録。
  - non-Windows verdict に新規 section の `NotRun` 行を追加。
- `docs/discovery/issue-629-virtual-endpoint-survey.md`:
  Known Code Gap #4（driver 署名）に 4th slice の best-effort
  実装を追記、Evidence 節に 2026-11-16 の実機記録を append。
- `evidence/rf629/endpoint_probe-windows-local.txt` を実機再実行
  結果で更新、`evidence/rf629/virtual_endpoint_slice4_verification-
  windows-local.txt` を新設。

### Decisions During Implementation

- **Test Runtime 結線は docs のみ（Coordinator 決定 (A)）**:
  `ruviform-audio-device -> ruviform-audio-test` dev-dep は承認済み
  依存変更リスト外のため不採用。oracle は test 内独立実装。
  #628 `PcmFixture`/`RunMetadata` との結線 IF は design doc に記録
  し、provisioned-runner 後続 slice へ hand-off。
- **capture buffer は channel 0 のみ**: oracle は 1 ch で足り、
  全 channel の non-finite/peak は callback 内 slab scan で別途
  観測するため、容量を rate×(CAPTURE_SECONDS+1) frames に抑えた。
- **frame_position gap と ClockDiscontinuity の対応**: gap 数が
  通知済み discontinuity 数を超えた場合のみ Fail（未通知の不連続
  は契約違反）。xrun/error/discontinuity の発生自体は AC5 領域
  として記録のみで gate しない。
- **`StreamInvalidated` は NotRun 固定**: 対話的 device 除去が必要
  で本 slice 範囲外。evidence block に明示行を出す。
- **powerShell/registry 観測は fail-safe**: spawn 失敗・非ゼロ
  終了・キー absent は全て `NotRun`/`absent` の note で、probe
  の exit code 0 を維持する（`midi_endpoint_probe` の WMS 検出
  先例に準拠）。
- **docs index は未再生成**: `docs/**/README.md` は生成物で本
  slice の編集範囲外 — Coordinator 統合時に再生成される前提で
  PR 本文の「未対応の作業」に明記。

## Related Experiments

### Adopted

- [`docs/impl/exp/issue-629-wasapi-endpoint.md`](../exp/issue-629-wasapi-endpoint.md)
  （+ `issue-629-wasapi-endpoint-exp.jsonl`）— feature gate/依存
  境界、pair 規則同期、oracle 独立性、lint 修正経緯、実機
  fail-closed 確認の全試行を採用。

### Rejected

- なし（exp は単一アプローチの記録。`ruviform-audio-test` dev-dep
  案は Coordinator 決定 (A) で棄却済み — exp 試行 1 に記録）。

## Validation

windows-local (DESKTOP-NH00KUI, Windows 11 10.0.26300.9974,
rustc 1.99.1) で実施。詳細は
`evidence/rf629/virtual_endpoint_slice4_verification-windows-local.txt`。

| command | exit | result |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-audio-device` | 0 | Pass |
| `mise run lint ruviform-audio-device` | 0 | Pass |
| `mise run test ruviform-audio-device` | 0 | Pass（`virtual_endpoint` は feature なしで空 compile・0 tests） |
| `mise run lint-audio-real-input` | 0 | Pass（`--all-targets --features real-device -D warnings`） |
| `mise exec -- prek run --all-files` | 0 | Pass |
| `cargo run -p ruviform-audio-device --example endpoint_probe` | 0 | Pass — pair 表・driver/permission 観測・cleanup 記録（補助証拠） |
| `cargo test -p ruviform-audio-device --features real-device --test virtual_endpoint -- --ignored --test-threads=1 --nocapture` | 0 | `verdict: NotRun — NOT_PROVISIONED`（補助証拠） |

## Risks

- **Pass 経路は未実走**: oracle 閾値（level band・tone share・
  marker interval）は provisional で、provisioned runner の実測で
  調整が要る可能性がある。誤 Fail/誤 Pass のリスクは evidence の
  生値記録で緩和する設計。
- **name-stem pair の偽陽性**: 同名規則の無関係 endpoint が
  pair になる可能性は残るが、実 streaming 照合が Pass の条件
  なので偽陽性 pair は Fail として顕在化する（fail-closed）。
- **capture buffer 容量**: negotiated input rate×4 s を超える
  長時間・高 rate では overflow counter が立つ — 記録されるため
  silent loss ではない。

## Follow-ups

- provisioned runner での実走 Pass 取得（driver install は本
  slice 範囲外・専用 runner 前提）。
- AC6 opt-in lane 登録（`mise.toml` — 本 slice では docs 記録のみ、
  `docs/designs/issue-629-wasapi-endpoint.md` 参照）。
- #628 `PcmFixture`/`RunMetadata` との完全結線（後続 slice —
  interface は design doc に記録済み）。
- `StreamInvalidated`/xrun/permission-denial 再現の実測（AC5）。
- docs index 再生成（Coordinator 統合時）。
