---
id: "SPEC-9bPUGYlLtIw7dXbS3LAyex"
type: "spec"
status: "approved"
title: "Issue #629 provisioning runbook and MIDI virtual lane separation (closeout-prep slice)"
created: "2026-11-17"
updated: "2026-11-25"
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
  status: rf#725 Phase A で deserialize alias `test-plugin`
  受容を適用済み・emit flip は第2 slice
  （`docs/designs/issue-725-serde-vocab-phase-a.md`）。
  status: rf#725 Phase B で emit flip を実施・検証済み —
  serde emit・`as_str()`・plugin-dock-test assert が全て
  `test-plugin` に統一され語彙 gap は解消（deserialize は
  新旧両綴りを恒久受容 —
  `docs/designs/issue-725-serde-vocab-phase-b.md`・
  `docs/impl/ir/issue-725-serde-vocab-phase-b.md`）。
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
  status: rf#725 Phase A で consumer 影響評価と deserialize
  alias `test-plugin` 受容を適用済み — emit flip は第2 slice
  （`docs/designs/issue-725-serde-vocab-phase-a.md`）。
  status: rf#725 Phase B で emit flip を実施・検証済み —
  serde emit・`as_str()`・plugin-dock-test assert が全て
  `test-plugin` に統一され語彙 gap は解消（deserialize は
  新旧両綴りを恒久受容 —
  `docs/designs/issue-725-serde-vocab-phase-b.md`・
  `docs/impl/ir/issue-725-serde-vocab-phase-b.md`）。
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

## 追記（Issue #977 Slice B / #978 Slice C / #979 Slice E）: ruviform 独自 driver による provisioning と lane 実走

本 runbook は外部 driver（VB-CABLE / loopMIDI / WMS loopback）前提で書かれたが、
#952 の方式確定により ruviform 独自仮想 audio driver（SysVAD 派生）が第一経路と
なった。Issue #977 stage 1 で `drivers/rvf-virtual-audio/`（render
`Ruviform Cable Input` / capture `Ruviform Cable Output` の 1 pair）と
provisioning 自動化 workflow を追加し、stage 2 で render→capture の kernel
loopback を実装（音声経路成立 — rf977 attempt-21 で round-trip Pass）、Issue
#978 で同 package に DMus MIDI loopback miniport（`Ruviform MIDI Loopback`
in/out 同名 pair）を同梱した。Issue #979（Slice E）では `midi-endpoint-probe`
stage の追加と本節・lane 表の整合を行った。外部 driver 候補の手順・license
条項は引き続き本書前文を正本とする（独自 driver が採用経路として優先される
だけで、外部候補の判断記録は残す）。

### provisioning・lane 実走手順（独自 driver — GitHub-hosted `windows-2025` x64）

manual workflow `.github/workflows/virtual-audio-driver-probe.yml`
（`workflow_dispatch` のみ・opt-in manual lane — required gate・`ci-summary`
の `needs`・ruleset と非接続）が、専用 runner 前提手順に相当する全工程を
stage 分解で実施する。sv#115 Slice A 以降、build/test-sign/install/devnode/
mic consent/endpoint readiness/cleanup の OS state 変更は
`tools/virtual-audio-driver/Provision-RvfVirtualAudioEndpoint.ps1` と
`Unprovision-RvfVirtualAudioEndpoint.ps1` に抽出され、workflow は primitive
state JSON を従来 stage 名・state key・stage log 名へ投影する:

1. `env-build-surface` — OS build・testsigning/SecureBoot・WDK/署名 tool の記録。
2. `driver-build` — repo 内ソースを MSBuild+WDK で build（WDK 不在時は winget
   `Microsoft.WindowsWDK.10.0.26100` job 内導入 + DriverKit task dll
   workaround — rf961 実測経路の踏襲）。
3. `catalog-create` — `RvfVirtualAudio.inf` + `.sys` の `.cat` を Inf2Cat →
   makecat fallback で生成。
4. `cert-create` / `store-register` — self-signed CodeSigning cert を生成し
   `LocalMachine\Root`+`TrustedPublisher` に pre-trust。
5. `sign` / `sign-verify` — `.sys`+`.cat` を signtool 署名・`/pa` 検証。
6. `pnputil-add-driver` — `pnputil /add-driver RvfVirtualAudio.inf /install`
   で driver store へ staging。
7. `enum-verify` — INF の driver store 在籍確認と、devnode 作成前の Ruviform/RvfVirtualAudio matching devnode 数の read-only 観測。sv#115 Slice A では primitive が `pnputil-add-driver` 後・`devnode-create` 前に計測し、workflow が legacy stage へ投影する（正常系では matching devnode 0 件）。
8. `devnode-create` — `tools/testsigned-driver/TestDevice.ps1` を
   `-ClassName MEDIA -ClassGuid {4d36e96c-e325-11ce-bfc1-08002be10318}
   -HardwareId 'ROOT\RvfVirtualAudio'` で呼び出し、root-enumerated MEDIA
   class devnode を作成。problem_code≠0 時は CI event log・
   setupapi.dev.log・testsigning 実効値を診断採取する。
