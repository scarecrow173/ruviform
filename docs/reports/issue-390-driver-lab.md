---
id: "IMPL-RF390-REPORT"
type: "impl-record"
status: "completed"
title: "Issue #390 Driver Boundary/Lab 実装記録"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  implements:
    - "docs/specs/issue-390-driver-contract-lab.md"
  derives-from:
    - "docs/designs/issue-390-driver-contract-lab.md"
    - "docs/plans/issue-390-driver-contract-lab.md"
  verified-by:
    - "docs/test-specs/issue-390-driver-boundary-lab.md"
---

# Issue #390 Driver Boundary/Lab 実装記録

## 実施内容

`ruviform-ui-test` に Driver Boundary Guard（`gui-boundary-guard`、
ruleset=`fixtures/driver-lab/boundary.json`）と Driver Lab
registry（cases/stories manifest + 検証 + evidence emission）を実装。
UI-DVR-01〜08 を `requiredCases` で pin し、UI-DVR-01/05/07/08 の
fake layer を実行 fixture 化。残 case/layer は公開契約未 merge
のため owner+rerun 条件付き `blocked`/`not_run`（stub 代替なし）。

## 検査設計の要点

- dep-graph: workspace member 起点で direct/feature/
  dep.features/transitive/conditional/dep-activation/allowlist
  整合を検査。cargo metadata の resolve graph は未 activation
  optional dep を省略するため、(a) conditional 到達は manifest
  宣言（`dep.path` 再帰走査）で補完、(b) dep-activation の
  adapter 参照解決は workspace member name 一致で補完する。
  optional/dev/rename 宣言による検査迂回は不可。
- signature: syn + proc-macro2(span-locations) の exposure-aware
  2-pass 走査。crate 外露出 item のみ検査（pub use chain・
  alias `use wry::X as W`・公開 surface を持つ impl の header/
  assoc item・cfg 保守走査）。allowlisted adapter の pub API
  漏出も検出対象（private 利用・private type 上の impl は
  public API でないため検出しない）。
- evidence: `{manifest,subject,layer,expected,observed,mechanism,
  detail}` JSONL（`RUVIFORM_DRIVER_LAB_EVIDENCE` 指定時のみ。
  `expected`=manifest 宣言、`observed`=実実行結果）。

## 独立 review 記録

- Reviewer A（設計/整合観点）・Reviewer B（機構観点）を独立
  dispatch。確認済み P0/P1:
  - RB-01 P0: dependency 宣言の `features`/`uses_default_features`
    による adapter viaFeatures 起動が未検査 → `check_dep_activation`
    追加（dep 宣言 feature 表 + adapter feature 連鎖 + 非 grant
    consumer の `adapter-consumer` 検査）。
  - RA critical: allowlisted adapter 自身の optional toolkit 辺が
    `forbidden-dep-conditional` で自爆 → 起点 grant exempt +
    adapter outgoing 辺の走査停止。
  - conditional 走査が resolve 辺のみで未 activation optional
    dep を見失う（fixture で検証）→ manifest-level 再帰走査へ
    再設計（検出確認済み）。
  - signature scanner の私的利用誤検出 → exposure-aware 2-pass
    へ再設計（adapter pub API は対象・private mod 内 item は
    re-export chain 経由のみ）。
  - alias 迂回・impl header・dep-activation 制御流・manifest
    binding 語彙（manual+pass の note 必須・requiredCases・
    layer 語彙・story themes）を対応済み。
- 修正後の targeted re-verify: 全 10 boundary fixture テスト +
  5 contract テスト + live workspace verdict pass。

## 独立 Validator 記録（第 2 ラウンド、coordinator 指示による）

実装者の自己確定を避けるため、内部 subagent（read-only）を
独立 Validator として dispatch。実コード・fixture・設計 doc を
照合した結果:

- 第 1 ラウンドの全修正（RB-01/RA critical/conditional 走査/
  exposure model/impl gating）は VALIDATED（file:line 証拠付き）。
- Validator が新規発見した残存ホールと disposition:

