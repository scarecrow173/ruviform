---
id: "DESIGN-E2nW52u7DBe6EqaWvY1PRi"
type: "design"
status: "approved"
title: "Issue #629 provisioning runbook and MIDI virtual lane separation design"
created: "2026-11-17"
updated: "2026-11-25"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/knowledge/audio-device-and-midi.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-629-provisioning-runbook.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-provisioning-runbook.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-provisioning-runbook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-629-provisioning-runbook.md"
    - "docs/test-specs/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
  defers: []
  deferred-by: []
---

# Issue #629 provisioning runbook and MIDI virtual lane separation design

## Context

Issue #629 closeout-prep slice。前提 merge 済みの成果 —
1st slice survey+capability probe（`docs/discovery/
issue-629-virtual-endpoint-survey.md`）、2nd slice `midi_output`
port + WinMM `midiOut*` backend、3rd slice `tests/loopback.rs`
round-trip verifier（AC2 仮想 pair 明示 Case）、4th slice
`tests/virtual_endpoint.rs` WASAPI streaming verifier +
`endpoint_probe` 拡張、5th slice `test-audio-virtual-endpoint`
lane 登録 + `EnvironmentKind` 語彙補完 — に対し、残る
headless 部分を閉じる。

本 slice の 4 つの対象:

- **AC6 残部**: `loopback_round_trip`（`tests/loopback.rs`）を
  実行する専用 opt-in lane `test-midi-virtual-endpoint` が
  未登録で、module doc が「`test-midi-real-device` が
  `#[ignore]` を収集するので lane 登録不要」と stale。
  audio 側は 5th slice で `test-audio-virtual-endpoint` が
  登録済み — MIDI 側の対になる lane がない。
- **AC4 残部**: probe・verifier・lane は揃ったが、専用
  Windows runner への provisioning 手順（候補別
  install/cleanup/probe・license 確認・記録責任分担・両 lane
  検証手順・NotRun 解釈）が未文書化。
- **AC5 残部（headless）**: survey doc の Capability Matrix
  は暫定。❓/⚠️ と Open Questions #1-#7 の実測待ちを整理した
  確定版がない。
- **AC7 headless**: consumer Issue snapshot（Synvelis
  #115/#117/#126・AudioLens #127）との照合と機材固有未保証
  範囲の記録がない。

## Scope

### In Scope

- `mise.toml` `[tasks.test-midi-virtual-endpoint]` 追加 +
  `test-midi-real-device` の description/コメント同期
  （run 不変）。
- `docs/development-environment.md` lane 表・opt-in 注記
  追記（`test-midi-real-device` の表行を併設 — 従来未収録の
  doc gap を解消）。
- `crates/ruviform-midi-device/tests/loopback.rs` の doc
  参照更新のみ（挙動不変）。
- 本 doc の「Provisioning Runbook」節 — runbook 本体。
- 本 doc の「Capability Matrix（確定版）」節 — survey doc
  暫定 matrix の確定版。
- 本 doc の「AC7 consumer 照合」節 — consumer AC 対応表と
  機材固有未保証範囲。
- wasapi-endpoint / virtual-endpoint-lane 両 design の
  Deferred Design Concerns 同期 + survey doc Evidence 節
  dated entry 追記 + lane self-run evidence。

### Out of Scope

- 実 provisioning・driver install・license 判断・実走 Pass・
  `.github/**` 統合・AC3 typed 結線・consumer 実受入・
  `real-abi-test-plugin`↔`test-plugin` 語彙 gap — spec の
  Out of Scope と同一。production コード・dependency・
  feature・`unsafe`・test target 構造の変更なし。
  status: rf#725 Phase A で deserialize alias `test-plugin`
  受容を適用済み・emit flip は第2 slice
  （`docs/designs/issue-725-serde-vocab-phase-a.md`）。
  status: rf#725 Phase B で emit flip を実施・検証済み —
  serde emit・`as_str()`・plugin-dock-test assert が全て
  `test-plugin` に統一され語彙 gap は解消（deserialize は
  新旧両綴りを恒久受容 —
  `docs/designs/issue-725-serde-vocab-phase-b.md`・
  `docs/impl/ir/issue-725-serde-vocab-phase-b.md`）。

## Components and Boundaries

### `mise.toml` `[tasks.test-midi-virtual-endpoint]`

`test-midi-real-device` 直後に配置（MIDI 系 lane のまとまり
維持）。`run` は Coordinator 確定どおり
`cargo test -p ruviform-midi-device --test loopback --
--ignored --test-threads=1`。

**収集範囲の設計判断**: `--test loopback` は target 全体を
収集し、`loopback_round_trip`（仮想 pair verifier）と
`real_device_silent_output_smoke`（in-box GS Wavetable
Synth 実 device smoke）の 2 テストを両方実行する。
name filter で `loopback_round_trip` のみに絞る案は不採用:

- libtest の name filter は test 改名時に 0 件一致でも exit 0
  で終わる fail-open 経路になる — target 全体収集なら filter
  による silent skip が起きない。
- `real_device_silent_output_smoke` は対象不在時に自ら
  `verdict: NotRun` を返す fail-closed 設計であり、同居
  させても正直さを損なわない。
- audio 側 `test-audio-virtual-endpoint`（`--test
  virtual_endpoint` 固定）と同じ「test target 単位の lane」
  記法で一貫する。

両テストの依存差（仮想 pair 必須 vs in-box synth）は
description に明記する。`loopback.rs` に `cfg(feature)`
ゲートは存在しないため `--features` は不要（audio 側
`--features real-device` との相違 — Coordinator 確定）。

**`test-midi-real-device` 側**: run コマンド不変
（`-p ruviform-midi-device -p ruviform-audio-plugin-bridge --
--ignored` の全収集のまま）。description を「全 `#[ignore]`
収集の包括 lane — 物理 port 系 + loopback verifier + silent
smoke、仮想 pair 専用は `test-midi-virtual-endpoint`」と
明記する方向で同期する。既存 lane の挙動は変えない。

**lint coverage**: 専用 lint lane は追加しない。既定
`mise run lint ruviform-midi-device`
（`cargo clippy --locked --all-targets -p`）が `loopback`
target を既にカバーする — 5th slice の
`lint-audio-real-input` 流用判断と同型で、feature gate が
無い分さらに単純（Coordinator 確定の設計方針を本 slice で
適用）。

### `docs/development-environment.md`

- lane 一覧表に `test-midi-real-device`・
  `test-midi-virtual-endpoint` の 2 行を
  `test-audio-virtual-endpoint` の次に追加。`test-midi-
  real-device` は従来表にも「未収録の opt-in タスク」注記
  （L99 付近）にも未掲載だった doc gap であり、本 slice の
  lane 分離作業と同じ文脈で併記して解消する。
- opt-in lane 注記節に MIDI 両 lane の bullet を追加
  （`test-audio-virtual-endpoint` bullet の次）。

### `loopback.rs` doc 参照更新

module doc（「`test-midi-real-device` が `#[ignore]` を収集
するので lane 登録不要」— lane 登録後は stale）と
`loopback_round_trip` doc comment（「canonical lane:
`mise run test-midi-real-device`」）を canonical lane
`mise run test-midi-virtual-endpoint` への参照に更新。
`#[ignore]` 属性・verdict 文言・テスト本体は不変。

### stale 参照の同期先

