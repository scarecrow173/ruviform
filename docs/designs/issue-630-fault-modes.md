---
id: "DSN-630faultModes01"
type: "design"
status: "approved"
title: "Issue #630 fault-modes slice — 設計"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-fault-modes.md"
    - "docs/designs/issue-630-test-fault.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-fault-modes.md"
  implements:
    - "docs/specs/issue-630-fault-modes.md"
  implemented-by:
    - "docs/test-specs/issue-630-fault-modes.md"
  depends-on:
    - "docs/specs/issue-630-fault-modes.md"
    - "docs/designs/issue-630-test-fault.md"
  related:
    - "docs/designs/issue-630-test-fault.md"
    - "docs/test-specs/issue-630-fault-modes.md"
    - "evidence/rf630/fault-modes-verification.md"
---

# Issue #630 fault-modes slice — 設計

## 設計判断

### 帯域拡張（第 2 param を選ばない）

`ParamController` は single-param fixture で、複数 param 化は
本 slice の lease 外。`Fault` plain range を `[0.0, 4.0]` →
`[0.0, 8.0]` に広げ `floor(plain)` clamp `[0, 8]` で 8 帯域を
decode する既存 law の拡張とした。`plain = normalized × 8.0`
に変わるため既存 test の normalized arm 値は全て rescale
（`abort 0.25→0.125`、`exit(42) 0.605→0.3025`、`exit(0) 0.5→0.25`、
`hang 0.75→0.375`、`error 1.0→0.5`）。`error` は従来 plain
`4.0` 固定 → 新 law でも `[4.0, 5.0)` が error 帯域となり
plain 値は不変（帯域の開始点として再解釈）。

### `rt_alloc` の oracle — marker 方式

fixture cdylib の確保は host test バイナリの allocator 境界を
越えて観測不能（dylib 固有 allocator / `assert_no_alloc` は
in-process RT 契約用）。そのため `process()` 内で
`vec![0xC8; 4096]` を確保・充填し `std::hint::black_box` で
除去を防ぎ、先頭出力 sample に `f32::from(buf[0]) = 200.0` を
書き出す。出力 marker は「確保した buffer を実際に読んだ」
ことの fixture 側発行証拠であり、host allocator 検出の偽装
ではない — honest boundary として spec に明記。

### `bad_time` — neutral result の timing 虚偽

`ProcessResult::new(Continue, TAIL_INFINITE)`。wire は
`status u8 + tail u32` で neutral result をそのまま運ぶため、
両 ABI で観測可能:

- VST3: component が最新 tail を atomic に保持し
  `getTailSamples()` が `u32::MAX` を返す。host bridge が同
  verb を読んで `tail_frames` に写す。
- CLAP: `map_process_status` が nonzero tail の `Continue` を
  `ClapProcessStatus::Tail` に写像し、host bridge が
  `ProcessResult { Continue, TAIL_INFINITE }` へ再構成。

実 tail を持たない pass-through fixture が unbounded tail を
主張すること自体が timing lie — 戻り値のみの虚偽で child は
死なず、disarm で `tail_frames == 0` に回復する。

### `invalid` — 出力の不正状態注入

全出力 channel/sample を `f32::NAN` で fill し status は
`Continue` を維持（「出力と status の矛盾」が注入する不正
状態）。L1 側は delivered output を全 NaN で走査して観測、
disarm で clean pass-through 回復を確認する。

### CLAP 固有 leg — `events.output` の malformed event

clack の `events.output` は crate 内に既存で、本 slice の
境界内。`bad_time` 時は block 境界を超えた時刻の
`ParamValue` event、`invalid` 時は NaN 値の `ParamValue`
event を ABI 上で発行する。ただし ruviform v1 host bridge は
plugin 出力を `OutputEvents::void()` に捨てるため host 側では
観測できず、`try_push` は best-effort（戻り値無視）。発行面
だけを honest に記録し、VST3 側には対応 leg を置かない。

## VST3 側変更

- `fault.rs`: `FAULT_STEP_COUNT = 4 → 7`、`FAULT_PARAM.max_value
  = 8.0`、`FaultMode::{RtAlloc, BadTime, Invalid}`、
  `decode_fault` の 8 帯域化、`TestFaultProcessor::process` に
  3 mode 追加。`RT_ALLOC_CAPACITY = 4096` /
  `RT_ALLOC_FILL = 0xC8` const。module doc の帯域表更新。
- `tests/host_path.rs`: `fault.max_value == 8.0` pin、
  `error` arm を normalized `0.5` に rescale、新 3 mode の
  ABI 実走 test 追加。

## CLAP 側変更

- `fault.rs`: `FAULT_PARAM.max_value = 8.0`、同 enum/decode/
  process 拡張、`bad_time_output_event()` /
  `invalid_output_event()` helper と `events.output.try_push`
  leg を `TestFaultAudioProcessor::process` に追加。module
  doc の帯域表更新。
- `tests/host_path.rs`: `fault.max_value == 8.0`、同 rescale +
  新 3 mode test。

## E2E 追加（両 crate `tests/l1_fault_test_plugin_e2e.rs`）