| ID | 内容 | disposition |
|---|---|---|
| H1（中） | 未 activation optional dep による adapter 依存が全検査をすり抜け（`optional`+feature 起動の迂回） | **修正済み**: dep-activation で manifest 宣言 dep を member name 解決し kind/activation 不問化 + consumer feature 表 `depkey/f`・`depkey?/f` 走査追加。fixture `adapter-feature/common2` で pin |
| H2（中低） | dev-dep による adapter 依存を exempt していた | **修正済み**: kind 不問で `adapter-consumer`。spec R8 に kind/activation は例外事由でない旨明記 |
| H3（低中） | feature 値 `dep:alias` が実 package 名へ解決されず rename 迂回可 | **修正済み**: manifest dep key → 実名解決後に forbidden 判定。fixture `feature-leak` を rename dep（`dep:mygpui`）へ変更して pin |
| H4（低） | `target.'cfg(..)'` 非 optional 辺の transitive 経路未捕捉 | 限界として design 限界 10 に記録（direct cfg-dep 宣言は manifest 検査が捕捉） |
| H5（低） | manifest 走査の解決失敗を無言 skip（fail-open） | **修正済み**: `conditional-manifest-error` finding に変更 + `--offline` 伝播 + `manifest_path` 一致で package 厳密選択 |
| H6（低） | dep 宣言先解決が name 一致ベース（同名 multi-version で誤同定の可能性） | 限界として design 限界 11 に記録 |
| H7（低） | `crate/feature` feature 値は adapter 宛のみ検査 | 限界として design 限界 9 に記録（非 adapter 宛は resolve/transitive 側で担保） |

- doc 不整合 3 件（allowedDeps semantics・report schema・R8 文面）
  も修正済み。
- Judge disposition: 本 report は root 統合 Judge への入力と
  する — Validator 検証結果と再検証根拠（下記検証結果節）を
  添付。実装者は自己承認していない。

## RF386 candidate 接続検証（d907bfc）

RF386 は「未着」ではなく **未 merge・実装済み**（candidate
`d907bfcc8a8f4be2ae90cdb6f17bef3cf45d34a0` が fetch 可能）。
`ruviform-ui/src/driver.rs`（driver-neutral window/host 契約 368 行）、
`ruviform-ui-test/src/fake_driver.rs` + `tests/fake_driver.rs`
（Fake driver 14 テスト）、`ruviform-ui-gpui` adapter（optional
`gpui = "0.2.2"`、`gpui = ["dep:gpui"]` 形）を追加。

- **二重実装の有無**: なし。RF386 は `WindowDriver` 契約・
  FakeDriver 実装・GPUI adapter を所有し、RF390 は
  boundary guard + case/story registry + UI-DVR 実行 fixture を
  所有。module 名・責務とも重複なし。RF386 の 14 fixture は
  新 `WindowDriver` 契約に対し、UI-DVR-01/05/07/08 は既存
  `HeadlessIo`/dock/runtime API に対しており、merge 後に
  cases.json の fake/gpui binding から接続可能（複製ではなく
  binding 追加）。
- **guard 接続検証**: detached worktree で candidate に対し
  同一 ruleset で guard を実実行（`--manifest-path` +
  `--ruleset` + `--offline`）:
  - 修正前（impl header 無条件走査）: `signature` で
    `impl gpui::Render for RvfRoot`（private type、lib.rs:964）
    を誤検出 → scanner を `impl_is_public` gate へ修正。
  - 修正後: **dep-graph/signature/allowlist 全 pass・verdict
    pass**。candidate の optional `gpui` dep + `dep:gpui`
    feature 形は allowlist 整合、public signature に backend 型
    の漏出なし（private `RvfRoot` impl は正しく非検出）。
  - この結果は `rvf.driver-boundary/v1` 意味での **依存・公開
    signature 境界適合** であり、GPUI の機能的受入・実機証拠
    ではない（UI-DVR-04/06 等の gpui layer は依然 not_run）。
- **正負 fixture の両方向証拠**: private type への
  `impl gpui::Render` は非検出（positive adapter + signature-leak
  の `impl Priv`）、pub type への `impl gpui::Render` は検出
  （signature-leak の `impl Pub`、detail=`gpui::Render`）を
  test が pin している。

## consumer-verify GUI-CONTRACT 接続 guidance

RF294 workflow の GUI-CONTRACT 行は **2 つの独立した実判定**を
持つべきで、guard pass だけで未実行ケースを pass にしない:

| lane | canonical command | 実判定の意味 |
|---|---|---|
| boundary | `mise run check-ui-driver-boundary` | ruleset 適合（dep-graph+signature+allowlist）。exit 0=適合、1=violation、2=tool error |
| semantic fixture | `mise run test-ui-driver-contract` | UI-DVR-01/05/07/08 fake layer の実行判定（5 tests）。UI-DVR-02/03/04/06 は manifest 上 `blocked`/`not_run` で実行されない |

- guard verdict=pass は「依存・公開 signature 境界が clean」
  のみを意味し、UI-DVR case の semantic pass ではない。
- semantic fixture の pass も fake layer の意味証拠のみ —
  GPUI/実 OS/Native Dock の受入証拠にならない。
- 上記 2 行を GUI-CONTRACT の別 command として接続し、
  blocked/not_run case は判定語彙を維持すること。

## 検証結果（実 exit code 記録、実行 SHA を明記）

| head | 検証 | 結果 |
|---|---|---|
| `72818c5` | `mise run check`（workspace） | exit 0 |
| `72818c5` | `mise run lint`（workspace） | exit 0 |
| `72818c5` | `mise run test`（workspace） | exit 0 — 158 suite 全 pass・0 failed（`ruviform-plugin-host` unit 222 + l1_proxy/supervisor/video/plugin_graph e2e lane 全 green） |
| `e6df814` | `mise run check`（workspace） | exit 0 |
| `e6df814` | `mise run lint`（workspace） | exit 0 |
| `e6df814` | `mise run test`（workspace） | **exit 101** — 1 flaky failure（下記分類節）。86 suite ok・1 failed |
| `e6df814` | `mise run check-ui-driver-boundary` | exit 0（verdict pass、3 check 全 pass） |
| `e6df814` | `mise run test-ui-driver-contract` | exit 0（5 tests pass） |
| `e6df814` | `cargo clippy -p ruviform-ui-test --all-targets -- -D warnings` | clean |
| `e6df814` | `mise run fmt-check` | pass |
| candidate `d907bfc` | guard `--manifest-path` 実実行 | verdict pass（3 check 全 pass、修正後 binary） |

### plugin-hostTimeout の具体分類

`e6df814` での workspace `mise run test` 実実行で
**観測・具体分類済み**（72818c5 では未発生）:

- 失敗名: `composite_e2e::command_timeout_converges_failed_with_timeout_cause`
  （`crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs:2193`、
  テスト binary は 183+1 tests）。
- 失敗形態: `HostCommand::Instantiate` の `.unwrap()` が
  `SpawnFailed { cause: Timeout, diagnostics_id: 1 }` — L1
  child process の **spawn/handshake が spawn timeout を超過**
  した経路。テスト本体が検証する `command_timeout=500ms` の
  command hang 経路（spawn 後の wedge 収束）ではなく、
  **spawn 自体が並行負荷下で timeout に乗った**別経路。
- 再実行判定: 同一 test を単独実行 → **0.61s で pass**。
  `l1_proxy_e2e` binary 全体（184 tests）の再実行も **11.37s
  で全 pass**。deterministic な失敗ではなく workspace 全体の
  並行負荷に依存する timing-dependent flake。
- 因果分類: RF390 の変更範囲（`ruviform-ui-test` + docs）と
  plugin-host に共有コード・依存変更はなく **回帰ではない**。
  coordinator 通知の「main にも存在」と整合する既存 flaky
  spawn 経路と分類（同一 failure と推測せず、観測形態を
  spawn-path timeout として記録）。owner lane は RF390 外の
  ため本 PR では修正しない。

## 残 gate

- UI-DVR-02/03/04/06: 契約未 merge（#385/#387/#388）で `blocked`。
- GPUI/実 OS/`headless-ts` layer: RF386 merge・#313/#294 接続で
  `not_run`。candidate 境界適合は上記の通り機能受入ではない。
- consumer-verify GUI-CONTRACT lane への接続は #294 owner 側の
  行更新（上記 guidance 表の 2 command）。
- `required:false` GPUI grant は RF386 merge で活性化（candidate
  との整合は接続検証済み）。
- evidence 行と manifest binding の機械照合は後続改善。
- Native Plugin Dock 受入は #388/#389 + Consumer 実機証拠待ち
  （本 PR は達成を主張しない）。