9. `endpoint-probe` — `tools/virtual-audio-driver/Invoke-EndpointProbe.ps1`
   が `endpoint_probe`（mise Rust toolchain 経由）を実行し `pair_count=` 等を
   stage record に記録する。
10. `midi-endpoint-probe` — `tools/virtual-audio-driver/Invoke-MidiEndpointProbe.ps1`
    が `midi_endpoint_probe`（同じく mise 経由）を実行し WinMM
    midiIn/midiOut 列挙・`loopback_candidates=`・`provisioning=` を stage
    record に記録する（Issue #979 で追加 — `endpoint-probe` の MIDI 版。
    候補未検出は観測値で失敗扱いしない）。
11. `stream-roundtrip` — `mise run test-audio-virtual-endpoint`
    （`tests/virtual_endpoint.rs`）を実走し、`verdict:` 行を stage record に
    記録する。`Ruviform Cable Input` への render は kernel loopback 経由で
    `Ruviform Cable Output` の capture に届く真の audio round-trip（Issue
    #977 stage 2 — rf977 attempt-21 で Pass 実績）。mic consent provisioning
    （DeviceAccess/ConsentStore/AppPrivacy + AudioEndpointBuilder/Audiosrv
    restart）は provisioning primitive の `mic-consent` stage で実施済みであり、本 stage は provisioned 環境を使う test 側に限定する。**verdict の読み方**: pass 時の detail
    は `verdict=no-verdict-line` になるのが正常 — cargo test は passing
    test の stdout/stderr を capture して表示しないため、test が emit した
    `verdict: Pass` 行は artifact ログに表面化しない（`verdict:` 行が
    表面化するのは Fail/NotRun 経路のみ — 失敗・異常時に capture 内容が
    出力される）。実走判定は `test result: ok` と round-trip test 本体の
    `finished in X.XXs` の非ゼロ秒数で読む — endpoint 不在の vacuous
    early return は `finished in 0.00s` になる（rf977 attempt-14 の
    観測）ため、非ゼロ秒数 + 同一 run の `endpoint-probe` `pair_count=1`
    で真の round-trip 実走と判断する（rf977 attempt-21・rf979 attempt-1
    ともに 3.48s）。Issue #1002 以降は同一 target に未武装で inactive 終了
    する fault test も含まれるため、合計 pass 件数は実走判定に使わない。
12. `midi-loopback` — `tests/loopback.rs`（`cargo test -p
    ruviform-midi-device --test loopback -- --ignored --test-threads=1`）を
    実走し、`Ruviform MIDI Loopback` の bidirectional name match pair 検出と
    channel-voice event round-trip の `verdict:` を記録する（Issue #978 —
    rf978 attempt-6 で Pass 実績）。事前に `midi-winmm-probe.ps1` を child
    pwsh で走らせて WinMM call の bisection 診断を採取する。
