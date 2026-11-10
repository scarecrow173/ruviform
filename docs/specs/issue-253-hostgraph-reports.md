---
id: "SPEC-034Vj1oEP5t1a8EwNeZV4L"
type: "spec"
status: "approved"
title: "Issue #253 plugin→host report の host-graph consumer 配線 slice"
created: "2026-11-10"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-253-os-midi-backend.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/designs/engine-plugin-host-integration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
  derived-by:
    - "docs/designs/issue-253-hostgraph-reports.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-hostgraph-reports.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/pull/555"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 plugin→host report の host-graph consumer 配線 slice

## 背景とスコープ

PR #555（`641f49dd`、VST3 MIDI 正規経路 slice）で plugin→host report の
中立契約 `PluginProcessor::drain_output_reports`（pull 型・非 RT）と
VST3 adapter 側の emit/収集経路が merge 済みである。しかし report が
plugin の bounded pending buffer に滞留したまま、host/graph 側の
consumer が canonical `Event` を引き出す面は未配線だった — これが
Issue #253 に残された本 slice の対象行
「plugin→host report の host-graph 到達経路」である。

対象:

- `ruviform-plugin-host` に、consumer が opt-in で canonical `Event`
  report を pull 取得できる非 RT の drain API を追加する
  （`PluginHost` / `InProcessHost` の双方、`BusReport` と同じ
  honest `Reported`/`Unavailable` 系の応答形）。
- L0（in-process）の `Plain`/`Shared` 両スロットで drain が機能する
  こと。`Shared` スロットは endpoint attach 中でも host 側 drain が
  成立すること（endpoint の `try_lock` RT leg と host verb の
  `core.lock()` が同一セルを共有する既存 protocol の上に乗る）。
- L1（out-of-process）スロットと video スロットは wire verb を持た
  ないため `Unavailable` の defined 応答 — report 対応を偽装しない。
- headless fixture による E2E: fixture processor が `process` で
  report を生成 → host drain → consumer が `Event` を取得、までを
  `plugin_graph_e2e` 系の mock device/session 経路で証明する。

対象外（Issue #253 に残る・本 slice では Pass に繰り上げない）:

- L1 wire protocol への report verb 追加と runner 側 emit
  （`ruviform-plugin-host/src/l1`・`ruviform-plugin-core/src/wire.rs`
  の wire 語彙拡張は別 slice — 本 slice は `Unavailable` を返す）
- adapter（VST3/CLAP/OFX）側の emit 経路変更 — PR #555 の実装を
  そのまま使う
- `ruviform-audio-plugin-bridge` の capability 行列の変更
  （`param_reporting` は本 slice でも `Declared` のまま — 実 plugin /
  実 DAW 往復が未検証のため。判断根拠は design/IR に記録）
- Synvelis/AudioLens 側の consumer 実装（利用は別 Issue）
- 実 OS MIDI backend の残 backend、opt-in 実デバイス lane、
  実 DAW VST3 往復、macOS/Linux MIDI backend
- `ruviform-graph` への変更 — graph IR は plugin/RT 非依存の純粋な
  型層であり、consumer 面は host 側に置く（依存方向の根拠は design
  doc に記録）

## 要件

### R1 consumer drain API（host 側・pull 型・非 RT）

- `PluginHost` と `InProcessHost` に、handle と caller 所有の
  `&mut Vec<Event>` を取り、since-last-drain の report を append
  する drain メソッドを追加する。名前・形は
  `PluginProcessor::drain_output_reports` に揃える。
- 応答は `buses`/`BusReport` と同型の honest split:
  drain が plugin に到達した場合と、層が report を供給できない
  場合（L1 audio/video スロット）を区別する。後者は error ではなく
  defined な `Unavailable` 値 — handle 自体は有効。
- RT path（`process`/`PluginEndpoint::process`）からの暗黙 drain・
  blocking・allocation を一切追加しない。drain は consumer が明示的
  に呼ぶ pull のみで、opt-in は「呼ばなければ report は plugin の
  bounded pending buffer に滞留し、capacity 超過分は producer 側で
  defined drop + 計数される」という既存契約のまま成立する。
- `Shared` スロットの drain は既存の `shared_verb` 三層 lock
  protocol（mirror pre-check → `core.lock()` → in-CS mirror
  re-read → `Failed` 収束）に乗せ、endpoint attach 中でも
  `try_lock` 競合なく直列化される。`Failed` 観測 union・poisoned
  lock の収束は他の `Shared` verb と同一。
- `Plain` スロットは `buses` と同じく lifecycle gate なしで plugin
  の drain を直接呼ぶ。
- drain は繰り返し呼べる: 空 drain は `out` 不変の `Drained`、
  destroy 済み/stale handle は `Err(InvalidHandle)`。
- report の意味論は plugin-core 契約のまま: canonical `Event`
  （`ParamValue`/`Midi`）、`frame_offset` は block 内 frame offset、
  複数 block 分が連結される場合の block 境界は consumer 側の drain
  周期で解釈する（host は再解釈しない）。

### R2 正直な capability 応答

- L1 audio スロット・L1 video スロットの drain は
  `Unavailable` を返す — wire に report verb が無い事実を隠さない。
- `Reported`/`Drained` を返せるのは L0 スロットのみであり、
  それは「この host 層が canonical report を consumer に届けられる」
  ことを意味し、adapter 個別の emit 実装や実機検証を含意しない。

### R3 検証と記録

- headless fixture E2E: `Send` fixture processor が `process` ごとに
  canonical report を pending に積み、`prepare_endpoint` → graph
  node → mock device/session の callback を介して RT leg を駆動した
  後、host 側 drain で `Event` が取得できることを証明する
  （endpoint attach 中の Shared スロット経路を含む）。
- unit: `Plain`/`Shared` 両スロット、空 drain、連続 drain、
  `InvalidHandle`、`Failed` 収束、opt-in（drain しなければ
  report は consumer に見えない）を検証する。
- `cap.rs`（`ruviform-audio-plugin-bridge`、編集範囲外）は変更
  しない — `param_reporting` の `Declared` 維持とその根拠を
  design/IR に記録する。
- impl record に API 形状の判断根拠・検証結果・残 NotRun を記録する。

## 受入条件（本 slice）

- `PluginHost::drain_output_reports` / `InProcessHost::
  drain_output_reports` が存在し、L0 `Plain`/`Shared` スロットで
  canonical `Event` を caller の `Vec` に append できる。
- endpoint attach 中の `Shared` スロットで、graph 駆動の RT leg が
  生成した report を host 側 drain が取得できる E2E が pass する。
- L1 audio/video スロットは `Unavailable`、stale/unknown handle は
  `InvalidHandle`、`Failed` 収束は `Err(Failed)` を返す。
- drain は非 RT 契約（caller 所有 vec への append、blocking lock
  許容）を doc で明示し、RT path に一切の drain/blocking を追加
  しない。
- `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` /
  `lint-l1-real-plugin` / `test-l1-real-plugin` が実行・記録される。
