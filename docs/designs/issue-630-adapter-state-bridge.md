---
id: "DESIGN-P7XCcqW3rGHPzKfQJ6SUA"
type: "design"
status: "approved"
title: "Issue #630 adapter-state-bridge slice — 設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-adapter-state-bridge.md"
  implements:
    - "docs/specs/issue-630-adapter-state-bridge.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  blocks:
    - "docs/plans/issue-630-adapter-state-bridge.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-630-adapter-state-bridge.md"
    - "docs/plans/issue-630-adapter-state-bridge.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-630-adapter-state-bridge.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #630 adapter-state-bridge slice — 設計

## Blob contract (両 format 共通 law)

中立 `save_state` が返す `Vec<u8>` は opaque bytes — 中身の解釈は
adapter 側の責務。VST3 adapter は component/controller の 2 系統の
state を 1 blob に収めるため、以下の framing を採用する:

```
[u32 LE comp_len][comp bytes][u32 LE ctrl_len][ctrl bytes]
```

- `comp_len`/`ctrl_len` は `u32` little-endian 固定。CLAP 側は
  framing を持たず plugin 定義の生 byte stream をそのまま返す
  （CLAP `clap.state` は単一 stream — format 間で blob を
  相互運用しない。中立層は opaque なので問題にならない）。
- single-component plugin（component が自身の CID を
  `getControllerClassId` で報告）では controller 側呼び出しを
  行わず `ctrl_len = 0`。
- restore は framing を parse し、長不足・`comp_len+ctrl_len` が
  blob 末尾と一致しない等の malformed 入力を
  `Err(HostError::PluginFailed)` で拒否する。wire 側の
  `WIRE_MAX_STATE_BYTES`（16 MiB）が上限を既に保証するため、
  adapter 独自の cap は設けない（KISS）。
- **foreign blob の扱い（実装時に確定した調整）**: 本 framing が
  生成し得る最小長は 8 byte（2 つの `u32` length header、payload
  空）。8 byte 未満の blob は本 adapter の serialization では
  あり得ないため「malformed frame」ではなく **foreign state**
  として扱い、blob 全体を component payload・ctrl を空として
  plugin の state 呼び出し列へそのまま渡す。plugin が自身の
  state 形式を検証する境界であり、stateless fixture の
  `kNotImplemented` → `Unsupported` という L1 e2e の pin
  （`RestoreState{bytes:[0xAA,0xBB]}`）を守るために必要な
  調整 — 厳密 unframe 先行では 2 byte blob が plugin 呼び出しに
  到達せず `PluginFailed` となり oracle を破る。8 byte 以上で
  parse に失敗する blob は従来どおり `PluginFailed`。

## VST3: `BridgeStream`（`host.rs` 新設）

`IBStreamTrait` を実装する `ComWrapper` class
（`BridgePlugFrame`/`BridgeAttributeList` と同じ
`impl Class` + `&self` + `RefCell` interior mutability 慣行）。
状態は `RefCell<BridgeStreamState>` 1 つに集約
（`data: Vec<u8>` + `pos: u64`）— `read`/`write`/`seek` が
bytes と位置を同時に触るため単一 borrow で矛盾を排除する。

| verb | 契約 |
| --- | --- |
| `read(buf, n, *read)` | `pos` から `min(n, len-pos)` byte を copy、`pos` 前進、実読数を `*read` に正直に報告（EOF での部分読みは正常）。`n<0`・`buf` null（`n>0` 時）→ `kInvalidArgument`。out-ptr null は許容（書かないだけ）。 |
| `write(buf, n, *written)` | `pos` が `len` を超える場合は 0 埋めで gap を伸長してから `n` byte を書き、実書数を `*written` に報告。`n<0`・`buf` null（`n>0` 時）→ `kInvalidArgument`。plugin からの任意長書き込みは `Vec` 伸長で受け付け、上限は wire の `WIRE_MAX_STATE_BYTES` に委ねる（design 判断 — 下記）。 |
| `seek(pos, mode, *result)` | mode は `kIBSeekSet`(0)/`kIBSeekCur`(1)/`kIBSeekEnd`(2) のみ — その他は `kInvalidArgument`。計算結果が負または `i64` overflow は `kResultFalse`（defined refusal）。**`len` 超過への seek は許可**する — file-like 意味論で、読みは EOF・書きは 0 埋め伸長。実プラグインが save 中に前方 seek→後方 patch する実装を拒否しないため（SDK `MemStream` 慣行に倣う）。成功時は新位置を `*result` に報告。 |
| `tell(*pos)` | 現在位置を `*pos` に報告。out-ptr null 許容。 |

**RT 境界**: `BridgeStream` は `save_state`/`restore_state` 呼び出し
の中でだけ生成・破棄される non-RT オブジェクト。`Vec` 伸長・
`RefCell` borrow・確保は全て state 呼び出し時に限定され、
`process` 経路には一切載らない（`rt_` テスト不変条件を侵さない）。

## VST3: state 呼び出し列と error mapping

`Vst3Plugin::{save_state,restore_state}`（`impl Plugin`、
`open_gui` と同じ phase gate — `phase < Phase::Initialized` は
`Err(PluginFailed)`、`component`/`controller` の `None` は
defensive `PluginFailed`）:

- **save**: `component.getState(stream)` →
  `!single_component` のときのみ `controller.getState(stream2)`
  （single-component で二度呼ぶと二重シリアライズになる）。
  各 stream の内容を framing して返す。
