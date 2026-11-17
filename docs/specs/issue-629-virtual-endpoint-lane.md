---
id: "SPEC-034YnyKRDisxQAnJdC2wTj"
type: "spec"
status: "approved"
title: "Issue #629 virtual endpoint lane registration and Test Runtime wiring check (5th slice)"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-629-virtual-endpoint-lane.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/plans/issue-629-virtual-endpoint-lane.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/test-specs/issue-629-virtual-endpoint-lane.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/specs/issue-629-wasapi-endpoint.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-virtual-endpoint-lane.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 virtual endpoint lane registration and Test Runtime wiring check (5th slice)

## What Should Be Built

Issue #629 の 5th slice。4th slice で実装済みの WASAPI streaming
verifier（`tests/virtual_endpoint.rs`）を canonical な opt-in lane
として `mise.toml` に登録し、stale 参照を同期し、#628 Test
Runtime との結線を headless 範囲で検証する。4 部構成:

- **AC6 opt-in lane 登録** — `mise.toml` に
  `[tasks.test-audio-virtual-endpoint]` を追加する。
  `cargo test -p ruviform-audio-device --features real-device
  --test virtual_endpoint -- --ignored --test-threads=1` を実行
  する manual/opt-in task で、provisioning 依存・never CI を
  description に明記する（`test-audio-real-input` /
  `test-midi-real-device` の先例に従う）。専用 lint lane は
  追加しない — `lint-audio-real-input`（`--all-targets
  --features real-device`）が新 test target を既にカバーする
  （Coordinator 確定）。`docs/development-environment.md` の
  lane 一覧表と opt-in lane 注記に追記する。
- **stale 参照の同期** — `virtual_endpoint.rs` module doc・
  `endpoint_probe.rs` verdict 文言・`docs/designs/
  issue-629-wasapi-endpoint.md` の Deferred Design Concerns・
  `docs/discovery/issue-629-virtual-endpoint-survey.md` の
  「canonical=mise lane 未登録」記述を、登録済み canonical
  lane（`mise run test-audio-virtual-endpoint`）に合わせる。
  probe は文言変更のため Windows 実機で再実行し
  `evidence/rf629/endpoint_probe-windows-local.txt` を更新する。
- **#628 Test Runtime 結線検証（headless）** — merge 済みの
  `ruviform-audio-test` API（`PcmFixture`・
  `oracle::pcm_reference_block`・RunMetadata JSON
  save/load）と前回 docs に記録した結線 IF を照合し、齟齬を
  docs に記録する。`EnvironmentKind` の語彙を Case Schema v1
  の closed set（`deterministic-backend` /
  `virtual-os-endpoint` / `test-plugin` / `real-window` /
  `physical-device`）と照合し、欠落 variant
  （`virtual-os-endpoint`・`physical-device`）を additive に
  補完する。`ruviform-audio-test` は AL/SV が dev-dep 消費する
  ため additive のみ — 既存 variant の改名・serde 語彙変更・
  API 破壊・新規 dep は禁止。
- **lane self-run + evidence + docs chain** — 本ホストで
  `mise run test-audio-virtual-endpoint` を 1 回実行し、未
  provisioning のため `verdict: NotRun`（NOT_PROVISIONED）の
  正直な evidence を `evidence/rf629/` に記録する（Pass を
  主張しない）。spec→design→plan→task→test-spec→impl
  ir/exp の docs chain を作成する。

本 slice が直接対象とするのは Issue AC6 の headless 部分
（opt-in lane の `mise.toml` 登録と lane 説明の文書化）と、
AC3 の headless 部分（#628 Test Runtime 結線 IF の照合・
`EnvironmentKind` 語彙欠落の補完判断）。provisioned runner
上での実走 Pass（AC1/AC2/AC5）と consumer 受入（AC7）は
外部 gate であり、fail-closed の NOT_PROVISIONED/NotRun を
維持する。

## Why Now