- `docs/designs/issue-629-wasapi-endpoint.md` —
  Deferred Design Concerns の AC5/AC7 項目を「実測は
  provisioned runner に残る・headless 部分（matrix 確定・
  runbook・AC7 記録）は本 slice で解消」に更新。
- `docs/designs/issue-629-virtual-endpoint-lane.md` — 同項目
  + AC6 MIDI lane 分離が本 slice で登録済みである旨を追記。
- `docs/discovery/issue-629-virtual-endpoint-survey.md` —
  Evidence 節に 2026-11-17 dated entry を追記（append-only
  規約 — 既存 entry は編集しない）。lane 分離・runbook・
  matrix・AC7 照合の記録と未解消残件を明記。

## Data and Control Flow

- lane 実行: `mise run test-midi-virtual-endpoint` →
  `cargo test -p ruviform-midi-device --test loopback --
  --ignored --test-threads=1` → `loopback_round_trip` が
  bidirectional name match を探索 → 未 provisioning では
  `verdict: NotRun — NOT_PROVISIONED`、`real_device_
  silent_output_smoke` は GS Wavetable Synth の有無で
  Pass/NotRun を正直に返す。
- evidence 採取: 5th slice と同じく canonical lane 実行
  ログ + `--nocapture` 補助実行（verdict 行採取用）を
  `evidence/rf629/` に保存。canonical `run` に
  `--nocapture` は含めない。
- provisioning フロー（runbook 本体）: runner 準備 →
  license 確認 → 候補 install → probe 実行 → capability
  判定 → lane 実行 → 記録、の順。

## Non-Functional Constraints

- **fail-closed 維持**: 3 lane の self-run が未 provisioning
  で `verdict: NotRun`/`NOT_PROVISIONED` を返すことを確認
  し、Pass を主張しない。`test-midi-real-device` の
  regression（既存挙動不変）を確認する。
- **additive のみ**: `mise.toml` は task ブロック追加 +
  既存 task の description/コメント更新のみ — run コマンド・
  既存 task 定義・3OS 経路は不変。`Cargo.toml`/
  `Cargo.lock` 無変更。
- **3OS lane green**: `loopback.rs` の doc 変更は compile に
  影響しない。`test-midi-*` lane は Windows WinMM 前提の
  manual lane で CI 対象外 — 他 OS の検証経路は無変更。
- **lint**: workspace clippy（pedantic + `-D warnings`）。
  `unsafe`・新規 dependency・feature flag の追加なし。
- **MSRV lane は実行しない**（daily CI 管轄 — task spec
  指定）。
- **docs index（`docs/**/README.md`）は再生成しない** —
  生成物であり Coordinator が統合時に再生成する。新規文書
  による index 未整合は PR 本文の「未対応の作業」に明記。

## Risks and Trade-offs

| Risk | 影響 | 対応 |
| --- | --- | --- |
| `--test loopback` が `real_device_silent_output_smoke` も収集し lane 名と齟齬 | 「virtual 専用」と誤読 | description・docs・本 design で 2 テストの依存差を明記。smoke は in-box synth で fail-closed — name filter による fail-open を避ける判断を記録 |
| `test-midi-real-device` description 変更で既存 lane の意味が変わる | 誤解 | run コマンド不変・description は包括収集の実態を正直に記述するのみ |
| `test-midi-real-device` を表に追加するのは範囲外では | doc 一貫性 | 本 slice の lane 分離と同じ文脈の doc gap — 表への併記で解消し design/PR に判断を記録 |
| runbook が候補を決め打ちに見える | Issue 方針違反 | 候補別手順 + probe 判定主導フロー + 採用未確定の明記で「決め打ちしない」方針を構造で担保 |
| matrix 確定版が「確定済み」と誤読される | 未実測を Pass 扱い | ❓/⚠️・実測待ち項目・Open Questions #1-#7 を明記し「provisioning 実測待ち」を消さない |
| AC7 照合が consumer Pass を主張する | 誠実さ違反 | headless 照合記録のみ — consumer 側の独立 Oracle 実受入は後続と明記 |

## Deferred Design Concerns

- provisioned runner での実 provisioning・driver install・
  license 判断・両 lane の実走 Pass（AC1/AC2/AC5 実測）—
  後続 slice / repo 外工程。再開条件: 専用 runner での
  provisioning 確立。
- `.github/**` CI/runner 統合（定期 opt-in 実行）— 後続の
  外部 gate。POLICIES §5 の opt-in lane 規範に対応。
- AC3 typed 結線 — **解消済み（AC3 wiring slice）**:
  逆方向 dev-dep（`ruviform-audio-test` → 両 device crate、
  test target のみ）を
  `docs/designs/issue-629-test-runtime-wiring.md` §1 で裁定・
  採用し、verdict→`RunMetadata` 対応付けを
  `crates/ruviform-audio-test/tests/os_endpoint.rs` に実装。
  device→audio-test dev-dep は不承認のまま。wiring test の
  canonical lane 登録（`mise.toml`）は後続 slice。
- 採用 endpoint の確定と再現 Case 実装 — provisioning
  実測後。runbook は候補別手順のみで採用を決めない。
- consumer 側の独立 Oracle での実受入判定（Synvelis
  #115/#117/#126・AudioLens #127）。
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
- capability matrix の ❓/⚠️ 項目の実測 — provisioned
  runner 初回実走待ち（runbook の検証手順で収集）。

## Verification Notes

- [ ] `mise run fmt-check` — Pass。
- [ ] `mise run check ruviform-midi-device` — Pass。
- [ ] `mise run lint ruviform-midi-device`（`--all-targets`）
  — Pass。
- [ ] `mise run test ruviform-midi-device`（既定経路 —
  `#[ignore]` は収集されない）— Pass。
- [ ] `mise run test-midi-real-device`（退行確認）—
  `evidence/rf629/` に記録。
- [ ] `mise run test-midi-virtual-endpoint`（本 slice 登録
  lane の self-run）— 未 provisioning のため `verdict:
  NotRun` を `evidence/rf629/` に記録（Pass を主張しない）。
- [ ] `mise run test-audio-virtual-endpoint`（既存 lane が
  挙動不変であることの確認）— `evidence/rf629/` に記録。
- [ ] `mise exec -- prek run --all-files`（`check-toml` が
  `mise.toml` の TOML 検査を兼ねる）— Pass。
- [ ] MSRV lane — NotRun（daily CI 管轄、task spec 指定
  どおり未実行）。

---

# Provisioning Runbook — Windows 仮想 Audio/MIDI Endpoint

Issue `scarecrow173/ruviform#629` の EXT-L3 受入に必要な、
専用 Windows runner への仮想 Endpoint provisioning 手順。
**本 runbook は repo 外工程の手順書であり、driver の導入・
削除・license 判断は provisioning 担当者が実施する。**
リポジトリのコード・CI は driver を自動導入しない —
「開発 PC へ無断 Driver 自動導入」禁止の Issue 方針どおり、
provisioning は専用 Windows Test Runner 上の明示的な手動
工程である。

## 1. 前提と用語

- **対象 runner**: 専用 Windows Test Runner（使い捨て・
  専用機を想定 — SysVAD 例外経路では必須）。開発者の
  通常利用 PC では実行しない。OS は Windows 11 を想定
  （WASAPI/WinMM の標準経路。Windows MIDI Services
  loopback は OS build 依存 — §3.3 参照）。
