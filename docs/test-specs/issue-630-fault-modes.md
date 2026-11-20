---
id: "TSPEC-630faultModes01"
type: "test-spec"
status: "approved"
title: "Issue #630 fault-modes slice — test specification"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-fault-modes.md"
    - "docs/designs/issue-630-fault-modes.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-fault-modes.md"
  implements:
    - "docs/specs/issue-630-fault-modes.md"
    - "docs/designs/issue-630-fault-modes.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-fault.md"
  related:
    - "docs/test-specs/issue-630-test-fault.md"
    - "evidence/rf630/fault-modes-verification.md"
  verifies:
    - "docs/specs/issue-630-fault-modes.md"
    - "docs/designs/issue-630-fault-modes.md"
---

# Issue #630 fault-modes slice — test specification

## Feature: fault law 拡張（in-crate unit tests, `src/fault.rs`）

- **Rule**: `decode_fault` は 8 帯域を decode し、範囲外入力は
  deterministic に clamp する。
  **Example**: `4.0→error`、`5.0/5.99→rt_alloc`、`6.0/6.99→
  bad_time`、`7.0/7.99/8.0→invalid`、`9.0→invalid`、
  `exit` 帯域の小数部 ×100 → exit code。
- **Rule**: `rt_alloc` armed block は先頭出力 sample に
  `200.0` marker を書き残りは pass-through。
- **Rule**: `bad_time` armed block は `Continue +
  TAIL_INFINITE` を返し、disarm で `tail_frames == 0`。
- **Rule**: `invalid` armed block は全出力 channel/sample が
  `NaN` で status は `Continue`。
- **Rule**: `none` disarm はいずれの非致命 mode からも clean
  pass-through に回復する。
- **Rule**: `rt_unarmed_block_is_alloc_free` — 未武装 arm 適用 +
  pass-through は `assert_no_alloc` 下で alloc-free（setup は
  計測窓外）。armed `rt_alloc` 自体は RT 契約対象外。
- **Rule**: `abort`/`exit`/`hang` は in-process test で絶対に
  踏まない（既存境界、変更不要）。

## Feature: host_path（in-process 実 ABI, `tests/host_path.rs`）

- **Rule**: `Fault` descriptor は両 format で `max_value == 8.0`
  （VST3 は加えて `stepCount == 7` を descriptor 経由で pin）。
- **Rule**: `error` mode は normalized `0.5` → plain `4.0` で
  defined rejection（VST3 `kResultFalse` / CLAP
  `CLAP_PROCESS_ERROR`）— 既存 oracle の arm 値のみ更新。
- **Rule**: `rt_alloc`/`bad_time`/`invalid` を実 ABI 経由で
  arm → marker `200.0` / `tail_frames == u32::MAX` / 全 NaN を
  観測 → `none` disarm で回復。

## Feature: L1 fault E2E（`tests/l1_fault_test_plugin_e2e.rs`, ignored）

全 case は実 cdylib + `l1-runner` runner を spawn。fixture/runner
欠落は assert failure（silent skip なし）。armed/mutation submit は
`dropped_submits` delta == 0 の着地確認まで bounded resend
（`ruviform#763` 先例どおり — drop は event を含む block 全体を
喪失する）。content/status assert は per-call `audio_counters`
delta で matched call（`passthrough_submits` delta == 0）のみを
計数し、substituted call（hold-last `last_output` 再生）は
分類して skip する — assertion 内容・最低到達数 gate は緩めない。

- **rt_alloc_marks_armed_blocks**: normalized `0.625` arm →
  `out_left[0] == 200.0` + `out_left[1..]`/`out_right` は
  pass-through → `Processing` 維持・notification 無し →
  disarm で clean pass-through → clean teardown。
- **bad_time_claims_infinite_tail**: normalized `0.75` arm →
  `tail_frames == u32::MAX` 観測（VST3 `getTailSamples` /
  CLAP `Tail` → wire `tail_frames`）→ `Processing` 維持 →
  disarm で `tail_frames == 0`。
- **invalid_emits_nan_block**: normalized `0.875` arm →
  delivered output が全 sample `NaN` → `Processing` 維持・
  notification 無し → disarm で clean pass-through。
- **既存 case の rescale**: `abort`=`0.125`、`exit(42)`=`0.3025`、
  `exit(0)`=`0.25`、`hang`=`0.375`、`error`=`0.5` — semantics
  は全て不変（同じ plain 帯域へ写る）。
- **CLAP `submit_block_full`**: 出力 channel の内容まで検査
  できる block submit helper（marker/NaN 観測用に新規追加）。

## Feature: state codec v2（in-crate unit tests, `src/stateful.rs`）

- **Rule**: `inspect_state_blob` は provenance `Canonical`/
  `Migrated`/`Degraded` と typed rejection `Malformed`/
  `UnsupportedVersion`/`ChecksumMismatch` を返す。
  **Example**: canonical v2 round-trip、v1 16B は `Migrated`、
  v2 + tail は `Degraded`、checksum 反転・version=3・bad
  magic・不正長・非 finite は各 typed rejection。
- **Rule**: rejected blob は stored gain を変えない。
- **Rule**: `save→save` / `save→restore→save` は byte 同一、
  migrated/degraded 入力は次回 save で canonical 20B に収束。
- **Rule**: finite 範囲外 gain は `[0.0, 2.0]` に clamp
  （従来契約維持）。
- **Rule**: COM/stream 経路 — VST3 `IBStream` component/
  controller verb・single-component 面・CLAP `clap.ext.state`
  save/load で同契約（CLAP `load` は stream 全消費）。

## Feature: state host_path + L1 stateful E2E

- **host_path**: frame/payload offset が v2 20B 前提に更新、
  `stateful_restore_migrates_v1_and_degrades_extension` と
  `stateful_restore_rejects_malformed_blob` が migration・
  degraded・checksum・unsupported version を網羅。
- **l1_stateful_test_plugin_e2e** (ignored + l1-runner): v2 20B
  payload offset、migration・degraded・checksum corruption・
  unknown version を out-of-process で観測、reject 後の
  instance 生存 + canonical re-save + restored gain の audio
  反映を確認（VST3 5 case / CLAP 3 case）。

## Feature: editor popup（in-crate unit tests）

- **VST3 `popup_key_toggles_plugin_owned_window`**: 通常 key は
  `kd:` のみ、`VK_P` で `po:{seq}`/`pc:{seq}` toggle、
  Windows では popup HWND が実在、reopen で seq 増分、
  `removed()` が popup を畳む。
- **CLAP `floating_popup_surface_is_honest`**: floating `create`
  → `set_parent` 拒否・`set_transient` の live-HWND gate・
  `suggest_title` → `show` で popup HWND 実在・`hide`/再
  `show`/`destroy` 収束。embedded↔floating 混在は honest
  rejection。非 Windows は全 defined rejection。

## Lane

- `mise run test ruviform-plugin-test-{vst3,clap}`: unit +
  host_path（deterministic evidence）。
- `mise run test-l1-real-plugin`: `--features l1-runner` +
  `-- --ignored` の real-run evidence（両 ABI、fault e2e 全
  case）。
- `mise run test-rt`: `rt_unarmed_block_is_alloc_free` を含む
  workspace `rt_` filter。
- `mise run check-l1-real-plugin` / `lint-l1-real-plugin`:
  e2e file が type-check される唯一の lane（`l1-runner`
  feature 配下）。
- `mise run fmt-check` / `lint` / `prek run --all-files`。
