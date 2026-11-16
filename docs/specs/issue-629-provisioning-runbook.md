---
id: "SPEC-9bPUGYlLtIw7dXbS3LAyex"
type: "spec"
status: "approved"
title: "Issue #629 provisioning runbook and MIDI virtual lane separation (closeout-prep slice)"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-629-provisioning-runbook.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "docs/plans/issue-629-provisioning-runbook.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "docs/test-specs/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-provisioning-runbook.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 provisioning runbook and MIDI virtual lane separation (closeout-prep slice)

## What Should Be Built

Issue #629 の closeout-prep slice（残存 headless 作業の閉塞）。
前提 merge 済み: 1st slice survey+capability probe（PR
`scarecrow173/ruviform#640`）、2nd slice `midi_output` port +
WinMM `midiOut*` backend（PR `scarecrow173/ruviform#658`）、
3rd slice MIDI loopback round-trip verifier（PR
`scarecrow173/ruviform#663`）、4th slice WASAPI streaming
verifier（PR `scarecrow173/ruviform#673`）、5th slice
`test-audio-virtual-endpoint` lane 登録 + `EnvironmentKind`
語彙補完（PR `scarecrow173/ruviform#680`）。本 slice は残る
headless 部分を 4 部構成で閉じる:

- **AC6 残部: MIDI 仮想 endpoint 専用 lane 分離** —
  `mise.toml` に `[tasks.test-midi-virtual-endpoint]` を追加
  する。`cargo test -p ruviform-midi-device --test loopback --
  --ignored --test-threads=1` を実行する opt-in manual task で、
  provisioned 仮想 MIDI loopback pair（loopMIDI / Windows MIDI
  Services loopback）必須・manual・never CI を description に
  明記する。`--test loopback` は `loopback_round_trip`（仮想
  pair verifier・AC2 証拠経路）と `real_device_silent_output_
  smoke`（in-box GS Wavetable Synth の実 device smoke）を
  収集する — 両テストの依存を description で正直に明記する。
  feature gate が無いため `--features` は不要（audio 側 lane
  との相違点）。専用 lint lane は追加しない — 既定 `lint
  ruviform-midi-device`（`cargo clippy --locked --all-targets
  -p`）が `loopback` target を既にカバーする。
  `test-midi-real-device` は run コマンド不変（全 `#[ignore]`
  収集の包括 lane）とし、description/注記で仮想 pair 専用は
  `test-midi-virtual-endpoint` である旨を明記する。
  `docs/development-environment.md` の lane 一覧表と opt-in
  lane 注記に追記する（`test-midi-real-device` は従来表未収録
  だったため本 slice で併記する）。`loopback.rs` は module
  doc・doc comment の lane 参照更新のみで挙動不変。
- **AC4 残部: provisioning runbook** — 専用 Windows runner
  前提の provisioning 手順を docs chain（主に design）に
  文書化する。Issue 方針「名前・インストール方法を決め打ち
  しない」に従い候補別手順とし、capability probe
  （`endpoint_probe`/`midi_endpoint_probe`）の判定主導で採否
  を決めるフローとする。候補は survey doc の Alternatives が
  正本: Audio=VB-CABLE/VB-CABLE Hi-Fi/VAC（VoiceMeeter 対象外）、
  MIDI=loopMIDI/loopBE/Windows MIDI Services loopback、例外経路
  =SysVAD。license 条項確認を必須チェック項目とし、driver
  署名・権限・cleanup の記録責任分担（probe best-effort と
  runner 残件）を明記する。
- **AC5 残部（headless）: capability matrix 確定** — survey
  doc の暫定 Capability Matrix を runbook 附属の確定版に
  近づける。❓/⚠️ 行と Open Questions #1-#7 の実測待ち項目を
  明記し「provisioning 実測待ち」の状態を消さない。採用
  endpoint 未確定のまま再現 Case の実装は追加しない。
- **AC7 headless: consumer 照合** — Synvelis
  `scarecrow173/synvelis#115`/`#117`/`#126` と AudioLens
  `scarecrow173/audio-lens#127` の AC 本文 snapshot と照合し、
  consumer が要求する仮想 Endpoint 経路と機材固有動作の未保証
  範囲（物理 USB 抜去・ADC/DAC アナログ遅延の bit-exact・
  機種固有 driver 不具合・物理 Direct Monitoring・UMP/MPE
  future `scarecrow173/ruviform#317`）を runbook/design と
  survey doc に記録する。