- **canonical lane**:
  - Audio: `mise run test-audio-virtual-endpoint`
    （`tests/virtual_endpoint.rs` — render→capture streaming
    verifier）
  - MIDI: `mise run test-midi-virtual-endpoint`
    （`tests/loopback.rs` — loopback round-trip verifier +
    GS Wavetable silent smoke）
- **capability probe**（provisioning 判定器・手動実行）:
  - `mise exec -- cargo run -p ruviform-audio-device
    --example endpoint_probe`
  - `mise exec -- cargo run -p ruviform-midi-device
    --example midi_endpoint_probe`
  両 probe は query-only で `verdict` 節に
  `provisioning: PROVISIONED/NOT_PROVISIONED` と各
  capability の `observed`/`NotRun` を出力する。
- **fail-closed 規則**: probe・lane とも環境不備では
  `verdict: NotRun` を返し exit 0。`NotRun` は Pass でも
  Fail でもなく「その環境では未実施」— Pass への昇格は
  provisioning + 実走証拠が揃った場合のみ。

## 2. License 事前確認（必須ゲート — survey Open
##    Question #4）

候補を install する**前に**、各候補の license が「専用
CI/test runner への install と自動 provisioning」を許容
するかを確認し、結果を provisioning 記録に残す。条項の
解釈が不明な場合は install せず Issue #629 に記録して
人間判断を仰ぐ。

| 候補 | license 概要 | runner install への含意 |
| --- | --- | --- |
| VB-CABLE / VB-CABLE Hi-Fi | Donationware — 個人無償、商用は $5/ユーザ | 専用 test runner が商用利用に該当するか確認。該当するなら purchase が前提 |
| VAC | 商用 perpetual（$30 home / $50 business） | purchase 前提。trial は 30 分音声 reminder で CI 不適、Lite は 1 cable/48k/16bit/1 client で不足 |
| loopMIDI（teVirtualMIDI） | 無償・**私的非商用利用のみ**・再配布は書面同意要 | 商用・組織利用の runner では license 違反の可能性 — 利用形態の確認が必須。代替は loopBE / WMS loopback |
| loopBE Audio | freemium | 無償 tier の制約を確認 |
| Windows MIDI Services loopback | OS 付属（追加 install なし） | license 問題なし — OS build・feature rollout が採用条件 |
| SysVAD | Microsoft sample（WDK） | test-signing 前提の例外経路 — 専用機 + WDK 環境が前提 |

出典（survey Evidence 節の一次情報）: VB-Audio 公式
shop/download/reference manual、Muzychenko VAC 公式
purchase/manual、Tobias Erichsen 公式 loopMIDI/virtualMIDI、
Microsoft Learn SysVAD sample、Microsoft MIDI
docs（`docs/discovery/issue-629-virtual-endpoint-survey.md`
の `references` と Evidence 節を参照）。

## 3. 候補別 provisioning 手順

採用 endpoint は**決め打ちしない** — 候補ごとに install →
probe → 判定を行い、capability matrix の要件を満たす最初の
候補を採用する。複数候補の併用は endpoint 名で区別可能
（probe の pair candidate 表で確認）。

### 3.1 Audio: VB-CABLE / VB-CABLE Hi-Fi

- 入手: VB-Audio 公式サイトの download（署名済み kernel
  driver 同梱 zip）。license は §2 の donationware 条項を
  確認。
- install: zip 展開 → `VBCABLE_Setup_x64.exe` を
  **管理者権限**で実行 → **要 OS 再起動**（driver install
  の既知要件 — runbook に必須として記録）。
- 観測: 再起動後 "CABLE Input"（render）/"CABLE Output"
  （capture）の endpoint pair が WASAPI/MME/KS/DirectX に
  出る。Hi-Fi は SRC なし bit-perfect・最大 24bit/384kHz・
  in/out 同一 sample rate 必須 — format 厳密 Case の補助。
- 制約: endpoint は静的（hotplug なし — install/uninstall
  のみ）。内部 SR 48kHz・SRC あり（標準版）。
- cleanup: アンインストーラ実行 + 再起動。driver 残存は
  `pnputil /enum-drivers` で確認。

### 3.2 Audio: VAC（Virtual Audio Cable）

- 入手: Muzychenko 公式 purchase page（署名済み・reseller
  cert NTONYX）。license は §2 の商用条項を確認。
- install: installer を管理者権限で実行 — **通常再起動
  不要**。cable 数・format 上限（384kHz/32bit/32ch）・
  multi-client・stream format limiting・channel remap・
  position/clock register を control panel で設定。
- 観測: "Line N (Virtual Audio Cable)" endpoint が WASAPI
  に出る。
- 採用動機: clock-domain/format-switch 系 Case で他候補に
  ない制御面（survey capability matrix 参照）。
- cleanup: control panel で cable 削除 → uninstaller。
  設定残存（registry/cable 定義）は手動確認。

### 3.3 MIDI: loopMIDI（teVirtualMIDI）

- 入手: Tobias Erichsen 公式サイト（署名済み kernel
  driver）。license は §2 の**私的非商用限定**を必ず確認
  — 商用・組織 runner では利用不可の可能性が高い。
- install: installer を管理者権限で実行。**port は app
  実行中のみ存在**・ユーザ単位・動的作成/破棄 — 検証時は
  loopMIDI app を常駐させ、port を作成しておく。
- 観測: 作成した port が midiIn/midiOut **双方**に同名で
  出る（bidirectional pair — `loopback_round_trip` の
  name-match candidate 条件）。
- hotplug 特性: app 終了で port 消失 → device-loss 系
  Case の再現源になり得る（capability matrix ⚠️ 項目）。
- cleanup: app 終了で port 消失を確認 → uninstaller。
  driver 残存は `pnputil` で確認。

### 3.4 MIDI: loopBE Audio

- loopMIDI と同等の bidirectional pair を提供する
  freemium 候補。license tier の制約を §2 で確認のうえ
  loopMIDI が使えない場合の代替として評価する。

### 3.5 MIDI: Windows MIDI Services loopback（OS-native）

- **追加 install なし** — Win11 25H2+ で提供される in-box
  component。利用可否は OS build・feature rollout 依存
  （survey Open Question #3 — 本ホスト build 26300 では
  loopback 未作成を実測）。
- 作成: PowerShell `New-MidiLoopback`（MIDI 2.0 A/B pair）
  または `New-MidiBasicLoopback`（MIDI 1.0 単一
  endpoint）。transient（service restart で消える）または
  `-SaveToConfiguration`（永続化）。
- 観測: WinMM/WinRT API は新 service へ repoint
  （`wdmaud2.drv` 互換層経由で legacy app に見える）。
- 採用条件: runner の OS build で cmdlet が利用可能で
  あることを probe で確認してから採用判定。

### 3.6 例外経路: SysVAD 系 fallback

COTS 候補で「再現可能」と判定すべき capability
（device-loss 強制・xrun 決定的注入・permission denial 完全
制御・format switch 境界）が `NotRun`/`Unsupported` に
留まる場合のみ。前提: WDK build 環境・
`bcdedit /set TESTSIGNING ON` + cert install + 再起動・
Secure Boot/BitLocker 整合・**専用機**（test-signing の
ため通常機では使わない）。driver を自作するため hotplug・
format・loss を実 driver 層で制御できる唯一の選択肢だが、
署名・配布・保守コストが大きい — 採用は別途 ADR/Issue で
判断し、本 runbook の標準経路には含めない。

