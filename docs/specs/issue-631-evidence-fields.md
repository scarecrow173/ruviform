---
id: "SPEC-m4Qz8wVn2Xk7pLc9dRt3Yb"
type: "spec"
status: "approved"
title: "Issue #631 AC4 evidence-fields 完成"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-631-evidence-fields.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 AC4 evidence-fields 完成

## What Should Be Built

Issue #631 AC4 の残 evidence field を、manifest 変更不要の
部分集合で完成させる。`crates/ruviform-plugin-dock-test` 内に
閉じ、catalog 件数（headless 9 + real 22）・全 `Cargo.toml`/
`Cargo.lock`・production crate は不変。

1. **window hash 全行** — session 開始時に mount 済み window
   A–D 各々の `window_geometry` 初回値（origin・physical px・
   scale factor）+ `WindowId` + mount title を canonical 文字列
   化して SHA-256（`dpi_body` の `window_geometry_sha256` と
   同方式）を計算し、`RealDockContext` に保持 →
   `finalize_case` の merged extras で全 real case の
   `environment.window_identities` に刻む。
2. **capture/artifact digest** — 全 pixel capture step row に
   `capture_sha256`（実画素バイトの SHA-256）を記録し、
   opt-in PNG artifact の記録を `artifact: {path, sha256}`
   object に拡張。`--pixel-artifacts` を `real-capture` にも
   拡張する（opt-in 維持・canonical 配線はしない）。
3. **semantic tree 全量** — `observed.semantic_tree` を記録する
   case で mount 済み `ViewSpec` の `ComponentSpec` children を
   `ViewHost::validate_spec` と同じ walk（Panel children 再帰 +
   Tabs alias/canonical 両 key）で列挙し、各 key を
   `resolve_target`+`observe` して `component_snapshot_json`
   で記録。観測失敗は `null`（捏造しない）。real lane の
   `real-window-mount`・`real-command-receipt` と headless
   `command-receipt` が共有 helper を使う。
4. **README 修復** — `real-window-dpi`/`real-window-close` の
   stale `Unsupported` 行を現行実装（live geometry 観測判定・
   armed close lifecycle 実観測）に合わせ、`real-command-receipt`
   行を catalog 順で追加。`mise.toml` の stale description
   （"honest Unsupported for DPI/IME/close"）も実態に合わせる
   （canonical 引数は不変）。
5. **evidence 再生成** — `plugin-dock-real-headless.jsonl`
   （`--blocked-probe`、必須）と `plugin-dock-real-scenario.jsonl`
   （interactive Windows desktop があれば実走）を再生成し、
   `evidence/rf631/verification.md` に slice 記録を追記する。

## Why Now

Issue #631 AC4 は「Screenshot/Pixel diff/semantic tree/Command+
Receipt/OS and GPU/Case Seed/Window & Plugin hash、Repo/SHA・
Ruviform resolved SHA を保存」を要求する。Case Seed・Repo/SHA・
plugin hash は先行 slice で実装済みだが、window hash は
`real-window-dpi` の 1 case のみ、capture/artifact は digest
なし・path のみ、semantic tree は fixture の一部 key のみ、
README verdict 表は catalog-flip/command-receipt slice で
stale 化したまま残っている。本 slice は残りの manifest 不要
field を完成させ、evidence の帰属可能性を揃える。

## Users and Value

- Issue #631 の受入判定者 — AC4 の evidence field が全 real
  case 行で揃い、run 間の window/capture 同一性を digest で
  検証できる。
- 後続 slice（IME・verb・pixel Pass・consumer 投影）の実装者 —
  evidence 形式の土台が確定する。
- evidence consumer — `capture_sha256`/`{path,sha256}`/
  `window_identities`/全量 `semantic_tree` により、PNG を
  保存しなくても画素・window・semantic 状態の run 間 diff が
  可能になる。

## Scope

### In Scope

- `real_dock.rs`: `RealDockContext` への window identity 保持、
  `run_all_with_pixel_artifacts` での session 開始時 digest 計算、
  `finalize_case` での `environment.window_identities` stamp、
  `capture_sha256` helper + 全 capture row への記録、
  `save_pixel_artifact`/`record_pixel_artifact` の `{path,
  sha256}` 拡張、`real-capture` の `capture_sha256` +
  `--pixel-artifacts` 対応、`real-window-mount`/`real-command-
  receipt` の semantic tree 全量化、feature-gated unit test 追加。
