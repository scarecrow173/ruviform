---
id: "SPEC-630faultModes01"
type: "spec"
status: "approved"
title: "Issue #630 fault-modes slice — test_fault 非致命 fault mode 拡張 spec"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-fault.md"
  changes:
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/src/fault.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/fault.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/tests/l1_fault_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/l1_fault_test_plugin_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-vst3/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/README.md"
    added:
      - type: "file"
        path: "docs/specs/issue-630-fault-modes.md"
      - type: "file"
        path: "docs/designs/issue-630-fault-modes.md"
      - type: "file"
        path: "docs/test-specs/issue-630-fault-modes.md"
      - type: "file"
        path: "evidence/rf630/fault-modes-verification.md"
  implements:
    - "docs/specs/issue-630-test-fault.md"
  implemented-by:
    - "docs/designs/issue-630-fault-modes.md"
  depends-on:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-fault.md"
  related:
    - "docs/specs/issue-630-test-fault.md"
    - "docs/designs/issue-630-test-fault.md"
    - "docs/test-specs/issue-630-test-fault.md"
    - "evidence/rf630/fault-modes-verification.md"
---

# Issue #630 fault-modes slice — spec

## 目的

`test_fault` fixture の非致命 fault 面を拡張する。既存の 5 mode
(`none`/`abort`/`exit`/`hang`/`error`) は process 生死と defined
rejection だけを覆い、host が isolate/recover すべき残る故障種
(RT contract 違反・時刻/タイミングの虚偽・不正な出力状態) が
fixture 側で再現不能のままだった。本 slice は `Fault` param を
plain `[0.0, 8.0]` の 8 mode に拡張し、`rt_alloc` / `bad_time` /
`invalid` を実 ABI 越しに観測可能にする。

## スコープ

`crates/ruviform-plugin-test-{vst3,clap}` の `src/fault.rs` と
その test 面のみ。production crate (`ruviform-plugin-{vst3,clap,
host,export,core}`) は zero-diff。arming channel は既存どおり
in-band `ParamValue` event のみ — 新規 param・env・argv 経路は
追加しない (`ParamController` が single-param のため第 2 param
追加は本 slice の境界外)。

## Fault law 変更（両 format 共通）

`Fault` param: `ParamId(0)`、name `"Fault"`、plain `[0.0, 8.0]`、
default `0.0`、stepped。mode decode は `floor(plain)` を
`[0, 8]` に clamp した純粋関数 `decode_fault`（既存契約を帯域
追加のみで拡張）。

| plain | mode | 振る舞い |
| ----- | ---- | -------- |
| `[0.0, 1.0)` | `none` | pass-through（default・完全 inert） |
| `[1.0, 2.0)` | `abort` | `std::process::abort()` |
| `[2.0, 3.0)` | `exit` | `std::process::exit(round((plain-2)*100))` |
| `[3.0, 4.0)` | `hang` | 応答しない wait（RT 内・復帰しない前提） |
| `[4.0, 5.0)` | `error` | `ProcessStatus::Error` — defined rejection |
| `[5.0, 6.0)` | `rt_alloc` | `process()` 内で bounded heap round-trip、先頭出力 sample に `200.0` marker |
| `[6.0, 7.0)` | `bad_time` | `Continue + TAIL_INFINITE` を返す timing の虚偽 |
| `[7.0, 8.0]` | `invalid` | 全出力 channel/sample を `NaN` で fill |

`plain = normalized × 8.0` なので normalized arm 値は
`abort=0.125` / `exit(42)=0.3025` / `exit(0)=0.25` / `hang=0.375` /
`error=0.5` / `rt_alloc=0.625` / `bad_time=0.75` / `invalid=0.875`。

## 受入条件（slice 確定版）

1. `Fault` descriptor が両 ABI で plain `[0.0, 8.0]`・stepped を
   報告する（VST3 `stepCount = 7`）。既存 5 mode の semantics・
   last-event-wins・cross-block persistence は不変。