### 3.7 MIDI: ruviform 独自 driver（RvfVirtualAudio 同梱）

**Issue #978 で実装 — GitHub-hosted `windows-2025`（x64
test-signing）での標準経路。** §3.3〜3.5 の COTS/OS-native
候補は license（loopMIDI 私的非商用・loopBE freemium
tier）または runner 到達不能（WMS: Server SKU 非対象・
`windows-11-arm` でも provisioning 経路不在を rf#962 で
実測確定）のため、Slice C で独自 driver 経路に一本化した
（設計: `issue-952-virtual-endpoint-driver.md` Slice C 実装
記録）。

- 構成: `drivers/rvf-virtual-audio/` の Slice B package
  に DMus MIDI miniport（WDK 8.1 DMusUART 由来・MS-LPL）を
  同梱 — audio endpoint と同一 INF・署名・install 経路を
  再利用する。
- 観測: install 後、WinMM midiIn/midiOut **双方**に同名
  `Ruviform MIDI Loopback` が列挙される（DMus filter の
  render/capture 両 pin が同一 pin-name GUID
  `{714b96c3-8003-41c2-b673-1b086cbdb3db}` を共有し、
  INF `MediaCategories` が上記文字列へ結び付ける —
  `loopback_round_trip` の bidirectional name-match
  candidate 条件を満たす）。
- loopback 意味論: midiOut への send byte は kernel 内
  FIFO 経由でそのまま midiIn へ返る（実 cable 同型 —
  capture 未 open 時は送信 byte は破棄）。
- install/probe/lane 手順: `drivers/rvf-virtual-audio/
  README.md` と `virtual-audio-driver-probe.yml` の
  `midi-loopback` stage（`windows-2025`・test-signed・
  `mise run test-midi-virtual-endpoint` 実走）に従う。
  Issue #979 で `midi-endpoint-probe` stage も追加 — `midi_endpoint_probe`
  example が WinMM midiIn/midiOut 列挙・loopback 候補・
  `provisioning=` を stage record に記録する（`endpoint-probe` の
  MIDI 版・spec 版 runbook 追記節の stage 一覧が正本）。

### 3.8 Audio: ruviform 独自 driver（RvfVirtualAudio）

**Issue #977 で実装 — GitHub-hosted `windows-2025`（x64
test-signing）での標準経路。** §3.1〜3.2 の外部 COTS driver は
license・配布・再起動の摩擦から、Slice B で独自 driver 経路に一本化
した（設計: `issue-952-virtual-endpoint-driver.md` Slice B 実装記録）。

- 構成: `drivers/rvf-virtual-audio/`（SysVAD 派生・MS-PL）が render
  `Ruviform Cable Input` / capture `Ruviform Cable Output` の
  MMDevice endpoint 1 pair を生成する。stage 2 で kernel loopback
  （render stream の DMA buffer を capture stream が読む経路）を実装
  し、speaker→capture の音声経路が成立（rf977 attempt-21 で
  round-trip Pass — `issue-952-virtual-endpoint-driver.md` Slice B
  loopback 構造節参照）。
- 観測: install 後、`endpoint_probe` が `Ruviform Cable *` の
  render↔capture pair を `pair_count=1` で検出する（name-stem
  match・unconfirmed の candidate 報告 — round-trip の確定証拠は
  `stream-roundtrip` stage の `tests/virtual_endpoint.rs` が担う）。
- install/probe/lane 手順: `virtual-audio-driver-probe.yml` が
  install 全 stage（`env-build-surface`〜`devnode-create`）→
  `endpoint-probe`→`midi-endpoint-probe`→`stream-roundtrip`
  （`mise run test-audio-virtual-endpoint` 実走）→ `midi-loopback` →
  `os-endpoint-wiring` → `os-endpoint-midi-e2e` → `fault-cases` → `cleanup` を束ねる。stage 一覧の正本は spec 版
  runbook 追記節（`docs/specs/issue-629-provisioning-runbook.md`）。

## 4. Provisioning 判定フロー（probe 主導）

1. **runner 環境記録**: OS version/architecture/
   `winver` build 文字列を記録（AC4 の OS Version 記録
   要件）。
2. **license 確認**: §2 の表で採用候補の license を確認・
   記録。
3. **install**: 採用候補の §3.x 手順を実施（再起動要否を
   記録）。
4. **probe 実行**: 両 probe を実行し出力を保存。
   - Audio probe: endpoint 列挙・state・format 交渉・
     pair candidate 表・driver 署名（best-effort）・
     microphone 権限・cleanup note を確認。
   - MIDI probe: WinMM input/output port 列挙・loopback
     candidate（bidirectional name match）・WMS
     availability を確認。
5. **判定**: probe の `verdict` が `PROVISIONED` で pair
   candidate が出れば次へ。`NOT_PROVISIONED` なら install
   未反映（再起動漏れ・app 未起動等）を疑い §3 を見直す。
   候補が要件を満たさなければ次候補へ（決め打ちしない）。
6. **lane 実行**: §5 の手順で両 lane を実行。
7. **記録**: §6 の記録項目を揃えて Issue #629 の impl
   record / evidence に残す。

## 5. Lane 検証手順

### 5.1 Audio lane

```text
mise run test-audio-virtual-endpoint
```

- `tests/virtual_endpoint.rs` の
  `render_capture_round_trip` が provisioned pair を探索し、
  render→capture の signal 往復を独立 oracle（marker 検出・
  level 比・Goertzel 基音比・callback 契約）で照合する。
- pair 未検出: `verdict: NotRun — NOT_PROVISIONED`（exit 0、
  fail-closed）。GS Wavetable 等の実 device があっても
  virtual pair がなければ Pass にはならない。
- verdict 行は libtest が capture するため、証拠採取時は
  `--nocapture` が必要（canonical `run` には含めない —
  補助実行 `cargo test -p ruviform-audio-device --features
  real-device --test virtual_endpoint -- --ignored
  --test-threads=1 --nocapture` で採取）。

### 5.2 MIDI lane

```text
mise run test-midi-virtual-endpoint
```

- `tests/loopback.rs` の `loopback_round_trip` が
  bidirectional name match の pair を探索し、Note/CC/
  PitchBend/NoteOff の固定 sequence を send→receive で
  payload・順序・timestamp 不変条件を独立 expected list と
  照合する。`real_device_silent_output_smoke`（GS
  Wavetable Synth 実 device smoke）も同時収集される。
- pair 未検出: `verdict: NotRun — NOT_PROVISIONED`
  （fail-closed）。name match は candidate であり virtual
  の証明ではない — Pass は OS-L1「実 OS API round-trip」
  証拠であり `environment_kind=virtual-os-endpoint` を
  主張しない（verifier 自身の evidence note どおり）。
- verdict 行の capture 注意は audio lane と同じ。

### 5.3 退行 lane

- `mise run test-midi-real-device` — 全 `#[ignore]` 収集の
  包括 lane。`test-midi-virtual-endpoint` 登録後も run
  不変であることを確認する regression lane。
- `mise run test-audio-virtual-endpoint` — audio 側 lane
  が挙動不変であることの確認。

## 6. 記録責任分担（AC4 の記録要件）

AC4「OS Version/Architecture/Driver/署名/権限/
Provisioning/cleanup を記録し、未 Provision 環境は
NotRun として fail-closed に報告」に対応する記録の
分担:

| 記録項目 | 担当 | 手段 |
| --- | --- | --- |
| OS Version/Architecture | runner 手動 | `winver` / `cmd /c ver` / probe 出力の環境節 |
| Driver 名・version・INF・署名 | probe best-effort + runner 残件 | probe の `Win32_PnPSignedDriver` + `Get-AuthenticodeSignature`（PowerShell 経由・失敗は `NotRun` note）。SetupAPI 面の機械的検証（driver package 一覧・署名状態の網羅確認）は runner 側で `pnputil`/`Get-WindowsDriver` 等により補完 |
| 権限（microphone privacy 等） | probe best-effort + runner 残件 | probe の ConsentStore/DeviceAccess 照会（best-effort）。privacy キーの体系的収集（全ユーザ・全 scope）は runner 側で手動確認 |
| Provisioning 状態 | probe verdict | `provisioning: PROVISIONED/NOT_PROVISIONED` |
| cleanup | runner 手動 + probe note | probe は query-only で永続状態を作らない旨を verdict に明記。driver/endpoint の撤去確認は runner 側で実施・記録 |
| lane 結果 | lane 実行ログ | `verdict:` 行・exit code・evidence ファイル |

## 7. NotRun の解釈

- `verdict: NotRun` は「その環境では未実施」であり、
  Pass でも Fail でもない（POLICIES §3 — Required の
  `NotRun` は当該機能の受入未達）。
- 原因切り分け順: (a) probe で pair candidate が出るか
  → (b) 出るなら lane を再実行 → (c) 出ないなら
  provisioning 未反映（再起動漏れ・app 未起動・install
  未実施）を疑う → (d) それでも出ないなら候補の capability
  不足として matrix の ❓/⚠️ 項目に記録し次候補を評価。
- lane/compile/lint の Pass を `virtual-os-endpoint` の
  Pass や実 OS L3 実走 Pass と主張しない — provisioning
  + 実走証拠が揃うまで Required Case は `NotRun` のまま。

## 8. Capability Matrix（採用経路 `RvfVirtualAudio` + `RvfMidi` 実測確定版）

本節の第一表は、採用経路である ruviform 独自 virtual audio endpoint
（`RvfVirtualAudio`）と MIDI loopback miniport（`RvfMidi`）の実測 matrix
である。測定 lane は GitHub-hosted `windows-2025` x64（probe 観測 OS
`10.0.26100.33828`）に限定し、他 OS/build へ一般化しない。凡例:
✅=採用 lane で実測済み / ⚠️=一部は実測済みだが明確な限界がある /
`NotRun`=本 slice で実測していない / `Unsupported`=bounded な実験では
現行構成で再現・観測できないと確定。✅ は必ず evidence 列の run id または
repo 内 evidence/文書へ結び付く。COTS 候補の比較は後掲の歴史資料であり、
採用経路の判定と混ぜない。

| Capability | 採用経路の判定 | 実測根拠・限界 |
| --- | --- | --- |
| provisioned audio endpoint 列挙 | ✅ | run `45543608858`・`evidence/rf979/attempt1-endpoint-probe-windows-2025.txt`: `Ruviform Cable Input`/`Ruviform Cable Output` が active で 2 endpoint、pair_count=1。非 active state を同時に列挙した証拠ではない。 |
| endpoint identity/state 取得 | ✅ | 同じく run `45543608858`: MMDevice id・state=active・production port id（`wasapi:...`）を両方向で観測。MIDI input 側 interface id `\\?\root#media#0000#...\midiloopback` を `attempt1-midi-endpoint-probe-windows-2025.txt` で観測。 |
| format 交渉（shared mode） | ✅ | 同じく run `45543608858`: render mix format 2ch/48k extensible、capture mix format 1ch/48k extensible。両 endpoint で `f32/2ch/48k`・`f32/2ch/44.1k`・`s16/2ch/48k` が `IsFormatSupported(shared)` supported。`ppclosestmatch` は有効 out-param を渡す規約で測定。`GetStreamLatency` は両 endpoint `0x88890001` refusal であり、latency 値の取得証拠ではない。 |
| capture/render open・stream | ✅ | run `45543608858` `stream-roundtrip`: `render_capture_round_trip` exit 0・非ゼロ秒（3.48s）で完走。run `45555696425`・`evidence/rf1001/attempt2-run-metadata-rf629-audio-virtual-endpoint-render-capture.json`: `result=pass`・`level=ext-l3`・`environment_kind=virtual-os-endpoint`。 |
| callback/stream viability | ✅ | run `45555696425` の audio wiring leg: marker/RMS oracle、callback、timestamp epoch grace 契約を通して Pass。rf#1001 attempt1 の mid-epoch regression は fail-closed 動作として記録済み。deep latency/position oracle の全項目を matrix capability とみなさない。 |
| MIDI Note/CC/PitchBend/timestamp/order round-trip | ✅ | run `45543608858` `midi-loopback`: `loopback_round_trip` と silent output smoke が exit 0。run `45555696425`・`evidence/rf1001/attempt2-run-metadata-rf629-midi-virtual-endpoint-loopback.json`: `result=pass`・`level=ext-l3`・`environment_kind=virtual-os-endpoint`。 |
| Windows MIDI Services 非依存 | ✅ | run `45543608858` `midi-endpoint-probe`: `New-MidiLoopback: absent`・`midisrv: absent` の環境で `Ruviform MIDI Loopback` midiIn=1/midiOut=2（GS synth を含む）と round-trip が成立。WMS は採用経路の前提ではない。 |
| license/provenance | ✅ | runtime capability ではなく文書確認: `drivers/rvf-virtual-audio/PROVENANCE.md` で audio 部は SysVAD 派生 MS-PL、MIDI miniport 部は DMusUART 派生 MS-LPL v1.1 と区別し、license 全文同梱と platform limitation を記録済み。 |
| virtual hotplug / devnode loss stimulus | ⚠️ | run `45575005948` `device_loss_audio`/`stage-fault-cases`: open stream 中の `Disable-PnpDevice`/`pnputil /disable-device` は拒否されたが、remove/recreate fallback による devnode removal stimulus は成立し、MIDI port disappearance が独立に観測された。recreate は新しい instance `ROOT\MEDIA\0001` を bind したものの、audio/MIDI endpoint surface は 45 秒の bounded polling 内に再列挙されなかった。したがって PnP removal stimulus は再現できるが、clean な disable/enable hotplug 制御または透過的 recovery とはみなさない。 |
| audio device-loss invalidation + recovery | Unsupported | run `45575005948` `device_loss_audio=Unsupported`: removal が stream 生存中に挟まったにもかかわらず `StreamInvalidated`=0・control-path `DeviceNotFound` 相当=0・`ClockDiscontinuity`=0。now/stop/start 制御経路でも typed invalidation は surface しなかった。typed invalidation は attempt1-5 で一貫して未観測だが、recovery は attempt1 `45567369491` で clean reopen（callbacks=38）が成功し、attempt3-5 では `no input devices on this host` となった不安定な境界である。OS removal と MIDI disappearance は stimulus 成立の代替 evidence だが、typed invalidation + 保証された recovery を組み合わせた capability は現行構成では成立しない。driver/adapter 側の PnP invalidation 契約を本 slice では実装しない。 |
| MIDI device-loss disappearance observation | ✅ | run `45575005948` `midi_device_loss_observation=Pass`: before=True/exit0、during=False/exit0、after=False/exit0。正規 oracle は少なくとも disappearance を観測することであり、`Ruviform MIDI Loopback` が removal 中に WinMM enumeration から消えたことを確認した。open 中 Rust MIDI adapter への非同期通知を観測した証拠ではない。 |
| MIDI reappearance after remove/recreate | ⚠️ | run `45575005948`: recreate と AudioEndpointBuilder/Audiosrv restart 後、45 秒（10 probe）の bounded polling でも midiIn=0/midiOut=1（GS synth のみ）のままで after=False。ただし attempt1 `45567369491` では audio reopen recovery（callbacks=38）が成功しており、attempt1 の MIDI probe は measurement bug で invalid なため、reappearance 不能とは断定しない。現行 evidence では再出現の再現性が確立していない不安定な境界である。 |
| microphone permission denial + restore | ✅ | run `45575005948` `permission_denial_audio=Pass`: microphone consent Deny + service restart 後の capture open が exact `DeviceError::Backend("permission_denied")`。Allow restore 後に `Ruviform Cable Output` が再列挙され、clean open/start/stop・callbacks=39 を確認。どの consent layer が効いたかは単一層へ断定せず、runbook の OS 制御面として記録する。 |
| xrun / underrun / overrun 注入と観測 | Unsupported | run `45575005948` `xrun_probe`: active stream（callbacks=423）に bounded user-mode starvation（2 spinner・900ms）を与えても `DeviceError::Xrun`・`ClockDiscontinuity`・`StreamInvalidated`・その他 error が 0。native input は discontinuity を `ClockDiscontinuity` に写し、cpal WASAPI output には typed xrun 発火経路がない現行契約では、OS/user-mode 制御だけでは typed xrun evidence を作れない。決定的注入面と adapter 観測契約は `scarecrow173/ruviform#1009` へ分離。 |
| format switch（streaming 中の動的切替） | NotRun | 本 slice では OS/driver 制御による streaming 中 format switch harness を実装せず、実測していない。採用経路で未実測のまま ✅ にしない。 |
| loopback latency の数値測定 | ⚠️ | render→capture round-trip 自体は ✅。ただし `GetStreamLatency` は endpoint refusal（`0x88890001`）、canonical lane の passing output は capture され、本 slice は bounded な数値 latency oracle を追加していない。したがって遅延値・制御性は未確定であり、signal viability の証拠と混同しない。 |

