---
id: "SPEC-034T9TZQ7ztGDAPvFaz9IT"
type: "spec"
status: "proposed"
title: "Issue #390 Driver Contract Lab — UI-DVR fixture registry・境界 Guard・GPUI なし Fake・Native Component Lab harness"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "docs/discovery/issue-390-driver-contract-lab-inventory.md"
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-ui-test/fixtures/driver-lab"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_boundary.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/bin/gui_boundary_guard.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_boundary.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_contract.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "target/driver-lab/evidence.jsonl"
  implements: []
  implemented-by: []
  depends-on:
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/257"
    - "docs/designs/issue-182-dock-tree.md"
    - "docs/designs/issue-312-test-gui-host-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/314"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-390-driver-contract-lab-inventory.md"
  derived-by:
    - "docs/designs/issue-390-driver-contract-lab.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-390-driver-contract-lab.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/262"
  defers: []
  deferred-by: []
---

# Issue #390 Driver Contract Lab — UI-DVR fixture registry・境界 Guard・GPUI なし Fake・Native Component Lab harness

## What Should Be Built

Issue #390（RF-GPUI-07）の 2026-11-07 横断レビュー確定仕様 v2 に
従い、Driver-neutral な共通 semantic truth としての
**(1) UI-DVR case manifest + fixture registry**、
**(2) 依存境界 Guard（cargo metadata 依存グラフ検査 + syn による
public signature 漏出検査 + 明示 allowlist）**、
**(3) GPUI なし Fake Driver 側の実行可能 fixture**、
**(4) Native Component Lab の story registry + harness の骨格** を
`ruviform-ui-test` とその `fixtures/driver-lab/` 以下に実装し、
`consumer-verify` の `GUI-CONTRACT`/`GUI-SEMANTIC` lane が接続できる
正規 mise task 入口と判定語彙（Pass/Fail/Unsupported/NotRun/Blocked）
を提供する。

## Why Now

- `consumer-verify.yml` は `GUI-CONTRACT`/`GUI-SEMANTIC` 行を
  `not_run`（Owner: RF #390）で固定済み。接続可能な ruviform 側の
  正規 task が存在しないことが現在の gap。
- GPUI 本体が依存木に入る前に「common crate は GUI toolkit に
  依存しない」「公開 signature に toolkit 型を露出しない」
  「例外は allowlist のみ」を機械検査として固定する必要がある。
  #386/#387 マージ後に境界を後付けすると漏出検出が遅れる。
- RF386（未 merge）が `WindowDriver`/`FakeDriver`/`ruviform-ui-gpui`
  を実装済みのため、#390 は型・Fake の重複新設をせず、契約到着時に
  同じ case registry へ bind する構造を先に確定する。

## Users and Value

- **#294 cross-repo lane**: GUI-CONTRACT/GUI-SEMANTIC の
  canonical command・verdict 語彙・証拠形式を得る。
- **#385〜#389 Owner**: 公開契約の semantic oracle と接続点を
  （重複 Harness なしに）得る。
- **両 Consumer**（Synvelis/AudioLens）: composition root 以外に
  GPUI/Toolkit が漏出しないことを tuple 証拠で確認できる。
- **将来の Driver 実装者**: 同一 case manifest を満たす
  semantic compatibility 検査の入口を得る。

## Scope

### In Scope

1. **`fixtures/driver-lab/cases.json`** — UI-DVR-01〜08 の
   case manifest。各 case は id・title・oracle 意味論・layer 別
   binding（`boundary`/`fake`/`gpui`/`real-os`/`headless-ts`）・
   status（`pass`/`fail`/`not_run`/`unsupported`/`blocked`）・
   Owner Issue・再実行条件・証拠先を持つ。Case ID は registry で
   固定し、consumer 固有名詞を含まない。
2. **`fixtures/driver-lab/stories.json`** — Native Component Lab
   story registry（schema + seed）。Synvelis Fader+Tabs/Dock・
   AudioLens Waveform+Seek を 2 Product Theme × 2 scale factor で
   宣言し、driver 別可用性を正直に記録。