- `scenario.rs`: `SemanticGuiTest` generic の semantic tree
  helper（`ViewSpec` walk + `resolve_target`/`observe` +
  `component_snapshot_json`、失敗 `null`）を追加し、headless
  `command-receipt` の `semantic_tree` を全量化。
- `bin/plugin-dock-real-scenario.rs`: `--pixel-artifacts` の
  doc を `real-capture` 拡張に追従。
- `README.md`: verdict 表の stale 行修復 + `real-command-receipt`
  行追加。
- `mise.toml`: `test-ui-plugin-dock-real` description の stale
  文言修正（canonical 引数は不変）。
- `tests/plugin_dock.rs`: headless `command-receipt` の
  semantic tree 全量断言。
- `evidence/rf631/plugin-dock-real-headless.jsonl` /
  `plugin-dock-real-scenario.jsonl` の再生成（後者は interactive
  desktop がある場合のみ）+ `verification.md` 追記。
- doc chain（discovery/spec/design/plan/task/test-spec/adr/ir/exp）。

### Out of Scope

- GPU 情報（`windows-sys` `Win32_Graphics_Dxgi` feature 追加
  または GPUI renderer-info API が必要 — manifest 変更は
  ask-first、別 Issue 候補）。
- `real-window-ime`・editor verbs・DPI 変更駆動・crash/hang
  out-of-process 化（`ruviform#389` 領域）。
- pixel Pass（`ruviform#532` gate — `real-dcomp-pixel` の既知
  `Fail` を正直に保持）。
- consumer 投影（synvelis#114/#176・audio-lens#127 gate）。
- canonical `--pixel-artifacts` 化（opt-in + canonical digest
  を採用 — 下記 ADR）。
- catalog 件数・row の追加/削除（headless 9 + real 22 を維持）。
- `crates/ruviform-ui`/`ruviform-ui-gpui`/`ruviform-ui-test`/
  `ruviform-exec` 等の production/test-driver crate 変更
  （必要 API は全て実装済み）。
- `Cargo.toml`/`Cargo.lock`/新規依存/feature 追加。
- `docs/**/README.md`（docs index — Coordinator が統合時に
  再生成）。
- lease 外の stale コメント残件
  （`bin/plugin-dock-console.rs:9`・`src/lib.rs:53-54`）。

## Requirements

- window identity digest は session 開始時に window A–D 各々に
  ついて 1 回だけ計算する — `window_geometry` の live 値 +
  `WindowId` + mount title を canonical 文字列化し SHA-256。
  geometry 読取に失敗した window は `sha256: null` の正直な
  記録とし、identity を捏造しない。`blocked_reports`/
  `blocked_probe` 経路は session が無いため刻まない。
- `real-window-dpi` の `window_geometry_sha256`（case 内再観測
  digest）は変更しない — 別 field として残す。
- 全 pixel capture step row に `capture_sha256`（実際に
  capture した RGBA 画素バイトの SHA-256）を記録する。
  対象は `real-capture`・`real-dcomp-pixel`（`screen_pixels`/
  `printwindow_pixels`/`windowdc_pixels`）・`real-dcomp-
  foreground`（`fg_observe_*` の screen object）・
  `real-dcomp-present`（`pres_capture`/`pres_screen`）。
- PNG artifact を書いた場合の記録は `artifact: {path, sha256}`
  — sha256 は書き込んだ PNG bytes の digest。artifact 書込
  失敗は従来どおり `artifact_error` field で、verdict を
  変えない。
- `--pixel-artifacts` は opt-in のまま — canonical
  `test-ui-plugin-dock-real` は artifacts dir を渡さない。
  `real-capture` も同じ opt-in で PNG を書ける。
- semantic tree は mount 済み `ViewSpec` の全 component key
  （`validate_spec` の flat 列挙と同じ — Panel children 再帰、
  Tabs は alias key + canonical child key の両方）を列挙し、
  各 key の `ComponentSnapshot` を `component_snapshot_json`
  で記録する。resolve/observe 失敗は `null`。verdict ロジック
  とは分離した best-effort evidence で、新規 driver API は
  追加しない。
- `REAL_CATALOG`/`real_entries`/headless catalog の件数・
  `case`/`scenario`/`required` は不変 — 既存の整合 test が
  green を維持する。