### survey Open Questions #1-#7 の closure

| OQ | closure | 根拠 |
| --- | --- | --- |
| #1 VB-CABLE disable/enable・既定 device 切替で `IMMNotificationClient` が発火するか | **採用経路では NotApplicable** | VB-CABLE は採用経路ではない。代替となる Rvf devnode loss/recovery は rf#1002 fault case で実測し、typed audio invalidation/recovery は Unsupported、MIDI disappearance は Pass という境界を上表へ記録した。 |
| #2 loopMIDI port 出現/消失が `midiInGetNumDevs` に動的反映されるか | **採用経路では NotApplicable** | loopMIDI は採用経路ではない。代替となる `Ruviform MIDI Loopback` disappearance/reappearance は rf#1002 `midi_device_loss_observation` で実測し、disappearance=Pass・reappearance は再現性未確立（⚠️）の境界を上表へ記録した。 |
| #3 対象 runner で Windows MIDI Services が利用可能か | **採用経路では NotApplicable（実測済み）** | run `45543608858` で `New-MidiLoopback`/`midisrv` absent を観測。採用経路は WMS に依存せず、この状態で MIDI round-trip が成立した。 |
| #4 VB-CABLE/loopMIDI license が CI provisioning を許容するか | **採用経路では NotApplicable** | 外部 COTS driver を採用しない。Rvf 側 license/provenance は `drivers/rvf-virtual-audio/PROVENANCE.md` で確定済み。 |
| #5 `IsFormatSupported` の `ppclosestmatch` 必須挙動は build 依存か | **採用 lane で実測解消（一般化しない）** | windows-2025 laneでは有効 out-param 規約で 3 format query が supported を返した（run `45543608858`）。実装は常に有効 out-param を渡す。NULL 拒否が他 buildで恒常かは本 slice の capability ではなく、一般化しない。 |
| #6 capture permission denial を仮想 endpoint で再現できるか | **実測で解消** | run `45575005948` `permission_denial_audio=Pass` で Deny 後 capture open が exact `Backend("permission_denied")`、Allow restore 後の clean reopen（callbacks=39）まで確認した。 |
| #7 `midiOutMessage` crash が GS synth 固有か winmm 全般か | **採用 laneの境界として解消（一般化しない）** | run `45543608858` で output interface-id query は host の `Microsoft GS Wavetable Synth` 側 crash により NotRun、Rvf input interface-id queryは成功。採用経路の round-trip は output interface-id query を必要としない。全 driver/WinMM 一般性は断定しない。 |

### 参考（歴史・比較資料）: COTS/SysVAD matrix

以下は採用経路確定前の比較資料であり、判定列は採用経路の実測値ではない。
❓/⚠️ の語は当時の公開情報・probe 設計ベースの見込みであり、採用経路
`RvfVirtualAudio` + `RvfMidi` の受入判定には使わない。特に SysVAD 列の ✅ は
upstream sample が持つ理論上の実装可能性であって、ruviform 採用 driver での
provisioned 実測を意味しない。

| Capability | VB-CABLE/Hi-Fi | VAC | loopMIDI/WMS loopback | SysVAD |
| --- | --- | --- | --- | --- |
| endpoint 列挙（全 state） | ✅ | ✅ | ✅（WinMM caps） | ✅ |
| endpoint identity/state 取得 | ✅（`GetId`/`GetState`） | ✅ | ✅（caps/interface-id） | ✅ |
| format 交渉（shared mode） | ✅（endpoint format 変更 + IsFormatSupported） | ✅（format 上限広い） | n/a（MIDI に format 概念なし — UMP/byte stream） | ✅（INF/driver で制御可） |
| capture/render open・stream | ✅（要 provisioning） | ✅ | n/a | ✅ |
| callback/stream viability | ✅ 見込み | ✅ 見込み | ✅（`midiInStart` callback） | ✅ |
| **virtual hotplug**（endpoint 出現/消失） | ❌（静的 driver endpoint — install/uninstall のみ） | ❌（同上） | ✅ **loopMIDI は app 起動/終了で port 出現/消失** — 最良の hotplug 再現源。WMS transient loopback も同等 | ✅（devcon/PnP で再現可） |
| **format switch**（streaming 中の format 変更） | ⚠️（endpoint format は OS 側設定変更で可・streaming 中の動的切替は driver 依存）❓ | ✅（format limiting/remap 制御あり） | n/a | ✅ |
| **device-loss / invalidation** | ⚠️（無効化・取り外しは PnP 経由 — 再現は限定的）❓ | ⚠️ 同左 | ⚠️（loopMIDI app kill で port 消失 → `MM_MIM_CLOSE`/device vanished を再現可能か要 probe） | ✅（driver が失敗を注入可能） |
| **permission denial** | ⚠️（Windows の microphone privacy 設定で capture を拒否可能 — OS 標準機能で再現可）❓ | 同左 | n/a | ✅ |
| **xrun / latency** | ⚠️（VB-CABLE は内部バッファ 7168 frame・latency 調整可。決定的 xrun 注入は困難） | ⚠️（clock register で位置制御は可・xrun 強制は ❓） | n/a | ✅（driver で drop 注入可） |
| **loopback latency**（signal→capture の往復遅延） | ✅（internal SR 48kHz・latency パラメータあり・実測可能） | ✅ | ✅（timestamp は WinMM ms domain — adapter が `MidiPortCapabilities.clock_domain` で報告） | ✅ |
| **MIDI Note/CC/PitchBend/Timestamp/Order** | n/a | n/a | ✅（loopback pair 経由 — send path は 2nd slice `scarecrow173/ruviform#658` で実装済み。round-trip verifier は 3rd slice `tests/loopback.rs`、provisioned pair 前提で未 provision 環境は `NotRun`） | n/a |

