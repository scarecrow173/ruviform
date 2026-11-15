---
id: "DESIGN-034Y3eit2mMvNAtCJxChw5"
type: "design"
status: "approved"
title: "Issue #628 close-out — detailed design"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-628-close-out.md"
    - "docs/designs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-deterministic-test-runtime-design.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-628-close-out.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-628-close-out.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-628-close-out.md"
    - "docs/designs/issue-628-scripted-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-628-close-out.md"
    - "docs/test-specs/issue-628-close-out.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-628-scripted-backend.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-628-close-out.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 close-out — detailed design

## Overview

close-out slice は 2 つのコード変更（`RunMetadata` JSON serialization・
`StreamFault::Cancellation`）と docs/evidence の簿記で構成される。
製品 crate の `src` は一切変更しない — cancellation 写像も
audio-test 内部で完結し、本番契約へ新 variant・新 API を追加しない
（publish=false test-only 境界不変）。

## 1. RunMetadata JSON serialization

### Derive 対象

- `metadata.rs`: `RunMetadata`, `RunParameters`, `CaseRequirement`,
  `CaseResult`, `VerificationLevel`, `EnvironmentKind`。
- `clock.rs`: `VirtualClockConfig`, `JitterSpec`（serde derive 追加のみ、
  挙動変更なし）。

### Wire format の選択

fieldless enum は `#[serde(rename_all = "kebab-case")]` で Case
Schema v1 語彙に揃える:

| enum | variant | wire |
| --- | --- | --- |
| `EnvironmentKind` | `DeterministicBackend`/`RealAbiTestPlugin`/`RealWindow` | `deterministic-backend`/`real-abi-test-plugin`/`real-window` — `as_str()` と一致 |
| `VerificationLevel` | `OsL1`/`GuiL2`/`ExtL3` | `os-l1`/`gui-l2`/`ext-l3` — POLICIES.md 語彙の kebab-case |
| `CaseRequirement` | `Required`/`Deferred`/`NotApplicable` | `required`/`deferred`/`not-applicable` |
| `CaseResult` | `Pass`/`Fail`/`NotRun`/`Unsupported` | `pass`/`fail`/`not-run`/`unsupported` |

選択理由: `EnvironmentKind::as_str` が既に kebab-case の wire spelling
を持ち、`rename_all = "kebab-case"` はそれと一致する。serde の
default（variant 名そのまま = PascalCase）を使うと Case Schema v1
語彙とずれた第 2 語彙を生やすことになるため採用しない。struct field
名は既存の snake_case のまま（serde default）。`#[serde(rename_all)]`
は container 単位で enum 全 variant に一律適用されるため、
`non_exhaustive` な将来 variant も同じ規則に従う。

`EnvironmentKind` は `as_str()` との一致を test で pin する
（`serde_json::to_value` の文字列が `as_str()` と等しいことを全
variant で検証）— 2 つの spelling が drift しないことを保証する。

### JSON API（`metadata.rs`、非 RT test-driver API）

```rust
impl RunMetadata {
    pub fn to_json(&self) -> serde_json::Result<String>;
    pub fn from_json(json: &str) -> serde_json::Result<Self>;
    pub fn save_json(&self, path: impl AsRef<Path>) -> std::io::Result<()>;
    pub fn load_json(path: impl AsRef<Path>) -> std::io::Result<Self>;
}
```

- `to_json`/`from_json` は serde_json の (de)serialize のみ。
- `save_json`/`load_json` は `std::fs` read/write + JSON codec。
  serde/IO error は `io::Error`（`ErrorKind::InvalidData`）へ写像する。
- **RT 契約**: これらは alloc・IO を伴う非 RT API。doc に明記し、
  `rt_` prefix テストにはしない。`rt_` 契約経路（callback firing・
  clock read・PCM fill・trace sink）へ割込み・alloc を持ち込まない。

### 依存追加

