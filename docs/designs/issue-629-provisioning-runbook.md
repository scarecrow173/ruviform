---
id: "DESIGN-E2nW52u7DBe6EqaWvY1PRi"
type: "design"
status: "approved"
title: "Issue #629 provisioning runbook and MIDI virtual lane separation design"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/plans/issue-629-midi-loopback.md"
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
- AC3 typed 結線（`ruviform-audio-device` →
  `ruviform-audio-test` dev-dep、逆方向も未裁定）—
  Coordinator 承認待ち。
- 採用 endpoint の確定と再現 Case 実装 — provisioning
  実測後。runbook は候補別手順のみで採用を決めない。
- consumer 側の独立 Oracle での実受入判定（Synvelis
  #115/#117/#126・AudioLens #127）。
- `EnvironmentKind` の `real-abi-test-plugin`↔`test-plugin`
  serde 語彙 gap — #389 占有解除後または Coordinator 調整。
  status: rf#725 Phase A で consumer 影響評価と deserialize
  alias `test-plugin` 受容を適用済み — emit flip は第2 slice
  （`docs/designs/issue-725-serde-vocab-phase-a.md`）。
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

## 8. Capability Matrix（確定版 — provisioning 実測待ち項目を明記）

survey doc「Capability Matrix（暫定）」を引き継いだ確定版。
凡例: ✅=再現可能見込み / ⚠️=条件付き / ❓=未確認（probe
実測要）/ ❌=不可または非現実 / `NotRun`=本 slice 範囲外。
**❓/⚠️ は全て provisioning 実測待ち — 本版は公開情報 +
probe 設計ベースの確定版であり、実測で更新する。**

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

### 実測待ち項目（survey Open Questions #1-#7 の対応）

provisioning 実測で確定させる項目 — runbook §5 の lane
実行と probe 出力で収集する:

1. VB-CABLE endpoint の disable/enable・既定 device 切替で
   `IMMNotificationClient` が発火するか（hotplug 相当）。
2. loopMIDI port の出現/消失が `midiInGetNumDevs` に動的
   反映されるか（app 起動/終了で index が変わるか）。
3. Windows MIDI Services が対象 runner の OS build で
   利用可能か（25H2+ かつ feature rollout 状態）。
4. VB-CABLE/loopMIDI の license が専用 CI/test runner への
   install と自動 provisioning を許容するか（§2 で確認・
   記録）。
5. `IsFormatSupported` の `ppclosestmatch` 必須挙動は
   Windows build 依存か恒常か（他 build での再現性）。
6. capture 側の permission denial（microphone privacy）を
   仮想 endpoint で再現できるか。
7. `midiOutMessage` crash が GS Wavetable Synth 固有か
   winmm 全般か（他 driver での再現性）。

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