## 9. AC7 consumer 照合（headless 記録）

Issue `scarecrow173/ruviform#629` の AC7「Synvelis
#115/#117/#126 と AudioLens #127 の影響範囲を照合し、
機材固有動作の未保証範囲を残す」に対応する headless
照合結果。consumer 側 Issue 本文 snapshot
（`G:/repositry/audio-products/.devin/tmp/issue-snapshot/
scarecrow173/{synvelis,audio-lens}/issues/*.md`）との照合
に基づく。**本節は照合記録であり、consumer 側の独立
Oracle での実受入判定ではない — consumer 実受入は後続。**

### 9.1 consumer AC との対応

| consumer Issue | #629 への要求（snapshot より） | #629 側の対応状況 |
| --- | --- | --- |
| `scarecrow173/synvelis#115`（RCD-01 録音） | EXT-L3: 「Ruviform #629 の OS が列挙・Open できる仮想 WASAPI Endpoint→本番 Callback」を通し、RF #628 固定 PCM と Synvelis 独立 sample/byte/Clock/Latency/Recovery/Journal/Arm/Punch Oracle を照合。物理 Audient iD4 の 10/10 は履歴的証拠、追加の物理 USB 抜去・ADC/DAC 特性は通常必須から外し未保証と明記済み | audio 側 verifier（`test-audio-virtual-endpoint`）が本番 WASAPI backend 経路をカバー。consumer 側独立 Oracle での L3 実走は consumer 責務・後続 |
| `scarecrow173/synvelis#117`（RCD-03 Monitoring/Punch/Latency UX） | Software Monitoring/Auto/Off/On/Punch/Latency は #628 仮想 Audio/Clock・#629 Windows 実 WASAPI 仮想 Endpoint・#631 実 GUI Runner で検証。物理 Direct Hardware Monitor のアナログ混合経路は仮想のみで実測不可 → 自動受入範囲外/未保証として明示済み | audio 側 lane が前提経路を提供。Direct Hardware Monitoring は consumer 側で未保証明記済み（下記 9.2） |
| `scarecrow173/synvelis#126`（TIM-02 MIDI 1.0/SMF） | SMF/Native timestamp/Chase は #628 決定的 MIDI Event/Clock で検証、Windows 実 WinMM API を通る入力は #629 の仮想 MIDI Endpoint で実走。物理 MIDI 機材は通常必須でなく、物理機材固有の Timestamp 特性は未保証。MIDI2 は #317 future | MIDI 側 lane（`test-midi-virtual-endpoint`）が WinMM loopback pair 経路をカバー。consumer 側独立 SMF/Timing Oracle での L3 実走は後続 |
| `scarecrow173/audio-lens#127` | device boundary を消費しない製品のため #629/#630 は AL 非提供機能として **NotApplicable** 方針（AL が MIDI/Plugin 機能を製品提供する場合のみ Required へ昇格 — `al127-next.explore.json` 記録済み） | AL 側の NotApplicable 判定を確認 — #629 の成果は AL の受入を阻害しない。AL 独立 Oracle での確認は consumer 責務 |

### 9.2 機材固有動作の未保証範囲

POLICIES §2.1「物理適合性の限界」に基づき、仮想
Endpoint の Pass は以下を保証しない — consumer・利用者が
正式サポートを宣言する場合や不具合調査で必要になった
時点で個別 Case を定義する:

- **物理 USB 抜去**（device removal の物理経路 — 仮想
  endpoint の disable/PnP 経由とは別経路）。
- **ADC/DAC アナログ特性**（物理変換の bit-exact・
  アナログ遅延 — 仮想 loopback の digital 往復とは別）。
- **機種固有 driver 不具合**（特定機材の driver 依存
  挙動 — 共通仮想 Fixture へ回帰再現を還元できないもの）。
- **物理 Direct Hardware Monitoring**（Synvelis #117 が
  未保証と明記済み — 仮想環境ではアナログ混合経路を
  実測できない）。
- **UMP/MPE・MIDI 2.0 高度機能**（`scarecrow173/
  ruviform#317` future — 現行 verifier は MIDI 1.0
  byte stream のみ）。
- **物理 MIDI 機材固有の Timestamp 特性**（Synvelis #126
  が未保証と明記済み — WinMM ms domain の仮想 pair
  timestamp は物理機材の挙動を代表しない）。

これらは「問題は可能な範囲で共通仮想 Fixture へ回帰再現を
還元し、再現不可なら未保証範囲を記録する」POLICIES §2.1
の方針に従い、本節で未保証として記録する。

## 10. 追記（2026-11-18）— wiring leg canonical lane の登録

§1 の canonical lane 一覧に続く残部として、AC3 wiring slice
（`docs/designs/issue-629-test-runtime-wiring.md`）で追加された
`crates/ruviform-audio-test/tests/os_endpoint.rs` の `#[ignore]`
leg 2 件（`audio_virtual_endpoint_round_trip`・
`midi_virtual_endpoint_loopback`）の canonical lane が
os-endpoint-lane slice（`docs/designs/
issue-629-os-endpoint-lane.md`）で `mise.toml` に登録された —
`mise run test-os-endpoint-wiring`（`cargo test -p
ruviform-audio-test --test os_endpoint -- --ignored
--test-threads=1`）。本 runbook の provisioning 手順・
capability probe 判定・lane 検証手順・NotRun 解釈は本 lane にも
そのまま適用される — provisioned pair が揃えば wiring leg が
実際に endpoint を stream した run のみ
`level=ext-l3`/`environment_kind=virtual-os-endpoint` の
`RunMetadata` を emit する（fail-closed 契約は device
verifier と同一）。詳細は
`docs/designs/issue-629-os-endpoint-lane.md` を参照。

## 11. 追記（2026-11-25）— wiring leg provisioned 実走 stage

Issue #1001 Slice F では、§10 の canonical lane を workflow の
provisioned 環境へ接続する。設計判断は次のとおり。

- **stage 配置**: `os-endpoint-wiring` は `midi-loopback` 後・
  `cleanup` 前に置く。audio capture は microphone privacy consent
  gate を通る必要があるが、`stream-roundtrip` が同一 job 内で
  DeviceAccess/ConsentStore/AppPrivacy の Allow 化と
  AudioEndpointBuilder/Audiosrv restart を済ませるため、wiring
  stage では複写しない。配置を変えないことが consent provisioning
  の共有を成立させる。
- **canonical lane と補助証拠の分離**: canonical lane は従来どおり
  `mise run test-os-endpoint-wiring`（`--nocapture` なし）である。
  workflow stage は verdict と `run_metadata:`/`run_metadata_path:`
  を artifact 生ログへ表面化させる必要があるため、補助証拠として
  `mise exec -- cargo test -p ruviform-audio-test --test os_endpoint --
  --ignored --test-threads=1 --nocapture` を直接実行する。`mise.toml`
  の lane 定義は変えない。