`Cargo.toml` に `serde = { version = "1", features = ["derive"] }`・
`serde_json = "1"` を理由コメント付きで追加（workspace 既使用:
`ruviform-io-structured`/`ruviform-ui-test`/`ruviform-observability`/
`ruviform-plugin-dock-test`）。`Cargo.lock` は再生成物として取り込む。

## 2. `StreamFault::Cancellation`

### 背景 — production 契約に cancel はない

`ruviform_audio_core::device` の `DeviceError`/`OutputStream`/
`InputStream`/error callback 契約に cancel 概念は存在しない。
Issue #628 の Fault Injector リストは `cancellation` を明示するが、
本番契約を拡張して cancel を発明することは禁止（test-only 境界）。

### 写像の解釈（採用案）

`StreamFault::Cancellation { at_callback }`:

- 当該 callback index で **data callback を発火しない**（in-flight
  delivery の中断 = cancel の核）。
- error callback に `DeviceError::Backend("cancelled")` を配送する。
  `Backend(&'static str)` は「他 variant に写像できない backend 失敗」
  の正規 bucket であり、分類名 `"cancelled"` は alloc なしの
  `&'static str` で既存契約内に収まる — 新 variant 不要。
- stream は終了する（`invalidated`/`dead` — `Invalidate` と同じ
  lifecycle）。後続 pump は `Err(StreamInvalidated)`、drop 後の
  再 open で reconnect 可能。
- `callback_index` は進む（cancel された firing も 1 回の発火）。
  `position`/delivered `frames` は進まない（0 frame 配送）。
- 同 index の他 fault（`Error` 等）の error 配送・mutation は通常通り
  適用され、cancellation は data callback skip + stream 終了を追加で
  行う。

`PumpReport` は既存 field で観測可能: `errors_fired`（cancel error
含む）、`invalidated: true`、`frames: 0`、`callback_index` =
cancel された index。`invalidated` field の doc を「`Invalidate`
または `Cancellation` で終了」と明確化する。

### 不採用案

- **error 通知のみで stream 継続**（`Error{Backend("cancelled")}` と
  同等）: 「中断」の意味を持たず、単なる error 注入と区別が付かない。
- **data callback 発火後に cancel**（`Error` fault と同じ順序）:
  in-flight 処理が完了してしまい cancel の意味が薄い。
- **`StreamInvalidated` を cancel の error として配送**: invalidation
  （device 除去等の backend 起因）と cancel（操作の中断）は原因が
  異なり、`Backend("cancelled")` の方が正直な分類。
- **MIDI 側（`backend/midi.rs`/`MockPort`）への拡張**: `MockPort` は
  既に `inject_dropped`/`inject_unsupported` を持ち、MIDI の
  cancel 相当の意味付けは audio stream と異なる。Issue リストの
  対象は audio stream と判断し、MIDI 側は棚卸し表で Deferred と記録。

### Expected/Observed テスト

- output/input 両方向で `Cancellation{at_callback: N}` を script し、
  index N の pump で data callback 未発火・error callback に
  `Backend("cancelled")`・`invalidated=true`・以後の pump が
  `StreamInvalidated`・drop 後再 open 可能、を Expected/Observed で
  assert する正負テスト。
- `inject_*_fault` の mid-stream 注入経路でも同じ効果を確認する。

## 3. docs 陳腐化修正

- `docs/tasks/issue-628-{deterministic-test-runtime,scripted-backend,
  pipeline-wiring}.md`: `status: "in-progress"` → `"done"`、task list
  の checkbox を実績に合わせて更新。`docs/tasks/README.md` は
  doc-maintenance の index 再生成で反映（手編集しない）。
- `crates/README.md` の `ruviform-audio-test` 行: "Issue #628 first
  slice" → deterministic core + scripted backend + pipeline wiring +
  close-out の現状記述へ。
- `crates/ruviform-audio-test/Cargo.toml` の `description` と crate
  `README.md` の slice 記述・Non-responsibilities（JSON serialization
  を後続 slice から外す）を現状へ更新。

