---
id: "DSC-732clapLifecycleAudit01"
type: "discovery"
status: "final"
title: "Issue #732 production CLAP adapter lifecycle 監査 — VST3同型欠陥の診断"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/732"
    - "https://github.com/scarecrow173/ruviform/issues/722"
    - "evidence/rf722/lifecycle-audit.md"
    - "docs/specs/issue-630-abi-validator.md"
  related:
    - "../specs/issue-732-clap-lifecycle-audit.md"
---

# Issue #732 production CLAP adapter lifecycle 監査

Issue #722（production VST3 adapter lifecycle audit・PR #731）で発見・
修正された 2 欠陥（`terminate` 非終端化・`setupProcessing` 受理過狭）と
同型の欠陥が production adapter `ruviform-plugin-clap`（clack 0.2.0 /
clap-sys 0.5.0・cdylib `ruviform_plugin_clap.dll`・plugin id
`org.ruviform.clap-gain-poc`）に存在するかを監査した記録。

## 監査対象と一次情報

- 対象: `crates/ruviform-plugin-clap/src/plugin.rs`（CLAP `Plugin` /
  `PluginAudioProcessor` 実装・extensions 宣言）・`src/adapt.rs`
  （CLAP ⇄ 中立 core の写像層）・`src/gain.rs`（`PluginProcessor`
  実装）・`src/host.rs`（`host` feature の host 側 bridge — read-only
  対象、報告経路の対称性確認のみ）。
- 照合正本: `crates/ruviform-plugin-core/src/lifecycle.rs`（共有中立
  Lifecycle 機械・read-only）と CLAP 仕様（`include/clap/plugin.h`
  の lifecycle 契約）・clack 0.2.0 の生存期間型。
- 先例: `evidence/rf722/lifecycle-audit.md` の 3 パターン照合様式
  （terminalization・受理 phase 過狭・bus info 自己整合）。

## rf#722 発見パターンの CLAP への写像