13. `os-endpoint-wiring` — Issue #1001 Slice F。`mise exec -- cargo test -p
    ruviform-audio-test --test os_endpoint -- --ignored --test-threads=1
    --nocapture` を実行し、AC3 wiring leg 2 件（audio case id
    `rf629.audio.virtual-endpoint.render-capture` / midi case id
    `rf629.midi.virtual-endpoint.loopback`）の verdict と `run_metadata:`
    JSON を採取する。canonical lane は `mise run test-os-endpoint-wiring`
    （`--nocapture` なし）で、passing test の stderr は libtest が capture
    するため、この直接 cargo 実行は verdict/RunMetadata を surface させる
    補助証拠と明確に区別する。provisioning primitive で実施済みの mic consent
    provisioning を同一 job 内で再利用するため `midi-loopback` 後・
    `cleanup` 前に配置する。stage は `RUVIFORM_RUN_REPO` に
    `${{ github.repository }}`、`RUVIFORM_RUN_SHA` に `${{ github.sha }}` を
    設定し、`run_metadata_path:`
    行が示す `CARGO_TARGET_TMPDIR` 内 JSON を workspace 内
    `os-endpoint-run-metadata-<os>/` へ copy して artifact に含める。stage
    record detail には audio/midi 各 result・verdict 行数・metadata 件数・
    copy 件数を記録する。Pass/NotRun は fail-closed contract 上の正常終端
    （stage `ok`）だが、Fail・crash・verdict 未到達・RunMetadata 未到達/
    不正・copy 不能は `error`。AC3 の Pass 判定は stage status ではなく
    artifact の両 RunMetadata JSON 本体が `result:"pass"`・
    `level:"ext-l3"`・`environment_kind:"virtual-os-endpoint"`・対象
    repo/SHA を示すことで行う。
14. `os-endpoint-midi-e2e` — Issue #253 AC1/AC6 vertical slice。`os-endpoint-wiring` 後・`fault-cases` 前に置き、fixture build 後に `mise exec -- cargo test -p ruviform-plugin-test-vst3 --features l1-runner --test os_endpoint_midi_e2e -- --ignored --test-threads=1 --nocapture` を補助証拠として実行する。canonical lane は `mise run test-os-endpoint-midi-e2e`（`--nocapture` なし）である。stage は `midi-endpoint-probe` の candidate/provisioning 行と `midi-loopback` の candidate/round-trip Pass 行を生ログへ転記し、両者が揃った場合だけ `RUVIFORM_VIRTUAL_ENDPOINT_PROVENANCE=rvfmidi-testsigned-driver-probe` を設定する。RunMetadata は `os-endpoint-midi-e2e-run-metadata-<os>/` へ copy し、repo/SHA・`level=ext-l3`・`environment_kind=virtual-os-endpoint`・result を検査する。honest NotRun は absence-is-data の stage `ok` だが、`result=pass` に RvfMidi provenance 行がない場合、または workflow 側 provenance が未成立のまま pass だった場合は証拠価値がないため stage `error` とする。Fail・crash・verdict/RunMetadata 未到達・copy 不能も `error`。
15. `fault-cases` — Issue #1002 Slice G。`os-endpoint-midi-e2e` 後・`cleanup`
    前に置き、OS 状態制御を workflow/PowerShell 側へ閉じ込めた fault harness
    を実行する。controller 本体は `tools/virtual-audio-driver/Invoke-FaultCases.ps1`
    に置き、workflow step は呼び出しと fail-closed stage record だけを担う。
    case key は `device_loss_audio`（devnode disable を第一選択とし、
    open stream により拒否された場合は remove/recreate fallback で stream
    invalidation 観測 → enable/recreate → reopen recovery）、
    `permission_denial_audio`（microphone consent Deny + service restart →
    capture open oracle → Allow restore）、`midi_device_loss_observation`
    （devnode loss 前/消失中/復帰後の WinMM 列挙で `Ruviform MIDI Loopback`
    の消失を正規 oracle として観測し、復帰後の再出現は限界 evidence として
    記録）、`xrun_probe`（bounded user-mode starvation が現行
    adapter で `DeviceError::Xrun` として観測できるかの判断材料）とする。
    Rust test は `RUVIFORM_FAULT_*` で武装された場合だけ active になり、
    ready/observed/restore phase file で controller と同期する。canonical
    lane で未武装実行された場合は inactive 記録に留まり、既存の generic
    `verdict:` 契約へ混入しない。OS 側 stimulus は成立したが現行 driver/
    adapter 契約が typed invalidation を surface しない場合は、測定 harness
    の Fail と区別して case-level `Unsupported` として記録する。case ごとの正規判定は
    `fault-cases-results-<os>.json`（`rf1002-fault-case/v1`・repo/SHA/run id/
    attempt を含む）と生ログの `fault_case_result:` 行で行い、語彙は
    Pass/Fail/NotRun/Unsupported。stage `ok` は測定 harness が完走し
    case-level Fail がなかった意味に限定し、前提不足なら `not_run`、
    controller/復帰/case-level Fail は `error` とする。disable した devnode
    と変更した microphone consent/service 状態は finally/cleanup で必ず
    Allow+Running へ戻す。
16. `cleanup` — `Unprovision-RvfVirtualAudioEndpoint.ps1` による devnode 削除・`pnputil /delete-driver`・証明書除去・consent before-value 復元・audio service restart と residue 観測。
17. `Upload probe evidence` — 上記全 stage の記録を attempt 固有名の
    artifact（`virtual-audio-driver-probe-<os>-attempt-<n>`）として upload。

- license: MS-PL（audio 部）/MS-LPL（MIDI miniport 部）派生のため外部 driver
  のような license 判断・再起動・再配布上の摩擦はない
  （`drivers/rvf-virtual-audio/LICENSE-MS-PL.txt`・`LICENSE-MS-LPL.txt`・
  `PROVENANCE.md` 参照）。
- reboot/bcdedit: 実施しない（testsigning=Yes+SecureBoot OFF は runner 出荷
  状態として確定済み — 本書前文の runner 要件と同じ根拠）。
- driver 署名: 上記 4-5 の job 内 test-signing 経路。attestation/WHQL 署名は
  要らない（x64 のみ対象 — #952 §Slice D 参照）。
- absence-is-data 契約: 各 stage は `if: always()` + 末尾 `exit 0` で完走し、
  stage の失敗・測定対象の不在は `{stage,status,detail,exit_code}`（status
  語彙 ok/absent/blocked/not_run/error）として `vaudio-probe-windows-2025.json`
  に記録する判定材料であり job 失敗ではない。生ログは
  `stage-*-windows-2025.txt`、probe 出力は `endpoint-probe-windows-2025.txt` /
  `midi-endpoint-probe-windows-2025.txt`、wiring leg の RunMetadata JSON は
  `os-endpoint-run-metadata-windows-2025/`、MIDI Host/Plugin e2e の RunMetadata JSON は
  `os-endpoint-midi-e2e-run-metadata-windows-2025/`、fault case の機械可読結果は
  `fault-cases-results-windows-2025.json`、同期/観測用の fault file は
  `fault-*-windows-2025*` に残り、失敗時も `if: always()` の artifact upload
  で保全される。attempt 固有 artifact 名で全 attempt の evidence が残る。
- 記録責任分担: stage record・生ログ・probe 出力は artifact として採取され、
  `evidence/rf977/`・`evidence/rf978/`・`evidence/rf979/`・`evidence/rf1001/`・
  `evidence/rf1002/`・`evidence/rf253/` に attempt 番号対応付きで記録される（install 後の残件は
  cleanup stage が除去）。`os-endpoint-wiring` の AC3 判定では stage record に
  加えて `run-metadata-rf629-audio-virtual-endpoint-render-capture.json` と
  `run-metadata-rf629-midi-virtual-endpoint-loopback.json` を保存する。
  `os-endpoint-midi-e2e` の AC1/AC6 vertical slice 判定では
  `run-metadata-rf253-midi-virtual-endpoint-host-plugin-e2e.json`、stage 生ログへ転記された
  provenance candidate 行、`midi-loopback` round-trip Pass を保存する。
  `fault-cases` の AC5 判定では stage record/conclusion ではなく
  `fault-cases-results-windows-2025.json` の case-level result、
  `stage-fault-cases-windows-2025.txt` の controller action/復帰記録、
  必要に応じて fault test の `fault_case_result:` 生行を保存する。
- provisioned pair の利用: endpoint 成立後は本書前文どおり
  `mise run test-audio-virtual-endpoint`・`mise run
  test-midi-virtual-endpoint`・`mise run test-os-endpoint-wiring`・`mise run test-os-endpoint-midi-e2e` の入力として
  使う — 独自 driver が供給する `Ruviform Cable Input` /
  `Ruviform Cable Output` / `Ruviform MIDI Loopback` は wiring leg の
  name-stem pair・bidirectional name match 条件を満たす。本 workflow が両 lane
  の実走まで束ねるため、1 回の dispatch で provisioning → probe → lane 実走 →
  evidence upload が通る（Issue #979/#1001 の受入構成）。
- 判定: lane stage の成否は stage record detail で確認するが、語彙は
  stage ごとに異なる。`midi-loopback` は cdb 経由で test 出力が透過する
  ため pass 時も `verdict: Pass — …` 行が detail に残る。一方
  `stream-roundtrip` は cargo test の capture により pass 時
  `verdict=no-verdict-line` となるのが正常で、`test result: ok`
  + round-trip test 本体の `finished in` の非ゼロ秒数（vacuous early return
  は 0.00s）で実走を
  判断する（11 項参照）。Issue #1002 で同一 test target に fault case が
  追加されたため、pass 件数は `1 passed` 固定では読まず、round-trip test
  自体の非ゼロ秒数・pair 検出と、未武装 fault test が inactive terminal
  state に留まることを区別する。`os-endpoint-wiring` は補助 `--nocapture`
  実行のため pass 時も 2 件の `verdict:` 行と
  `run_metadata:`/`run_metadata_path:` 行が生ログに残り、audio/midi の
  result は stage record detail と JSON 本体で読む。`os-endpoint-midi-e2e` も補助
  `--nocapture` 実行のため verdict・`run_metadata:`・provenance candidate 転記行が生ログに残り、
  `result=pass` は `rvfmidi-testsigned-driver-probe` provenance 付きだけが受入証拠になる。
  provenance なし pass は stage `error` で fail-closed に落とす。`fault-cases` は run
  conclusion や stage `ok` を case の Pass と混同せず、case ごとの
  `result`・`expected`・`observed`・`detail` を
  `fault-cases-results-<os>.json` で読む。未 provisioning 環境では verifier
  自身が `verdict: NotRun — NOT_PROVISIONED` を返し、stage は `ok` 記録のまま
  完走する（fail-closed — 未実行・不合格を Pass にしない）。