## 4. AC 棚卸し表

`evidence/rf628/` に Issue 全 7 AC × 現状 verdict の表を追加する。
各項目に Pass/Partial/NotRun + 理由 + root 証拠（実 test 名・merge 済み
PR・既存 evidence 引用）を記す。MIDI→Host adapter `attach()` 配線は
`scarecrow173/ruviform#630` の所有として正直に Partial/Deferred と
記録し、本 slice では実装しない。

## 5. CI 統合の設計判断

AC 第 4 項目「既存CIに統合」: 本 slice の新規テストは既存
`mise run test ruviform-audio-test` / `mise run test-rt`（workspace
`rt_` name filter）の自動収集対象であり、新規 lane は不要。
workflow/mise の変更は範囲外。この判断を design doc・crate README・
棚卸し表に明記する。

## Components and Boundaries

- `src/metadata.rs` — serde derive + JSON API + doc 更新
  （"deferred to a later slice" コメントの解消）。
- `src/clock.rs` — `VirtualClockConfig`/`JitterSpec` に serde derive
  のみ追加。
- `src/backend.rs` — `StreamFault::Cancellation` variant + module doc
  の honest limits 更新。
- `src/backend/stream.rs` — `collect_faults` の返り値に cancel を追加、
  両 pump で cancel 経路（data callback skip → error 配送 → 終了）。
- `tests/metadata.rs` — round-trip・wire 語彙 pin・save/load test 追加。
- `tests/backend_faults.rs` — cancellation 正負テスト追加。
- docs chain・`evidence/rf628/`・index 再生成。

## Data and Control Flow

- JSON: `RunMetadata` → `serde_json::to_string` → file / file →
  `serde_json::from_str` → `RunMetadata`。round-trip で
  `PartialEq` 一致を検証。
- Cancellation: `collect_faults` が index 一致の `Cancellation` を検出
  → pump が data callback を skip → `Backend("cancelled")` を error
  callback へ → `invalidated`/`dead` セット → `PumpReport` 返却。

## Non-Functional Constraints

- JSON save/load は非 RT（alloc/IO）。`rt_` 契約経路へ割込み禁止。
- cancellation の pump 経路も既存と同じ alloc 特性（steady state で
  alloc なし — `Vec::new()` の error scratch は fault 発火時のみ
  alloc）。新規 `rt_` テストは追加しないが、既存 `rt_` テストの
  非回帰を `mise run test-rt` で確認する。
- `unsafe` なし。公開 enum の `#[non_exhaustive]` を維持
  （`StreamFault` variant 追加は enum レベル属性を変えない）。

## Risks and Trade-offs

| Risk | Mitigation |
| --- | --- |
| serde wire が Case Schema v1 語彙と drift | `as_str()` 一致 test + kebab-case rename の統一 |
| cancellation の解釈が consumer 期待とずれる | rustdoc/design/棚卸し表に写像を明記し、Issue close 判定材料として残す |
| `PumpReport` に cancel 専用 field がない | `errors_fired`+`invalidated`+`frames=0` で観測可能 — 公開 struct への field 追加は consumer 互換のため避ける |
| serde/serde_json 追加が test-only 境界を侵す | path 依存のみ・publish=false・本番 crate からの依存なしを維持 |

## Deferred Design Concerns

- MIDI 側 fault への cancellation 拡張 — spec の Deferred Design
  Concerns と同じ（consumer 要求時に再検討）。
- `RunMetadata` の schema versioning（fixture_version とは別の
  metadata format version）— 現状 v1 語彙固定で十分、必要になれば
  別 slice で `schema_version` field を検討。

## Verification Notes

- [ ] `mise run fmt-check` / `check` / `lint` / `test
  ruviform-audio-test` / `test-rt` / `prek run --all-files` pass。
- [ ] `git diff` が許可範囲内のみ（製品 crate src 差分なし）。
- [ ] docs index（触った dir）を doc-maintenance/impl-doc の再生成で
  整合。