- 4th slice（PR `scarecrow173/ruviform#673` merge 済み）で
  verifier・probe 拡張・fail-closed NOT_PROVISIONED が完了
  したが、canonical な起動入口が直接 cargo 実行（補助証拠）
  のままで、verifier test・probe verdict・design doc・survey
  doc に「mise lane 未登録」の stale 記述が残っている。lane
  設計は 4th slice design の AC6 lane 設計記録に確定済み
  なので、登録作業だけが残っている。
- #628 Test Runtime（`PcmFixture`/`oracle`/
  `RunMetadata`）は merge 済みだが、`EnvironmentKind` が
  Case Schema v1 の closed set 5 値に対し 3 variant のみで、
  `virtual-os-endpoint`・`physical-device` の語彙が欠落して
  いる。後続 slice が `virtual-os-endpoint` の証拠を typed に
  記録する前に語彙を揃える必要がある。
- `ruviform-audio-device` → `ruviform-audio-test` の dev-dep
  は 2026-11-16 Coordinator 不承認のため typed 実結線は行わ
  ず、結線 IF の照合と語彙補完を headless 範囲で閉じる。

## Users and Value

- **後続 slice（#629 残受入）**: provisioned runner 到着時に
  `mise run test-audio-virtual-endpoint` 一発で verifier を
  実行でき、lane 説明が `docs/development-environment.md` に
  揃う。
- **consumer harness（AL/SV・plugin-dock-test 等）**:
  `EnvironmentKind` が Case Schema v1 の closed set を typed
  に表現できるようになり、`virtual-os-endpoint` /
  `physical-device` の証拠を `RunMetadata` に正しく記録できる
  下地ができる。
- **証拠の誠実さ**: lane self-run が未 provisioning 環境で
  `verdict: NotRun` を返すことを canonical lane 経由で確認し、
  Pass 偽装の経路が無いことを記録で固定する。

## Scope

### In Scope

- `mise.toml` への `[tasks.test-audio-virtual-endpoint]`
  追加（opt-in manual lane、`test-audio-real-input` 先例の
  構造・description 記法）。
- `docs/development-environment.md` の lane 一覧表・opt-in
  lane 注記への追記。
- `crates/ruviform-audio-device/tests/virtual_endpoint.rs`
  module doc・`crates/ruviform-audio-device/examples/
  endpoint_probe.rs` verdict 文言の canonical lane 参照への
  更新。
- `docs/designs/issue-629-wasapi-endpoint.md` の AC6 lane 関連
  記述（Deferred Design Concerns・Verification Notes・
  Non-Functional Constraints）の同期。
- `docs/discovery/issue-629-virtual-endpoint-survey.md` の
  lane 登録・self-run evidence 追記（Evidence 節は
  append-only 規約のため新規 dated entry で記録）。
- `crates/ruviform-audio-test/src/metadata.rs` への
  `EnvironmentKind` additive variant 追加
  （`VirtualOsEndpoint`・`PhysicalDevice` — serde
  `kebab-case` を schema 語彙に一致）と
  `crates/ruviform-audio-test/tests/metadata.rs` の serde
  round-trip/spelling テスト更新。
- #628 結線 IF（`PcmFixture`/`oracle::pcm_reference_block`/
  RunMetadata JSON save/load）と merge 済み API の照合結果・
  verifier verdict→RunMetadata 対応付けの spec 決定の docs
  記録。
- `evidence/rf629/` への lane self-run 記録
  （`virtual_endpoint_slice5_lane_selfrun-windows-local.txt`）
  と probe 再実行による
  `endpoint_probe-windows-local.txt` 更新。
- docs chain（spec/design/plan/task/test-spec/impl ir・exp +
  `*-exp.jsonl`）。

### Out of Scope

- provisioned runner での verifier 実走 Pass（AC1/AC2/AC5）・
  実 provisioning・driver install・license 判断（repo 外工程）。
- `.github/**` の CI/runner 統合（定期 opt-in 実行）— 後続の
  外部 gate。