- **restore**（VST3 spec の順序厳守）:
  `component.setState(comp_stream)` → `component.getState(norm_stream)`
  → `!single_component` のとき `controller.setComponentState(norm_stream を seek(0) して渡す)`
  → `controller.setState(ctrl_stream)`。
  `component.getState` の再シリアライズは controller に渡す
  *正規化済み* component state を得る VST3 SDK の hand-off
  パターンであり、single-component でも component 側の一部と
  して実行する（ctrl 側呼び出しのみ省略）。
- **tresult → HostError mapping（hard constraint）**:
  `kNotImplemented` → `Err(HostError::Unsupported)`（能力欠如の
  正直な decline — 既存 L1 e2e が init 済み stateless fixture に
  `Err(Unsupported)` を pin しており、fixture の
  `setState`/`getState` は `kNotImplemented` を返す）、
  その他の非 `kResultOk` → `Err(HostError::PluginFailed)`、
  8 byte 以上で parse 不能な corrupt frame →
  `Err(HostError::PluginFailed)`、8 byte 未満の foreign blob は
  plugin へ委譲（plugin の tresult がそのまま結果になる）。

実装形: COM call 列は component/controller の `ComPtr` を取る
free function（`save_component_state`/`restore_component_state`
相当）に切り出し、`Vst3Plugin` の method は phase gate + field
取り出し + framing の薄い殻とする。test は `impl Class` の
test-only stateful double（`IComponent`/`IEditController` を実
装し `IBStream` を `ComRef::from_raw` で借用して read/write
する）を `ComWrapper` 経由で駆動し、実 vtable dispatch を通した
round-trip を検証する — fixture cdylib を変更せずに済む
（`component.rs` test の `TestQueue`/`TestChanges` 先例）。

## CLAP: `clap.ext.state` 橋渡し

- `Cargo.toml`: `host` feature に `"clack-extensions/state"` を
  1 行追加（既存 non-optional 依存への feature flag のみ —
  `clack-extensions` 側 `state = []` は additive gate、
  `Cargo.lock` 差分なし想定）。
- `ClapPlugin::{save_state,restore_state}`:
  `instance=None` → `Err(PluginFailed)`（`open_gui` と同じ
  bad-state convention）→ `handle.get_extension::<PluginState>()`
  が `None` → `Err(Unsupported)`（`PluginGui` パターンそのまま）
  → `state.save(&handle, &mut Cursor<Vec<u8>>)` /
  `state.load(&handle, &mut Cursor<&[u8]>)`。
  `StateError` → `Err(PluginFailed)`。
- host 側 `HostState`/`mark_dirty` は登録しない
  （`BridgeHandlers::declare_extensions` は no-op のまま）。
  plugin 駆動の dirty 通知は neutral contract に対応する
  surface が無く、後続 slice の判断材料として本 design に
  記録するに留める。
- blob は plugin 定義の生 byte stream をそのまま中継する
  （opaque — adapter は framing を被せない）。

## Side finding: `controller.rs` の不正直 stub

fixture `GainController::setState`/`getState` は `kResultOk` を
返しながら stream を一切読み書きしない空 no-op — host が
controller state を要求すると成功を偽装する。`component.rs` の
`GainComponent::setState`/`getState`（既に正直な
`kNotImplemented`）と対称に `kNotImplemented` へ修正し、
`:78-80` のコメントを整合させる。`setComponentState` は既に
`kNotImplemented`。test に `kNotImplemented` assertion を追加
（`component.rs:1752-1755` の stub 検証の mirror）。

## RT / lifecycle 境界

- `save_state`/`restore_state` は non-RT verb
  （`Plugin` contract 明記）— `BridgeStream`・`Cursor`・blob
  `Vec` の確保は全てこの呼び出し内に限定。`process` の
  alloc/lock/wait-free 不変条件は不変。
- phase gate は `phase >= Initialized` 以降（`Initialized`/
  `Activated`/`Processing` 全て許可 — VST3 spec は initialize
  後の任意状態での state 呼び出しを認める）。CLAP は
  `instance.is_some()` のみを条件とし、activated/processing 中の
  呼び出しも plugin 側 `clap.state` の可否に委ねる。
- 既存 L1 e2e（lease 外 oracle）は init 済み stateless fixture
  への `SaveState`/`RestoreState` が `Err(Unsupported)` で
  返ることを pin — 本実装はその contract を維持する
  （VST3: `kNotImplemented` → `Unsupported`、CLAP: extension
  無し → `Unsupported`）。

## 却下した代替案

- **VST3 state を component 側のみに限定**: 却下 — distributed
  plugin の controller state（GUI 永続値等）を落とすため
  VST3 spec の 4 呼び出し列が必要。
- **CLAP host 側 `HostState` 登録**: 却下（KISS）— save/load は
  host extension 宣言なしで成立し、`mark_dirty` の neutral
  contract 対応は未定義。登録すると `MainThread=()` に
  `HostStateImpl` 実装が必要になり diff が増える。
- **adapter 独自の blob cap**: 却下 — wire の
  `WIRE_MAX_STATE_BYTES` が既に上限。二重 cap は policy の
  分散。
- **seek を `[0, len]` に制限**: 却下 — save 中の前方 seek +
  後方 patch を行う実プラグインを拒否する互換性リスク。
  非負位置への seek は file-like に許可する。
- **restore で常に strict unframe（8 byte 未満も `PluginFailed`）**:
  却下 — lease 外の L1 e2e が stateless fixture への
  `RestoreState{bytes:[0xAA,0xBB]}` に `Err(Unsupported)` を
  pin しており、plugin 呼び出しへ到達しない `PluginFailed` は
  oracle を破る。8 byte 未満を foreign state として plugin へ
  委譲する方式で両立させた（上記「foreign blob の扱い」）。