- **stale 参照の同期 + lane self-run + evidence + docs
  chain** — `docs/designs/issue-629-wasapi-endpoint.md`・
  `docs/designs/issue-629-virtual-endpoint-lane.md` の
  Deferred Design Concerns を本 slice の解消範囲に同期し、
  survey doc Evidence 節へ dated entry を追記する
  （append-only 規約）。`mise run test-midi-virtual-endpoint`・
  `mise run test-midi-real-device`・`mise run test-audio-
  virtual-endpoint` を本ホストで各 1 回実行し、未 provisioning
  の正直な verdict（`NotRun`/`NOT_PROVISIONED`）を
  `evidence/rf629/` に記録する（Pass を主張しない）。

本 slice が直接対象とするのは AC6 の MIDI lane 分離、AC4 の
runbook 文書化、AC5 の headless 範囲（matrix 確定・実測待ち
明記）、AC7 の headless 範囲（consumer 照合・未保証範囲の
記録）。実 provisioning・driver install・license 判断・実走
Pass・`.github/**` CI/runner 統合・AC3 typed 結線・consumer
側の独立 Oracle での実受入判定は後続であり、fail-closed の
NOT_PROVISIONED/NotRun を維持する。

## Why Now

- AC6 は audio 側 lane（5th slice `scarecrow173/ruviform#680`
  merge 済み）だけが登録され、MIDI 側は `loopback.rs` の
  module doc が「`test-midi-real-device` が `#[ignore]` を
  収集するので lane 登録不要」と stale なまま残っている。
  3rd slice verifier（`loopback_round_trip`）が AC2 の仮想
  pair 明示 Case であり、audio 側と対になる専用 lane がない
  と「Windows のみの OS 仮想 Endpoint 実走を分けて再実行可能」
  という Issue AC6 の要件を満たせない。
- AC4 は probe・verifier・lane が揃ったが、専用 Windows
  runner への provisioning 手順（候補別 install/cleanup/
  license 確認・probe 判定フロー・両 lane の検証手順・
  NotRun 解釈）が未文書化で、provisioning 実測を担う担当者が
  誰でも同じ手順を再現できない。
- AC5 の capability matrix は survey doc で暫定のまま —
  ❓/⚠️ 項目と Open Questions #1-#7 の実測待ちを整理した
  確定版がないと、provisioned runner 到着後の実測計画が
  立てられない。
- AC7 は consumer Issue 本文 snapshot が揃っており、
  headless で照合・記録できる — consumer 側の実受入は
  後続だが、#629 が consumer に何を提供し何を保証しないかの
  記録は今閉じられる。

## Users and Value

- **provisioning 担当者（repo 外工程）**: 候補別の
  install/license 確認/cleanup/probe 手順と lane 検証手順が
  runbook として揃い、専用 Windows runner を再現可能に
  provisioning できる。