| パターン | VST3 (rf#722) | CLAP 対応観点 |
| --- | --- | --- |
| terminalization | `terminate` が `Destroyed` 終端のまま残し再 initialize 不可 → `Created` リセットで修正 | CLAP は instance ごとに `destroy` が終端（再 init 経路なし）。clack が init を instance 生存中 1 回に制限し、`destroy` で processor cell を解放するか |
| 受理 phase 過狭 | `setupProcessing` が `Activated` のみ受理 → `Initialized \| Activated` に拡大 | `activate`/`start_processing`/`stop_processing`/`process`/`reset`/`deactivate` の受理 state 集合が CLAP 契約の `[thread & state]` 条件と一致するか |
| bus info 自己整合 | bus 宣言と `setBusArrangements` 受理の整合 | audio-ports extension の port 宣言（数・channel count・flags・`in_place_pair`）と `process` の実振る舞いの一致、latency/tail 報告経路 |

## 監査で確認した構造的事実（実コード照合済み）

1. **clack 0.2.0 の生存期間型**: `PluginWrapper::activate` が
   `PluginAudioProcessor` を構築・保持し、`deactivate` が `take()` で
   消費、`PluginInstance::drop` が raw `destroy` を呼ぶ。
   `wrapper()` は `INITIALIZING`/`UNINITIALIZED` 状態の callback を
   `PluginCalledDuringInitialization` で拒否する。したがって host の
   不正な呼び出し順序（activate 前の process、deactivate 後の
   process、init 中の extension 照会）は binding 層で既に弾かれる。
2. **中立機械の終端契約**: `Destroyed` は終端で全 event を拒否
   （`lifecycle.rs`:21-24,55-63）。CLAP の `destroy` は instance の
   終端操作であり、VST3 の `terminate`（COM object 再利用を許す
   SDK 契約）とは意味が異なる — 終端維持は CLAP 契約と整合する。
3. **adapter の遷移写像**: `activate`→`Activate`（失敗は
   `ActivateFailed` で `Initialized` に留まり retry 可）、
   `start_processing`→`StartProcessing`（`Activated` のみ）、
   `stop_processing`→`StopProcessing`（`Processing` のみ）、
   `process`→`Process` self-loop ゲート（`Processing` のみ）、
   `deactivate`→`Deactivate`（`Activated` のみ = post-stop）、
   `reset` は遷移を起こさず mirror から gain を再同期
   （`[audio-thread & active]` で Activated/Processing 双方に合法）。
4. **error channel のない callback**（`stop_processing`/`deactivate`/
   `Shared::drop`）は拒否された遷移を記録するのみで状態機械を
   破壊しない — fail-closed。
5. **port/buffer 契約**: stereo 1 ペア宣言（`MAX_CHANNELS=2`・
   `in_place_pair`・`IS_MAIN` index 0）、f32 のみ
   （`SUPPORTS_64BITS` 非宣言 ↔ f64 pair 拒否）、pointer 検証
   （null/alignment/alias）→ in-place は mutable output-only view、
   `frames_count > max_frames` 拒否、`MAX_EVENTS_PER_BLOCK=256`。
6. **latency/tail**: latency/tail extension を宣言しない
   （zero-latency gain・tail を生成しない）。`map_process_result` は
   neutral `Continue + tail_frames > 0` → `ClapProcessStatus::Tail`、
   `Sleep` → `Sleep`、`Error` → `PluginError` に写像済み
   （`adapt.rs`:939-945・unit test `adapt.rs`:1907-1920）。
   CLAP `Tail` status は長さを持たないため有限 tail 長の情報は
   ABI 上失われるが、tail extension 未宣言の plugin が Tail を
   返すこと自体が不適切 — 現行 gain processor は `CONTINUE`/`SLEEP`
   のみ返し Tail を生成しないため実害なし。
7. **process status の戻り値**: `process` は `Result<ClapProcessStatus,
   PluginError>` を返し、clack が `CLAP_PROCESS_ERROR` に写像。
   status 写像の単体 test あり。

## 外部 validator による欠陥検出（コード照合では未検出）

コード照合だけでは発見できなかった 2 件の契約違反を、clap-validator
0.4.1 の production leg 実走（`--clap-production`）が検出した:

| # | 失敗 leg | 根本原因 | fixture との divergence |
| --- | --- | --- | --- |
| D1 | `param-set-*`/`param-fuzz-*`/`param-conversions`/`param-default-values`（9 件） | `plugin.rs` の `get_info` が `module: b"/"` を報告 — clap-validator は leading `/` の module 名を拒否 | fixture は全 plugin で `module: b""`（空 = root module） |
| D2 | `process-varying-sample-rates`（1 件） | `adapt.rs::to_process_config` が `rate.fract() != 0.0` で fractional sample rate を拒否 → 1234.57 Hz で `activate` が false | fixture の `to_process_config` は `rate.round()` で整数 Hz に丸めて受理 |

いずれも「受理 phase 過狭」パターンの CLAP 版 — fixture が
clap-validator 実走で鍛えられた契約に対し、production adapter が
写像層・metadata 層で divergence していた。中立 `SampleRate` が
整数 Hz である以上、fractional rate の扱い（reject vs round）は
adapter 層の契約判断であり、fixture の実績ある選択（round）に
揃えるのが正しい境界。

## 監査結論（詳細は evidence/rf732/lifecycle-audit.md）

- **terminalization・bus/port info・latency/tail・process status**:
  欠陥なし（CLAP の instance 終端 `destroy` は VST3 `terminate` と
  非対称で、終端維持は契約と整合）。
- **受理 phase（写像層の過剰拒否）**: 2 欠陥を発見・修正
  （D1: param module `/` → 空、D2: fractional sample rate reject →
  round）。TDD red（validator 10 failed + unit test）→ green
  （unit test + validator 再実走）で検証。