3. **`fixtures/driver-lab/boundary.json`** — 境界 ruleset。
   `forbiddenDepRoots`（gpui 系・wry/tao・他 GUI toolkit/backend）、
   `forbiddenPathRoots`（signature 走査対象 path root）、
   `allowlist`（driver adapter/composition root/extension の
   grant: package・allowedDeps・viaFeatures・reason・required）。
4. **`driver_boundary` モジュール + `gui-boundary-guard` bin**
   （`ruviform-ui-test`）— ruleset を読み、(a) cargo metadata の
   resolve + manifest `optional`/`features` から依存グラフを検査し、
   (b) `syn` で各 protected crate の lib target 公開 module tree を
   走査し pub signature 内の forbidden path root を検出し、
   (c) allowlist grant の逸脱（非 optional 化・grant 外 feature・
   未宣言 adapter）を検出する。verdict JSON + human report を出す。
5. **境界 Guard の正負 fixture** — `fixtures/driver-lab/boundary/`
   下に canned `cargo metadata` 相当 JSON と source tree を置き、
   positive（許容形）/negative（直依存・推移依存・feature 漏出・
   signature 漏出）の双方で検出能力を pin する。
6. **`tests/driver_contract.rs`** — cases.json の manifest 整合性
   検査 + main で実行可能な UI-DVR fixture（05 Dock 2-phase・
   07 capability honesty・08 window/job/subscription teardown・
   01 driverless build の guard 接続）を実装。
   `tests/driver_boundary.rs` — 正負 fixture による guard 検証。
7. **mise task**: `check-ui-driver-boundary`（GUI-CONTRACT 入口）・
   `test-ui-driver-contract`（GUI-SEMANTIC/fake 入口）。
8. **docs**: development-environment.md タスク表追記、
   `crates/ruviform-ui-test/README.md` 責務追記、
   `evidence/rf390-driver-lab/report.md` 実測記録。

### Out of Scope

- GPUI 実描画・実 OS window・実 plugin Native Dock・Visual Golden
  撮像/比較（#313/#314/#388/#389/Synvelis #176 の gate）。
- `WindowDriver`/`FakeDriver`/`ViewSpec`/`ResolvedTheme`/
  `NativeSurfaceSlot`/`PluginEditorSession` の型定義そのもの
  （#385〜#389 Owner。本 Issue は到着した公開契約へ bind する）。
- Consumer repo 側の import 検査実装（#294 の Consumer lane が
  各 repo の検査 task を呼ぶ。本 Issue は語彙と接続点を定義）。
- rustdoc JSON/public-api による完全名解決検査（将来拡張候補）。
- 新規 CI workflow・PR required gate・Daily 運用の変更（#294 が
  workflow Owner。本 Issue は task と語彙のみ提供）。
- 旧 WebView/SolidJS/TS harness の削除・置換（履歴保持）。
- WASM。

## Requirements

- **R1（semantic truth の一元化）**: cases.json が UI-DVR case の
  oracle・binding・status の唯一の正本。同一 case を Fake/GPUI/
  real-os で再実行する場合も semantic 期待は 1 箇所に記述し、
  driver 差は binding 側の status/証拠で表す。
- **R2（既定 deny + 明示 allowlist）**: workspace 全 member を
  既定 protected とし、GUI toolkit/backend 依存を持てるのは
  allowlist 記載 package のみ・grant 範囲のみ。allowlist entry は
  role（`driver-adapter`/`composition-root`/`driver-extension`）・
  allowedDeps・viaFeatures・reason を必須とする。
- **R3（2 系統の検査の併用）**: 依存グラフ検査（直依存＋非
  optional 推移到達 + manifest feature 解析）と public signature
  漏出検査（`syn` による pub item signature の path root 走査）を
  両方実装する。文字列 grep 単独による保証は行わない。
- **R4（検査能力の正直な記述）**: Guard の検出能力と限界
  （glob re-export・`#[path]`・macro 内・bare OS handle 型名・
  pub(crate) 境界の扱い）を ruleset 同梱コメントと test-spec に
  明記し、positive/negative fixture で実際の検出範囲を pin する。