- **後続 slice（#629 残受入）**: `mise run
  test-midi-virtual-endpoint` で MIDI 仮想 pair verifier を
  audio 側 lane と対称に実行でき、lane 説明が
  `docs/development-environment.md` に揃う。
- **consumer（Synvelis/AudioLens）**: #629 が提供する仮想
  Endpoint 経路と機材固有の未保証範囲が文書化され、
  consumer 側 Issue の受入条件との対応が追跡できる。
- **証拠の誠実さ**: lane self-run が未 provisioning 環境で
  `verdict: NotRun` を返すことを canonical lane 経由で確認し、
  Pass 偽装の経路が無いことを記録で固定する。

## Scope

### In Scope

- `mise.toml` への `[tasks.test-midi-virtual-endpoint]` 追加
  （opt-in manual lane）と `test-midi-real-device` の
  description/コメント同期（run コマンド不変）。
- `docs/development-environment.md` の lane 一覧表（
  `test-midi-real-device`・`test-midi-virtual-endpoint` 行の
  追加）と opt-in lane 注記への追記。
- `crates/ruviform-midi-device/tests/loopback.rs` の module
  doc・`loopback_round_trip` doc comment の canonical lane
  参照更新（挙動不変 — doc のみ）。
- provisioning runbook の文書化（docs/designs/
  issue-629-provisioning-runbook.md に収録）— 専用 Windows
  runner 要件・候補別 install/cleanup/probe 手順・license
  確認・記録責任分担・両 lane 検証手順・NotRun 解釈。
- capability matrix 確定版（runbook 附属）— survey doc の
  暫定 matrix を引き継ぎ、実測待ち項目を明記。
- AC7 consumer 照合記録 — Synvelis #115/#117/#126・
  AudioLens #127 snapshot との対応と機材固有未保証範囲。
- `docs/designs/issue-629-wasapi-endpoint.md`・
  `docs/designs/issue-629-virtual-endpoint-lane.md` の
  Deferred Design Concerns 同期、survey doc Evidence 節への
  dated entry 追記。
- `evidence/rf629/` への 3 lane self-run 記録と docs chain
  （spec/design/plan/task/test-spec/impl ir・exp +
  `*-exp.jsonl`）。

### Out of Scope

- provisioned runner での実 provisioning・driver install・
  license 判断・両 lane の実走 Pass（AC1/AC2/AC5 実測）—
  repo 外工程 / 後続 slice。
- `.github/**` の CI/runner 統合（定期 opt-in 実行）— 後続の
  外部 gate。
- AC3 typed 結線（`ruviform-audio-device` →
  `ruviform-audio-test` dev-dep 不承認・逆方向も未裁定 —
  'ask' 扱いのまま残す）。
- 採用 endpoint 未確定のままの再現 Case 実装（VB-CABLE 系=
  hotplug 不可・loopMIDI=app 終了で port 消失 — 採用決定後）。
- consumer 側の独立 Oracle での実受入判定（Synvelis
  #115/#117/#126・AudioLens #127 — consumer 責務）。
- `EnvironmentKind` の `real-abi-test-plugin`↔`test-plugin`
  serde 語彙 gap（#389 占有中 — 本 slice では触れない）。
- production コード変更・新規 dependency/feature/`unsafe`・
  test target 分割・関数移動・docs index 再生成。

## Requirements

- 新規 lane は manual/opt-in — 通常 Daily Full CI の必須前提
  に無条件追加しない（Issue #629 本文 CI 節・POLICIES.md §5
  「Windows専用の仮想 Endpoint/実Window/実ABI Test Pluginの
  手動・opt-in・定期実行レーン」）。3OS lane は green を維持
  し、既存 lane・他 OS の経路を変えない。
- lane の description は provisioning 依存・manual・never CI
  を明記し、未 provisioning で `verdict: NotRun`
  （NOT_PROVISIONED）を返す fail-closed 契約を記述する。
  `--test loopback` が収集する 2 テストの依存差（仮想 pair
  必須 vs in-box synth）を正直に区別する。
- runbook は「名前・インストール方法を決め打ちしない」Issue
  方針に従い候補別記述とし、license 条項確認を必須
  チェック項目とする。実 provisioning・driver install は
  repo 外工程であり、開発 PC への無断 Driver 自動導入を
  前提にしない。
- 実行していない検証を Pass と書かない。lane self-run の
  NotRun や lint/compile の Pass を `virtual-os-endpoint` の
  Pass や実 OS L3 実走 Pass と主張しない。証拠語彙は Case
  schema（Required/Deferred/NotApplicable × Pass/Fail/NotRun/
  Unsupported）と `environment_kind` に従う。
- `unsafe`・新規 dependency・feature flag の追加はない。
  `loopback.rs` の変更は doc 参照更新のみで挙動不変。
- 文書・コメント・報告は日本語。PR 本文・コミットメッセージ
  で他 Issue/リポジトリを参照するときは auto-reference を
  誘発しない inline code 形式を使い、対象 Issue への参照は
  `Refs #629`（`Closes` 不可）。

## Acceptance Criteria

- [ ] `mise.toml` に `[tasks.test-midi-virtual-endpoint]`
  が登録され、`run` が `cargo test -p ruviform-midi-device
  --test loopback -- --ignored --test-threads=1`、
  description に provisioning 必須・manual・never CI・
  収集対象 2 テストの依存差が明記される。専用 lint lane は
  追加されず既定 `lint ruviform-midi-device` が coverage
  する旨が記録される。
- [ ] `test-midi-real-device` の run コマンドが不変のまま、
  description/注記が包括 lane（物理 port 系 + 全 `#[ignore]`
  収集）と仮想 pair 専用 lane の関係を正直に記述する。
- [ ] `docs/development-environment.md` の lane 一覧表と
  opt-in lane 注記に両 MIDI lane が追記される。
- [ ] `loopback.rs` の module doc・`loopback_round_trip`
  doc comment が canonical lane `mise run
  test-midi-virtual-endpoint` を参照し、挙動は不変。
- [ ] provisioning runbook（候補別 install/cleanup/probe・
  license 確認・記録責任分担・両 lane 検証手順・NotRun
  解釈）が design doc に文書化される。
- [ ] capability matrix 確定版が runbook 附属として存在し、
  ❓/⚠️・Open Questions #1-#7 の実測待ち項目が明記される。
- [ ] AC7 consumer 照合（Synvelis #115/#117/#126・
  AudioLens #127）と機材固有未保証範囲が docs に記録される。
- [ ] wasapi-endpoint・virtual-endpoint-lane 両 design の
  Deferred Design Concerns と survey doc Evidence 節が本
  slice の解消範囲に同期される。
- [ ] `mise run test-midi-virtual-endpoint`・`mise run
  test-midi-real-device`・`mise run test-audio-virtual-
  endpoint` の本ホスト self-run が `evidence/rf629/` に
  記録され、Pass を主張しない。
- [ ] Issue #629 の残受入（実 provisioning・実走 Pass・
  `.github/**` 統合・AC3 typed 結線・consumer 実受入）は
  本 slice 対象外として docs/PR に明記される。

## Deferred Design Concerns

- provisioned runner での実 provisioning・driver install・
  license 判断・両 lane の実走 Pass（AC1/AC2/AC5 実測）—
  後続 slice / repo 外工程。再開条件: 専用 runner での
  provisioning 確立。
- `.github/**` CI/runner 統合（定期 opt-in 実行）— 後続の
  外部 gate。POLICIES §5 の opt-in lane 規範に対応。
- AC3 typed 結線（`ruviform-audio-device` →
  `ruviform-audio-test` dev-dep）— Coordinator 承認待ち。
  再開条件: 依存承認。
- 採用 endpoint の確定（VB-CABLE vs VAC vs SysVAD、loopMIDI
  vs WMS loopback）とそれに伴う再現 Case 実装 — provisioning
  実測後。
- consumer 側の独立 Oracle での実受入判定（Synvelis
  #115/#117/#126・AudioLens #127）— consumer 責務。
- `EnvironmentKind` の `real-abi-test-plugin`↔`test-plugin`
  serde 語彙 gap — #389 占有解除後または Coordinator 調整。
- 実体は `docs/designs/issue-629-provisioning-runbook.md`
  の Deferred Design Concerns に記録する。

## Sources and Relations

- `scarecrow173/ruviform#629`（本 Issue、open のまま — PR は
  `Refs`）、`scarecrow173/ruviform#627`（Case Schema v1・
  `environment_kind`・Test Depth の契約元）。
- `docs/discovery/issue-629-virtual-endpoint-survey.md`
  （1st slice 成果 — 候補比較・暫定 capability matrix・
  Open Questions #1-#7・Evidence 節の正本）。
- `docs/designs/issue-629-wasapi-endpoint.md`（4th slice —
  AC6 lane 設計記録・verifier 契約）、
  `docs/designs/issue-629-virtual-endpoint-lane.md`（5th
  slice — audio lane 登録の先例）、
  `docs/specs|designs|plans|tasks|test-specs|
  issue-629-midi-loopback.md`（3rd slice — loopback
  verifier の要件）。
- consumer AC snapshot: `scarecrow173/synvelis#115`/
  `#117`/`#126`、`scarecrow173/audio-lens#127`（Issue 本文
  snapshot との照合 — 実体は design の AC7 節）。
- `POLICIES.md` §2.1（仮想優先の受入・証拠要件）・§5
  （ローカル検証と Daily CI の責務分離・opt-in lane 規範）。
- 先例: `mise.toml` の `test-audio-virtual-endpoint` /
  `test-midi-real-device`（opt-in manual lane の構造・
  description 記法）、`docs/development-environment.md`
  （lane 一覧表・opt-in lane 注記の記法）。