- headless/blocked では fail-closed 形状を維持 — 新 field も
  session 不在の行に刻まず、`environment_blocked`/`NotRun`
  を偽装しない。`EnvironmentKind::RealWindow` は実走 session
  内の report のみが名乗る。
- evidence 拡充は additive — 既存 field の型・意味を変えない
  （`artifact` の path-only 文字列 → `{path, sha256}` object
  は AC4 要求の shape 変更として docs/verification に明記）。

## Acceptance Criteria

- [ ] 全 real case の `environment.window_identities` に
  session 開始時の per-window identity（window id・title・
  geometry・sha256、失敗時 `null`）が記録される
  （`plugin-dock-real-scenario.jsonl` 実走 evidence で確認）。
- [ ] 全 pixel capture step row に `capture_sha256`・`backend`・
  `width`・`height` が記録され、opt-in artifact は
  `{path, sha256}` object になる。
- [ ] `real-capture` が `--pixel-artifacts` で PNG を書け、
  canonical lane は artifacts を書かない。
- [ ] `real-window-mount`・`real-command-receipt`・headless
  `command-receipt` の `observed.semantic_tree` が mount 済み
  spec の全 component key（alias 含む）を持ち、観測失敗 key は
  `null`。
- [ ] README verdict 表が `REAL_CATALOG`/実挙動と一致し、
  `real-command-receipt` 行を含む 22 行になる。
- [ ] `plugin-dock-real-headless.jsonl` が全行 `NotRun`+
  `environment_blocked` を exit 0 で維持して再生成される。
- [ ] interactive Windows desktop があれば
  `plugin-dock-real-scenario.jsonl` が新 field 込みで再生成
  される（不可なら NotRun として正直に記録）。
- [ ] 非回帰 lane（`fmt-check`/`check`/`lint`/`test`
  （default+`real-dock`+`lab-gui`）/`test-ui-plugin-dock`/
  `test-ui-plugin-dock-console`/`test-ui-plugin-dock-lab`/
  `test-ui-scenario-contract`/`check-ui-driver-boundary`/
  `prek`）が green。
- [ ] `evidence/rf631/verification.md` に本 slice の検証記録が
  追記される。

## Deferred Design Concerns

- GPU 情報（adapter/GPU 識別子）— `windows-sys` feature 追加
  （manifest 変更 = ask-first）または GPUI renderer-info API
  （現存しない）が必要 | 別 Issue 起票・承認時 | AC4 の GPU
  field が未記録のまま。
- `real-window-ime`・editor verbs・DPI 変更駆動・crash/hang
  out-of-process — #389 領域 | #389 session lifecycle 着地時 |
  該当 row が `Unsupported` のまま。
- pixel Pass — #532 gate | DComp occlusion 解消時 |
  `real-dcomp-pixel` verdict=fail を正直に保持。
- consumer 投影 — synvelis#114/#176・audio-lens#127 gate |
  各 repo の受入時 | driver-neutral 投影が未検証のまま。
- canonical `--pixel-artifacts` 化 — PNG binary churn のため
  opt-in + canonical digest を採用（ADR 記録） | opt-in
  artifacts の運用で不足が判明した場合 | PNG が evidence
  に残らない（digest のみ）。
- lease 外 stale コメント（`plugin-dock-console.rs:9`・
  `lib.rs:53-54`）— lease 外 | 範囲拡張の承認時 | stale 記述が
  残る。

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — auto-close しない）。
- `docs/discovery/issue-631-evidence-fields.md` — 探索記録。
- `docs/specs/issue-631-real-window-catalog-flip.md` —
  `session_scale_factor`/`finalize_case`/`case_seed` 先例の spec。
- `docs/adr/issue-631-headless-command-receipt.md` —
  real/headless command-receipt 対応構造。
- `POLICIES.md`（検証レベル・Pass/Fail/NotRun/Unsupported
  規範・fail-closed）。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`・
  `scenario.rs`・`README.md`・`src/bin/plugin-dock-real-
  scenario.rs`・`tests/plugin_dock.rs` — 変更対象。
- `crates/ruviform-ui/src/view.rs`・
  `crates/ruviform-ui-test/src/lab_story.rs`・
  `crates/ruviform-ui-test/src/scenario/plugin_dock.rs` —
  spec 構造の参照先（変更なし）。
- `mise.toml` — `test-ui-plugin-dock-real` description 修正。
- `evidence/rf631/` — 再生成対象 evidence + verification 記録。