- **R5（未実装契約を stub で完成扱いしない）**: 契約が main に
  存在しない case/binding は `not_run`/`blocked` + Owner Issue +
  再実行条件を記録する。仮互換型・shadow contract を新設しない。
- **R6（判定語彙の統一）**: case/binding/lane の結果語彙は
  `pass`/`fail`/`not_run`/`unsupported`/`blocked`。
  `not_run`（capability はあるが未接続）と `unsupported`
  （capability 不足・実機/OS 依存）と `blocked`（上流契約待ち）
  を区別し、skip を pass にしない。
- **R7（既存資産の再利用）**: FakeHost/HeadlessIo/TestDriver、
  `tests/dock.rs`/`tests/runtime.rs`/`tests/headless_contract.rs`、
  `ui/guitest` 系 lane、mise task パターンを再利用し、重複
  Runner/Harness/EvidenceDB を新設しない。
- **R8（composition root 例外の形）**: Consumer 側 composition
  root が driver adapter を直接参照するのは許容（allowlist の
  `composition-root` role）。ruviform workspace 内では adapter への
  依存は adapter 自身の example/bin のみが持てる形を検査する。
  依存の kind（normal/build/dev）と activation 状態（optional
  含む）は例外事由にならない — dev-dep や未 activation の
  optional 宣言による adapter 参照も検査対象とする。
- **R9（RT 契約）**: lab/guard は全て非 RT。RT/Audio thread
  semantics を fixture に持ち込まない。

## Acceptance Criteria

- [ ] `mise run check-ui-driver-boundary` が現行 workspace で
  `verdict=pass` を返し、`ruviform-ui-webview` 以外に GUI toolkit
  依存が存在しないことを cargo metadata から検証する。
- [ ] negative fixture（直依存・推移依存・grant 外 feature・
  signature 漏出）に対して guard が `fail` と正確な violation
  箇所を返し、positive fixture（optional + grant feature の
  adapter、private mod 内 pub item、cfg-gated pub mod）が `pass`
  を返すことを `tests/driver_boundary.rs` で検証する。
- [ ] `mise run test-ui-driver-contract` が UI-DVR-01/05/07/08 の
  実行可能 binding を実際に実行し pass し、cases.json の各 case が
  schema・語彙・binding 参照整合性を満たすことを検証する。
- [ ] UI-DVR-02/04/06 と全 GPUI/real-os binding が `not_run`/
  `blocked`/`unsupported` のいずれか + Owner + 再実行条件を持ち、
  pass へ繰り上がっていないことを manifest 整合性 test が確認。
- [ ] `check-ui-driver-boundary`/`test-ui-driver-contract` が
  docs/development-environment.md のタスク表に登録され、
  `mise run verify-ci`（fmt-check/lint-ci/test-ci）が pass する。
- [ ] `evidence/rf390-driver-lab/report.md` に実行した検証・
  verdict・未実行 lane・3 repo SHA・限界の記述がある。

## Deferred Design Concerns

- rustdoc JSON による完全な public API 名解決 | 現状は syn 走査 +
  依存グラフで contract v2 を満たす | 外部型経由の再 export 漏出が
  実害となった時 | 検査 gap の放置
- GPUI Driver 側 binding の実装 | `ruviform-ui-gpui` が main に
  到着後に cases.json の `gpui` binding を有効化 | #386 merge |
  実描画 semantic parity の未検証が続く
- Consumer 側 composition root allowlist schema の確定 | 各
  Consumer repo の ruleset 拡張時に共同決定 | Consumer GUI slice
  実装時 | 製品側例外の不統一

## Sources and Relations

- Issue #390 v2 spec（本 spec の normative 入力）・追加合意 2 節
  （依存境界・等価性・漏出検査・実機独立）
- Issue #384（Driver-neutral 境界・allowlist 原則・static check）
- Issue #257（ViewSpec/ComponentKey/SurfaceSlotId・共通契約正本）
- Issue #294（lane 語彙・tuple 証拠・GUI-CONTRACT not_run 行）
- `docs/discovery/issue-390-driver-contract-lab-inventory.md`
  （main 上の実行可能 subset 棚卸し）
- RF386 branch `d907bfc`（bind 予定の契約実装・非 merge 参照）
