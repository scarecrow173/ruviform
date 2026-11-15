---
id: "TSPEC-034Y4fXVmWB9YKK3r9ty7n"
type: "test-spec"
status: "approved"
title: "Issue #630 test_stateful slice — test specification"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-stateful.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-test-stateful.md"
  implements:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-stateful.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "evidence/rf630/test-stateful-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-stateful.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-630-test-stateful.md"
    - "docs/designs/issue-630-test-stateful.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 test_stateful slice — test specification

## Purpose

`issue-630-adapter-state-bridge` の production state bridge
（VST3 `IBStream` 往復 / CLAP `clap.ext.state`）を、production
adapter 内部を再利用しない独立 fixture plugin が実 ABI で
save → mutate → restore → save round-trip を検証できるようにする。
本 spec は `test_stateful` plugin class が満たすべき oracle として
残し、後続 slice の fixture 拡張時にも参照される。

## Feature

**Stateful test plugin round-trip** — `test_stateful` class の
fixture plugin が、production host bridge 経由で state blob を
save/restore でき、restore 後の値が DSP を駆動し、malformed blob
を defined rejection で拒否する。

## Rules

- **R1 — State blob law**: state blob は `[u32 LE magic
  "RFST"][u32 LE version=1][f64 LE gain]` の 16 byte。VST3 は
  component payload = plain gain、controller payload =
  normalized gain（`plain = normalized × 2`）。CLAP は plain gain
  を framing なしで write/read する。
- **R2 — Deterministic serialization**: 同一 gain に対する blob は
  byte-exact（`f64::to_le_bits` → `u64::to_le_bytes`）。
- **R3 — Defined rejection**: 長さ不一致・magic 不一致・version
  不一致・非有限値は reject。有限の range 外値は `[0.0, 2.0]` に
  clamp。CLAP は trailing bytes も reject。
- **R4 — Persistent-state law**: `test_stateful` は activation で
  mirror を reset しない（`test_gain` の per-activation reset
  law を継承しない）。restore → activate → process で restored
  gain が DSP を駆動する。
- **R5 — Stateless decline**: `test_gain`/`test_fault`/`test_editor`
  は VST3 `kNotImplemented`、CLAP `HostError::Unsupported` を
  維持する。
- **R6 — Instance survival**: malformed blob reject 後も instance
  は生存し `SaveState` が有効 blob を返す。
- **R7 — RT boundary**: state verb は non-RT 面（VST3
  component/controller、CLAP main-thread `PluginStateImpl`）にのみ
  存在し、`process`/`clap_process` は allocation-free・lock-free・
  wait-free を維持する。
- **R8 — L1 ordering**: L1 E2E の state verb は delivered audio
  observation（distinct gain 1.0/0.75/1.75 が pairwise-distinct
  output を生む）の後にのみ送出する — async data plane の
  lookahead 経由で mutation event block が state verb を追い越す
  race を排除する。

## Examples

- **E1 — VST3 round-trip**: Given `Vst3Library` で
  `test_stateful` instance、When gain=1.0 で save → mutate to
  0.75 → save → mutate to 1.75 → save → restore initial → save、
  Then 初期 blob と復元後 blob が byte-exact、復元後 DSP は
  gain=1.0 を駆動する。
- **E2 — VST3 malformed**: Given save 済み instance、When
  truncated/trailing/corrupted-magic/NaN blob を restore、Then
  `HostError::PluginFailed`、instance 生存。
- **E3 — CLAP round-trip**: Given `ClapLibrary` で
  `test_stateful` instance、When save → mutate → restore → save、
  Then blob byte-exact、activation 後 DSP が restored gain を
  駆動。
- **E4 — CLAP malformed**: truncated/trailing/bad-magic/NaN blob
  は `HostError::PluginFailed`、instance 生存。
- **E5 — Stateless decline**: `test_gain`/`test_fault`/
  `test_editor` は state verb で VST3 `kNotImplemented`、CLAP
  `HostError::Unsupported` を返す。
- **E6 — L1 VST3 round-trip**: out-of-process runner 経由で
  descriptor handshake → process → save → mutate → save →
  mutate → save → restore → save byte-exact → audio が
  restored gain を証明 → malformed reject → teardown。
- **E7 — L1 CLAP round-trip**: 同上、CLAP descriptor/id/16-byte
  blob で実走。

## Guarantees

- VST3 factory `countClasses` = 8、CLAP `plugin_count` = 3 を
  in-process host-path test が assert する。
- `test_stateful` descriptor（VST3: component CID-derived UID、
  name `test_stateful`、vendor `Ruviform Test`、version `0.0.0`、
  feature `Fx`；CLAP:
  `org.ruviform-test.clap-test-stateful`）が production discovery
  で独立に観測される。
- in-process host-path test が production `Vst3Library`/
  `ClapLibrary` ABI 経由の round-trip + malformed + stateless
  decline を全て pass する。
- L1 E2E test が `l1-runner` feature + `#[ignore]` で gate され、
  実走で pass する。missing artifact は assert-fail（silent skip
  しない）。
- `rt_` filter test が `process`/`clap_process` の
  allocation-free を継続証明する。

## Non-goals

- preset/bank・GUI state・foreign format migration・commercial
  plugin integration。
- production adapter crate の変更。
- `test_instrument` / CLAP `test_editor` class（後続 slice）。
- 外部 SDK validator。

## Risk

- **L1 ordering race**: async data plane の lookahead + fire-and-
  forget `process` submit で state verb が mutation event block
  を追い越し得る → R8 の delivered-audio gate で排除。これが
  drop されると flaky/misleading な round-trip failure を招く。
- **VST3 component/controller domain ずれ**: component=plain /
  controller=normalized の変換が `setComponentState` で正しく
  行われないと restore 後の DSP gain がずれる → in-process
  COM-level test + host-path round-trip で固定。
- **CLAP `load` の trailing bytes**: host bridge は raw bytes を
  そのまま渡すため plugin 側で exact-length を要求しないと
  不正 blob を silent accept するリスク → R3 で defined
  rejection を固定。