- Test Runtime 完全結線（`ruviform-audio-device` →
  `ruviform-audio-test` dev-dep は Coordinator 不承認 —
  typed 結線が必要と判断した場合は `ask`）。
- `EnvironmentKind::RealAbiTestPlugin` の serde 綴り
  `real-abi-test-plugin` と schema 語彙 `test-plugin` の差の
  解消 — 破壊的変更のため本 slice では行わず、既知 gap と
  して docs に記録し後続 slice の serde 移行判断へ hand-off
  （Coordinator 回答 (A)）。
  status: rf#725 Phase A で consumer 影響評価
  （`evidence/rf725/consumer-impact-assessment.md`）と
  deserialize alias `test-plugin` 受容を適用済み — emit flip
  は第2 slice（`docs/designs/issue-725-serde-vocab-phase-a.md`）。
- consumer 照合（AC7 — `scarecrow173/synvelis#115`/`#117`/
  `#126`・`scarecrow173/audio-lens#127`）。
- MIDI 側 lane（`test-midi-real-device` は登録済み・変更なし）。
- production 契約・verifier/probe の挙動変更（文言・doc
  comment のみ — 挙動は不変）。
- docs index（`docs/**/README.md`）の再生成 — 生成物であり
  Coordinator が統合時に再生成するため本 slice では触れない。

## Requirements

- 新規 lane は manual/opt-in — 通常 Daily Full CI の必須前提
  に無条件追加しない（Issue #629 本文 CI 節・POLICIES.md §5
  「Windows専用の仮想 Endpoint/実Window/実ABI Test Pluginの
  手動・opt-in・定期実行レーン」）。3OS lane は green を維持
  し、既存 lane・他 OS の経路を変えない。
- lane の description は provisioning 依存・manual・never CI
  を明記し、未 provisioning で `verdict: NotRun`
  （NOT_PROVISIONED）を返す fail-closed 契約を記述する。
- `EnvironmentKind` への variant 追加は additive のみ —
  既存 variant の改名・serde 語彙変更・API の変更/削除・新規
  dep は禁止（`ruviform-audio-test` は AL/SV が dev-dep
  消費）。enum レベルの `#[non_exhaustive]` は既付を維持。
- 実行していない検証を Pass と書かない。lane self-run の
  NotRun や lint/compile の Pass を `virtual-os-endpoint` の
  Pass や実 OS L3 実走 Pass と主張しない。証拠語彙は Case
  schema（Required/Deferred/NotApplicable × Pass/Fail/NotRun/
  Unsupported）と `environment_kind` に従う。
- `unsafe`・新規 dependency・feature flag の追加はない。
- 文書・コメント・報告は日本語。PR 本文・コミットメッセージ
  で他 Issue/リポジトリを参照するときは auto-reference を
  誘発しない inline code 形式を使い、対象 Issue への参照は
  `Refs #629`（`Closes` 不可）。

## Acceptance Criteria

- [ ] `mise.toml` に `[tasks.test-audio-virtual-endpoint]`
  が `test-audio-real-input` 先例の構造で登録され、
  description に provisioning 必須・manual・never CI が明記
  される。専用 lint lane は追加されず、既存
  `lint-audio-real-input` が coverage する旨が docs に記録
  される。
- [ ] `docs/development-environment.md` の lane 一覧表と
  opt-in lane 注記に `test-audio-virtual-endpoint` が追記
  される。
- [ ] `virtual_endpoint.rs`・`endpoint_probe.rs`・
  `docs/designs/issue-629-wasapi-endpoint.md`・
  `docs/discovery/issue-629-virtual-endpoint-survey.md` の
  stale な「lane 未登録」記述が canonical lane
  `mise run test-audio-virtual-endpoint` に同期され、probe
  再実行で `evidence/rf629/endpoint_probe-windows-local.txt`
  が更新される。