- **RunMetadata artifact**: test は `CARGO_TARGET_TMPDIR` 配下に
  `run-metadata-<case>.json` を書く。Cargo は integration test へ
  同ディレクトリを compile-time env として供給するが runtime env が
  常にあるわけではないため、emit 側は runtime env を優先しつつ
  compile-time `option_env!("CARGO_TARGET_TMPDIR")` を fallback に
  使う。workflow は stderr の `run_metadata_path:` 行から JSON 本体を
  workspace 内 `os-endpoint-run-metadata-<os>/` へ copy し、reset と
  upload artifact path に含める。copy 不能は捨てず stage `error` に
  写す。
- **stage 判定**: stage record は従来どおり absence-is-data /
  fail-closed 語彙に従う。両 leg の Pass/NotRun は観測終端なので
  stage `ok`、Fail・crash・verdict 未到達・RunMetadata 未到達/不正・
  artifact copy 不能は `error`。ただし stage `ok` は rf#1001 AC3 の
  Pass 証拠ではなく、受入判定は audio/midi 両 RunMetadata JSON の
  `result:"pass"`・`level:"ext-l3"`・
  `environment_kind:"virtual-os-endpoint"`・repo/SHA を読む。
- **audio timestamp epoch oracle**: rf#977 で確定した shared-mode
  render stream の timestamp epoch 契約を contract 級 oracle の範囲で
  wiring legへ移植する。stop/start 境界で input/output 両方向の
  baseline を split し、callback index を bounded regression slots
  （32）へ記録する。epoch 先頭 `EPOCH_START_GRACE_CALLBACKS=24`
  内の regression のみ transient として raw count から引き、
  mid-epoch regression と slot overflow は fatal のまま残す。device
  verifier の latency/position/tone/xrun など深い oracle は複製しない。
  分類は epoch 1 grace・epoch 2 grace・mid-epoch fatal・slot overflow
  fatal を区別する deterministic test で固定する。

## 12. 追記（2026-11-25）— fault case 実測確定（Issue #1002 Slice G）

`fault-cases` stage の正規判定は workflow conclusion ではなく
`fault-cases-results-windows-2025.json` の case-level result で行う。最終
fault attempt は branch-ref run `45575005948` / SHA
`6265671a50541a3ace2559bfb6f1ad277fc89691` で、stage record は
`fault-cases=ok`・`harness_problems=0` だった。同 run では既存の
`stream-roundtrip`・`midi-loopback`・`os-endpoint-wiring`（audio/midi 両
RunMetadata Pass）も完走し、fault stage が先行 lane を壊していないことを
確認した。ここでの `ok` は測定 harness 完走だけを意味し、capability の合否は
次の case result である。

| case | result | 実測確定 |
| --- | --- | --- |
| `permission_denial_audio` | Pass | microphone consent Deny + service restart 後の capture open が exact `Backend("permission_denied")` に分類され、Allow restore 後に endpoint 再列挙と clean open/start/stop（callbacks=39）まで回復した。 |
| `midi_device_loss_observation` | Pass | `Ruviform MIDI Loopback` は removal 前に存在し、removal 中の WinMM enumeration から消失した（before=True/exit0, during=False/exit0）。after=False も同時に記録し、reappearance を Pass にしていない。 |
| `device_loss_audio` | Unsupported | removal stimulus は stream 生存中に適用されたが、`StreamInvalidated`/control-path `DeviceNotFound`/`ClockDiscontinuity` は 0。recreate 後の endpoint surface はこの run では bounded window に復帰しなかった。typed invalidation は全 attempt で未観測だが、recovery は attempt1 で成功・attempt3-5 で失敗しており再現性がない。したがって typed audio device-loss invalidation + 保証された recovery を OS 制御だけで観測する capability は成立しない。 |
| `xrun_probe` | Unsupported | active stream callbacks=423 に bounded user-mode starvation を与えても xrun/discontinuity/invalidated/error はすべて 0。現行 adapter 契約では typed `DeviceError::Xrun` を OS/user-mode 制御だけでは観測できない。 |

device-loss の解釈は、「fault stimulus が効かなかった」ではなく、「PnP removal
自体は MIDI enumeration disappearance で独立確認できたが、audio adapter が
要求する typed invalidation/recovery oracle へ写らなかった」である。open
handle の lifetime、driver の PnP removal 処理、WASAPI error surface のどこに
恒久修正を置くべきか、また stop/start control probing が removal pending 状態の recovery へ影響するかは本 slice の lease 外なので断定しない。AC5 の代替
evidence は controller action log、before/during/after WinMM probe、phase file、
reopen observation、case JSON であり、error が出なかった事実を Pass に読み替えない。

xrun の要否判断は「必要」に確定する。必要なのは driver 制御面だけではなく、
drop/gap/underrun を adapter がどの error kind・timestamp/frame evidence として
露出するかの観測契約も含む。実装は `scarecrow173/ruviform#1009`（仮想 endpoint
の決定的 xrun 注入と adapter 観測契約）へ分離した。OS-L1 deterministic backend
の `StreamFault::Error(DeviceError::Xrun)` は API/backend 契約の evidence として
再利用できるが、EXT-L3 xrun Pass の代用にはしない。`ClockDiscontinuity` は現行
native input の discontinuity 表現であり、typed Xrun と等価とはみなさない。

## 13. 追記（2026-11-25）— Issue #253 MIDI Host/Plugin e2e stage と provenance fail-closed

Issue #253 AC1/AC6 vertical slice では、`os-endpoint-wiring` 後・`fault-cases` 前に
`os-endpoint-midi-e2e` stage を追加する。 canonical lane は
`mise run test-os-endpoint-midi-e2e` であり、workflow は verdict と RunMetadata を
artifact へ表面化させるため fixture build 後に `mise exec -- cargo test -p
ruviform-plugin-test-vst3 --features l1-runner --test os_endpoint_midi_e2e --
--ignored --test-threads=1 --nocapture` を補助証拠として実行する。

この stage の `virtual-os-endpoint` 解釈は name match ではなく workflow provenance を根拠にする。`midi-endpoint-probe` の loopback-candidate/provisioning 行と
`midi-loopback` の candidate/round-trip Pass 行を stage log へ転記し、両者が同一
attempt で成立した場合だけ `RUVIFORM_VIRTUAL_ENDPOINT_PROVENANCE` を
`rvfmidi-testsigned-driver-probe` にする。RunMetadata JSON は
`os-endpoint-midi-e2e-run-metadata-<os>/run-metadata-rf253-midi-virtual-endpoint-host-plugin-e2e.json`
として artifact に保存し、repo/SHA・`level=ext-l3`・
`environment_kind=virtual-os-endpoint`・result を検査する。

fail-closed 境界は wiring stage より強い。honest `not-run` は absence-is-data の
stage `ok` だが、`result=pass` に `rvfmidi-testsigned-driver-probe` provenance 行が
ない場合、または workflow 側 provenance が未成立のまま pass だった場合は証拠価値が
ないため stage `error` とする。Fail・crash・verdict/RunMetadata 未到達・artifact copy
不能も `error` であり、run conclusion や stage `ok` だけを AC1/AC6 Pass と読まない。