- module doc の normalized 対応表を `[0.0, 8.0]` law に更新、
  既存 5 case の arm 値を rescale。
- `submit_block_full` helper（CLAP は新規追加）で出力 channel
  内容を検査可能に。
- 新 3 case: armed submit を繰り返し（lookahead/drop 対策は
  既存先例どおり）、marker / `tail_frames == u32::MAX` /
  全 NaN を観測 → `Processing` 維持 + notification 無し →
  `none` disarm で pass-through / `tail_frames == 0` 回復 →
  `StopProcessing`/`Deactivate`/`Destroy` clean teardown。

## RT boundary

inert 経路（`none`/未武装の pass-through + arm 値適用）は
alloc/lock/wait-free を維持 — `rt_unarmed_block_is_alloc_free`
が setup を計測窓外に置いて `assert_no_alloc` で pin する。
`rt_alloc` の確保は **armed block 内だけ**の意図的 fault で
あり、`assert_no_alloc` の対象にはしない（armed fault は
RT 契約の対象外 — `abort`/`exit`/`hang` と同じ扱い）。

## State codec v2（拡張 scope）

### codec を単一実装に集約する

`inspect_state_blob` を唯一の decode 入口とし、provenance
(`Canonical`/`Migrated`/`Degraded`) と typed rejection
(`Malformed`/`UnsupportedVersion`/`ChecksumMismatch`) を返す。
`decode_state_blob` は `Option<f64>` を返す薄い wrapper として
残す — `component.rs`/`controller.rs`/`preset.rs`/`single.rs`
が共有する既存 signature は lease 外 caller を巻き込むため
変更しない（AGENTS「正しい境界で修正」: codec の domain rule を
`stateful.rs` 一箇所に置き、caller 側に補正を増やさない）。

- v2 canonical = magic `RFST` + version=2 + f64 gain + 先頭 16B
  の FNV-1a-32 checksum (20B)。
- v1 (16B) は checksum を持たないため受理時 `Migrated` と
  判別し、次回 `encode_state_blob` は常に canonical v2 を吐く
  — migrate は read 側の解釈であって write 面は単一形式に
  収束させる（DRY）。
- v2 + 正当 checksum + 余剰 tail は `Degraded` 受理 — tail を
  捨てて reserialize すると canonical に収束する。version が
  未知のものは tail の有無に関わらず `UnsupportedVersion` で
  拒否し、先頭 8B 以外を信頼しない（untrusted input は codec
  境界で検証、呼び出し側に防御を散らさない）。
- checksum 検証は version 確定後・gain 解釈前 — checksum を
  pass した blob のみ f64 を読み、非 finite は `Malformed`。

### VST3 側

`stateful.rs` の `STATE_BLOB_LEN` を 16→20 にし、component/
controller/single の state verb は全て同じ codec を通るため
frame 長だけが変わる — caller は blob bytes を透過するだけで
migration/degraded 判定を持たない。`setState` 系の rejection
写像は既存どおり `kResultFalse`。

### CLAP 側

`clap.ext.state::load` は `InputStream` (= `io::Read`) を
`read_to_end` で全消費する — 旧実装の「canonical 長ぴったり
要求」は degraded tail と両立しない。検証と適用は分離し、
inspect 成功時のみ gain mirror を更新 + params values rescan
通知を発行する（rejected load は状態不変が観測契約）。

## Editor popup（拡張 scope）

### VST3 — plugin 所有 popup HWND + key 契約

`IPlugView` は popup の verb を持たないため、fixture は key
input を契約とする: `onKeyDown` は既存どおり `kd:{key}:
{key_code}:{modifiers}` を記録した上で `key_code == 0x50`
(`VK_P`) のみ `popup_toggle` を呼ぶ。open は `po:{seq}`・
close は `pc:{seq}` を同じ event log (window title) に記録し、
`win32::popup` が `WS_POPUP | WS_VISIBLE | WS_CLIPSIBLINGS`
HWND を editor child の root ancestor (`GetAncestor(GA_ROOT)`)
を owner に生成する — editor HWND が死んでも popup が残らない
owner 契約。`removed()` は child 破棄前に popup を必ず畳む。
非 Windows shim は `None` を返すが `po:`/`pc:` token は記録
される — 「verb が走った」論理面と HWND 実在面を分離するのは
既存 `attached` token の先例と同じ。

### CLAP — `is_floating` floating window 契約

CLAP gui 拡張には既に popup 相当の verb がある: floating GUI +
`set_transient` + `suggest_title` + `show`。`GuiState` に
`floating`/`transient`/`title` を追加し、`is_api_supported`
を `is_floating` 両方で受理（Windows のみ — 非 Windows と
非 Win32 API は従来どおり honest `false`）。`create` が mode
を記録し、混在 verb は相互に defined rejection (`set_parent`
on floating / `set_transient` on embedded)。`set_transient`
は `as_win32_hwnd` + `IsWindow` gate を `set_parent` と同じ
境界で行い、`show` が初回に `win32::popup` で実 HWND を生成
する（transient owner + 提案 title または default）。host
harness が無いため popup の L1 面は設けず、plugin-impl 単体
test を受入 oracle とする（AC8）。