- [ ] #628 `PcmFixture`/`oracle::pcm_reference_block`/
  RunMetadata JSON save/load の merge 済み API と結線 IF の
  照合結果、verifier verdict→RunMetadata 対応付けの spec
  決定、`real-abi-test-plugin`↔`test-plugin` の既知語彙 gap
  が docs に記録される（status: rf#725 Phase A で deserialize
  alias `test-plugin` 受容を適用済み・emit flip は第2 slice —
  `docs/designs/issue-725-serde-vocab-phase-a.md`）。
- [ ] `EnvironmentKind` に `VirtualOsEndpoint`・
  `PhysicalDevice` が additive 追加され、serde
  `kebab-case` が schema 語彙（`virtual-os-endpoint`・
  `physical-device`）と一致し、`as_str`・doc コメント・
  serde round-trip テストが整合する。既存 variant・serde
  語彙・API は不変。
- [ ] `mise run test-audio-virtual-endpoint` の本ホスト
  self-run が `verdict: NotRun`（NOT_PROVISIONED）を
  `evidence/rf629/` に記録し、Pass を主張しない。
- [ ] Issue #629 の残受入（実 provisioning・実走 Pass・
  `.github/**` 統合・Test Runtime 完全結線・consumer 照合）
  は本 slice 対象外として docs/PR に明記される。

## Deferred Design Concerns

- provisioned runner での verifier 実走 Pass（AC1/AC2/AC5）・
  実 provisioning・driver install — 後続 slice / repo 外工程。
  再開条件: 専用 runner での provisioning 確立。
- `.github/**` CI/runner 統合（定期 opt-in 実行）— 後続の
  外部 gate。POLICIES §5 の opt-in lane 規範に対応。
- Test Runtime 完全結線（`ruviform-audio-device` →
  `ruviform-audio-test` dev-dep）— Coordinator 承認待ち。
  再開条件: 依存承認。
- `EnvironmentKind::RealAbiTestPlugin` の serde 綴り
  `real-abi-test-plugin` ↔ schema `test-plugin` の gap —
  破壊的変更のため後続 slice で serde 移行方針（rename+alias
  等）とともに決定。再開条件: consumer 影響評価 + 承認。
  status: rf#725 Phase A で consumer 影響評価
  （`evidence/rf725/consumer-impact-assessment.md`）と
  deserialize alias `test-plugin` 受容を適用済み — emit flip
  は第2 slice（`docs/designs/issue-725-serde-vocab-phase-a.md`）。
- reconnect/xrun/permission 実測（AC5）・consumer 照合
  （AC7）— 後続。
- 実体は `docs/designs/issue-629-virtual-endpoint-lane.md`
  の Deferred Design Concerns に記録する。

## Sources and Relations

- `scarecrow173/ruviform#629`（本 Issue、open のまま — PR は
  `Refs`）、`scarecrow173/ruviform#627`（Case Schema v1・
  `environment_kind`・Test Depth の契約元）、
  `scarecrow173/ruviform#628`（Test Runtime — AC3 結線対象）。
- `docs/designs/issue-629-wasapi-endpoint.md`（4th slice
  design — AC6 lane 設計記録の正本）、
  `docs/specs/issue-629-wasapi-endpoint.md`（4th slice spec）。
- `docs/designs/issue-627-virtual-first-verification-contract-
  design.md`（Case schema `environment_kind` closed set の
  正本）。
- `docs/discovery/issue-629-virtual-endpoint-survey.md`
  （1st slice 成果 — capability matrix・`--nocapture` 注意）。
- `POLICIES.md` §2.1（仮想優先の受入・証拠要件）・§5
  （ローカル検証と Daily CI の責務分離・opt-in lane 規範）。
- 先例: `mise.toml` の `test-audio-real-input` /
  `lint-audio-real-input` / `test-midi-real-device`（opt-in
  manual lane の構造・description 記法）、
  `docs/development-environment.md`（lane 一覧表・opt-in
  lane 注記の記法）。
