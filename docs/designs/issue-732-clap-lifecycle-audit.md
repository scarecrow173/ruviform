---
id: "DESIGN-732clapLifecycleAudit03"
type: "design"
status: "approved"
title: "Issue #732 production CLAP adapter lifecycle 監査 — 設計"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-732-clap-lifecycle-audit.md"
    - "docs/discovery/issue-732-clap-lifecycle-audit.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-732-clap-lifecycle-audit.md"
  implements:
    - "docs/specs/issue-732-clap-lifecycle-audit.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-732-clap-lifecycle-audit.md"
    - "docs/discovery/issue-732-clap-lifecycle-audit.md"
  blocks:
    - "docs/plans/issue-732-clap-lifecycle-audit.md"
  supersedes: []
---

# Issue #732 設計 — production CLAP adapter lifecycle 監査

## 設計方針

監査は「変更を前提としない照合」を基本とし、欠陥発見時のみ TDD
red-green で修正する。rf#722 の 3 パターンを CLAP 契約へ写像して
照合し、各判定に根拠行（コード行 + CLAP 仕様条項 + clack binding
行為）を対応付ける。結論は `evidence/rf732/lifecycle-audit.md` に
記録する。

## 監査手順

1. **binding 層の契約確認**: clack 0.2.0 `PluginWrapper` の状態機械
   （`INITIALIZING`/`INITIALIZED`/active processor cell）と raw C
   callback の対応を crates.io ソースで確認し、host の不正呼び出し
   を binding が弾く範囲を確定する。
2. **terminalization 照合**: CLAP `destroy` は instance 終端。
   adapter の `Shared::drop` → `LifecycleEvent::Destroy` →
   `Destroyed`（終端・全 event 拒否）が契約と整合するか、
   deactivate なし destroy・二重 destroy・destroy 後 callback の
   受理/拒否を clack 経路込みで確認する。
3. **受理 phase 照合**: 各 callback の遷移要求（`Activate`/
   `StartProcessing`/`StopProcessing`/`Process`/`Deactivate`）と
   `reset` の無遷移設計が、CLAP 仕様の `[thread & state]` 条件と
   中立機械の遷移表（`lifecycle.rs`:13-24）の両方に整合するか確認。
4. **bus/port info 自己整合**: `declare_extensions` が宣言する
   `PluginAudioPorts` + `PluginParams` と、`process` 経路の
   port/buffer/frames/events 受理条件の一致、latency/tail 非宣言の
   妥当性、process status 写像（`map_process_result`）の正しさを
   確認。
5. **host.rs 対称性確認**（read-only）: host 側 bridge の
   status/latency/tail 写像が plugin 側と対称かを確認（変更はしない）。

## validator lane 拡張設計（`ci/plugin-abi-validator/run.mjs`）

現行 lane は `CLAP_PLUGINS`（rf#630 fixture 5 plugin id）を単一の
`--clap-dll` に対して実行する。production adapter は別 cdylib
（`ruviform_plugin_clap.dll`・id `org.ruviform.clap-gain-poc`）で
あり、fixture id を production dll に流用すると NotFound noise に
なるため、plugin id と dll を組で指定する経路が必要。

設計判断: **opt-in の第 2 CLAP target** として `--clap-production`
（+ `--clap-production-dll` / `CLAP_PRODUCTION_DLL`）を追加する。

- 既定 lane の契約（fixture 5 legs + informational fault leg + VST3
  leg）を一切変更しない — rf#630/#726 が共有する file のため構造変更
  を避け、`CLAP_PLUGINS` の dll 別 profile 化（選択肢 b）は採用しない。
- CLAP legs 節を「target ごとの leg set」ループに一般化するが、
  guard 順序（tool 不在 → artifact 不在 → build 失敗 → 実走）と
  informational leg の fixture 限定を維持し、既定経路の leg 順・
  出力を不変に保つ。
- `buildFixtureArtifacts` の build 対象に `ruviform-plugin-clap` を
  `--clap-production` 指定時のみ追加（`--vst3-package` 同型の
  条件付き package 追加）。
- evidence 出力は `clap-<plugin.name>.txt` の既存規約に従い、
  production は `clap-production_gain.txt` で衝突しない。
- `report.artifacts.clapProductionDll` に production dll の
  path/sha256（未指定時は `null`）を記録。
- fail-closed 契約は不変: tool/artifact/build 不在は NotRun、
  validator 異常終了は Unsupported/Failed のまま。

却下した代替案:

- `--clap-plugin-id` 反復指定: fixture suite との併用時に dll 帰属が
  曖昧になり、leg の artifact 帰属を run.json で説明しにくい。
- `CLAP_PLUGINS` への production entry 常時追加: 既定 lane の leg
  数・build 対象が変わり、rf#630/#726 の共有契約へ無断の影響を与える。

## 欠陥発見時の修正方針（条件付き）

adapter 層での修正を優先（rf#722 先例: fixture と同じく adapter 層で
Lifecycle をリセットし core は不変更）。unit test は
`plugin.rs` `#[cfg(test)] mod tests`（`activated` helper 等）と
`adapt.rs` `assert_chain` を雛形に、違反した契約を直接検証する
red test → 修正 → green の順で行う。core 変更は共有機械自体の欠陥が
確定した場合のみ（条件付き lease）。

## evidence 記録様式

`evidence/rf732/lifecycle-audit.md` に rf#722 と同型の照合表
（パターン × 判定 × 根拠行 × CLAP 契約根拠）と validator 実走結果
（コマンド・exit code・counts）を記録する。`evidence/rf732/
abi-validator*/` に run.json と leg 出力を保存する。