2. `rt_alloc` armed block が `process()` 内で bounded 確保
   （`vec![0xC8; 4096]`、最適化不能）を実行し、先頭出力 sample
   `200.0` の marker で heap round-trip を証明する。残りの
   出力は pass-through。allocator 越境は観測不能のため marker
   が honest oracle — `assert_no_alloc` で別 dylib 内確保を
   捕まえる試みは行わない。
3. `bad_time` armed block が `Continue + TAIL_INFINITE` を返し、
   VST3 `getTailSamples()` / CLAP `Tail` status 経由で wire 上
   `tail_frames == u32::MAX` として観測される。disarm で
   `tail_frames == 0` に回復。
4. `invalid` armed block が全出力 sample を `NaN` で埋め、
   instance は `Processing` を維持（defined nonfatal）。disarm
   で clean pass-through に回復。
5. `rt_alloc`/`bad_time`/`invalid`/`error`/`none` の in-process
   unit/host_path test と、L1 out-of-process e2e（`#[ignore]` +
   `l1-runner`）が両 format で揃う。`abort`/`exit`/`hang` は
   in-process では絶対に踏まない既存 RT 境界を維持。
6. 既存 lane は緑のまま — normalized arm 値の rescale は全
   test/doc に波及し、既存 fault case は回帰しない。

## 振る舞い契約

- 非致命 4 mode（`error`/`rt_alloc`/`bad_time`/`invalid`）は
  child を殺さない — arm → 観測 → `none` disarm → pass-through
  回復を同一 instance で辿れる。
- `rt_alloc` の marker は fixture が自発的に報告する heap
  round-trip 証拠であり、host 側の allocator 検出を装わない。
- `bad_time` は neutral `ProcessResult.tail_frames` を
  `TAIL_INFINITE` にして返すだけ — 実際の tail は存在しない
  pass-through fixture が「無限 tail を主張する」時点が fault。
- `invalid` の NaN は audio 出力面の不正状態注入 — status は
  `Continue` のまま（出力と status が矛盾する状態）。
- CLAP 側のみ追加で `events.output` へ malformed event leg を
  持つ: `bad_time` は block 境界を超えた時刻の `ParamValue`、
  `invalid` は NaN 値の `ParamValue` を push する。ruviform v1
  host bridge は plugin 出力 event を `void` に捨てるため観測
  対象は fixture ABI の発行面であり、push 失敗は無視する
  （honest record — VST3 側に対応 leg は存在しない）。

## 検証要求

- in-crate unit test: 全 8 帯域の `decode_fault`、新 3 mode の
  processor semantics、`none` disarm 回復、
  `rt_unarmed_block_is_alloc_free`（inert 経路の
  `assert_no_alloc` — 計測窓外に setup を置く）。
- `tests/host_path.rs`: descriptor `max_value == 8.0`、
  新 3 mode の ABI 実走観測（marker / tail / NaN）+ disarm
  回復。`error` は normalized `0.5`。
- `tests/l1_fault_test_plugin_e2e.rs`: 新 3 mode を out-of-process
  で arm → 観測（`out_left[0] == 200.0` / `tail_frames ==
  u32::MAX` / 全 sample NaN）→ `Processing` 維持 → disarm 回復 →
  clean teardown。armed submit は既存先例どおり繰り返す。
- deterministic evidence と real-run evidence を分離して記録。
- 既存 `test-l1-real-plugin` / `test-rt` / lint / fmt / prek lane。

## 非目標

- stateful migration/checksum/degraded 挙動、editor popup 挙動。
- real dock/DComp gate、consumer oracle 照合。
- `assert_no_alloc` による fixture dylib 内確保の直接検出
  （allocator 境界を越えて観測不能 — marker が受入 oracle）。
- output event injection の host 側観測（v1 bridge が `void`
  のため到達しない — CLAP fixture ABI の発行面までが本 slice）。
- production crate 変更、`ParamController` の複数 param 化。
- docs index（`docs/**/README.md`）登録 — Coordinator が統合時に
  整備。
- PR は `Refs #630` — Issue は後続 slice のため開いたまま。
