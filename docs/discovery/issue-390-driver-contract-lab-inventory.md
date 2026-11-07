---
id: "DISC-034T9T71FKLbmcJ4jmuUcD"
type: "discovery"
status: "final"
title: "Issue #390 Driver Contract Lab 既存資産棚卸し（Fake/Boundary/Dock/Headless 現行 coverage と gap）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/257"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/314"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/issue-390-driver-contract-lab.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #390 Driver Contract Lab 既存資産棚卸し（Fake/Boundary/Dock/Headless 現行 coverage と gap）

## Summary

UI-DVR-01〜08 の共通 semantic fixture・GPUI なし Fake Driver・
依存/Public API 境界 Guard・Native Component Lab harness を
実装する前に、main（`355b7a1`）時点で再実行可能な既存資産と、
未マージ契約（#385〜#389）待ちの領域を棚卸しした。
結論: 境界 Guard と case manifest・registry は main 上で
完結して実装できる。UI-DVR-05/07/08 は既存 test suite へ
bind 可能。UI-DVR-02/04/06 と全 GPUI lane は Owner Issue の
公開契約が main に届くまで `not_run`/`blocked` として記録し、
stub 互換型で完成扱いしない。

## Exploration Goal

Issue #390 の 2026-11-07 横断レビュー確定仕様 v2 が要求する
「Case manifest + Fake/GPUI 同一意味 fixture + 機械的依存境界
検査 + Native Component Lab」のうち、**(a) いま main で正攻法に
実装できるもの** と **(b) 他 Owner Issue の契約到着を待つべきもの**
を切り分け、二重 Harness・仮 stub・grep だけの無根拠保証を
避ける実装境界を確定する。

## Key Issues and Assumptions

- `consumer-verify.yml` は既に `GUI-CONTRACT` / `GUI-SEMANTIC` を
  `not_run` 固定行で宣言済み（Owner: RF #390 + Consumer Issue）。
  #390 の成果物は「ruviform 側の正規 mise task + 判定語彙」で、
  Consumer 側配線は #294 経由の後続接続（`extra_tasks`/専用 lane）。
- RF386（PR394・head `d907bfc`・Draft/Refs）が `ruviform-ui` に
  `WindowDriver` 中立契約、`ruviform-ui-test` に `FakeDriver` +
  14 semantic fixtures、`ruviform-ui-gpui` に隔離 GPUI adapter を
  実装済みだが **main 未マージ**。本 Issue は同型の Fake/型を
  新設せず、公開契約到着時に同じ cases.json へ bind する構造を
  先に作る。
- main 上の driver-neutral 契約面: `ruviform-ui` の
  `WindowHandle`/`HostEvent`/`WindowConfig`/`DockWorkspace`
  （Issue #181/#182 系・WebView 語彙を含む旧契約）と
  `ruviform-ui-test` の `FakeHost`/`HeadlessIo`/`TestDriver`。
  `WindowSpec`/`WindowDriver`/`WindowCapabilities` 等の新型は
  RF386 マージ後にのみ存在する。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. 自分の branch に RF386 head を merge し、Fake/GPUI fixture を即接続 | UI-DVR の大半が即実行可能 | 未確定 Draft の全差分を #390 PR が内包・review 不可分離・Owner の未完了変更を取り込む | no |
| B. main のみで Guard/manifest/registry/executable subset を実装し契約待ちを not_run 記録 | 差分が #390 scope に閉じる・guard が GPUI 到着前の境界を先に固定 | UI-DVR 一部は契約 merge まで未実行 | **yes** |
| C. Guard を cargo tree/文字列 grep のみで実装 | 実装が薄い | 「公開型への漏出」検査が根拠不足・issue が明示的に禁止 | no |
| D. Guard を rustdoc JSON で実装 | 公開 API を完全解決できる | nightly/rustdoc-json 依存・toolchain 増・CI 重い | no（将来拡張候補として記録） |

## Tentative Conclusions and Hypotheses

- **(confirmed)** 依存グラフ検査は `cargo metadata`（resolve +
  manifest `optional`/`features` 情報）で「protected crate が
  非 optional 辺のみで forbidden root に到達しない」「allowlist
  adapter が forbidden dep を宣言する場合は optional + grant
  feature 限定」を検査できる。
- **(confirmed)** Public signature 漏出検査は `syn` で pub item
  （fn/struct pub field/enum variant field/type/use/trait/impl
  pub fn/generics bound）を `pub` module tree のみ走査すれば、
  文字列 grep では不可能な「pub シグネチャ限定」の検査になる。
  `pub(crate)`/private mod 内は公開面に出ないため除外する。
- **(confirmed)** UI-DVR-05（Dock 2-phase move/rollback）は
  `crates/ruviform-ui/tests/dock.rs` の 7 test（prepare/commit・
  dest 破棄 rollback・detach/reattach stable ViewInstanceId・
  close disposition・layout snapshot）が main で実行可能。
  UI-DVR-08 相当の subscription/job teardown は
  `tests/runtime.rs`（`close_requested_cancels_jobs_and_emits_lifecycle`・
  `job_cancel_reaches_terminal_state`）+ `FakeHost`/`HeadlessIo`
  が一部をカバー。
- **(confirmed)** UI-DVR-07（capability negotiation）は
  `ruviform-ui-test` の `Capabilities`/`DeniedCapabilities`
  （`rvf.uitest.v1` handshake）で fake 側の honest capability を
  実行可能。`WindowCapabilities`/`Unsupported` 語彙の driver 版は
  #386/#388 到着待ち。
- **(blocked)** UI-DVR-02（ViewSpec mount/reconcile/stable keys）、
  UI-DVR-04（ResolvedTheme mode/override）、UI-DVR-06
  （NativeSurfaceSlot）は対象契約が main に存在しない。
  manifest では `not_run` + Owner Issue + 再実行条件を記録する。
- **(hypothesis)** Component Lab の story registry（Synvelis
  Fader+Tabs/Dock・AudioLens Waveform+Seek・2 theme × 2 scale）
  は schema +  seeded entry として定義できるが、実行 binding は
  #385/#387 契約待ちで全件 `not_run`/`blocked` になる見込み。

## Open Questions

- RF385 `ResolvedTheme` の Rust 型 candidate SHA は Owner 未提供
  （coordinator 経由の型要求フロー）。本 Issue は ResolvedTheme を
  参照する fixture を `not_run` 接続点として定義し、仮互換型を
  作らない。
- GPUI version pin（PR394 は `gpui = "0.2.2"` optional）の
  license/MSRV/transitive 審査は #386/#384 の記録対象であり、
  本 Issue の allowlist は「`ruviform-ui-gpui` が `gpui` feature
  で optional 隔離する」形だけを検査する。

## Promotion Candidates

- [x] spec-doc needed: Issue #390 の DriverContractFixture /
  boundary guard / component lab registry の実装 spec。
- [ ] adr-doc needed: なし（#384 が後継 ADR Owner。本 Issue は
  検査・fixture の実装 Issue であり技術採否を記録しない）。

## Sources and Relations

- Issue #390（本 Issue の v2 spec・DoD・追加合意 3 節）
- Issue #384（依存方向・allowlist 原則・Static architecture check 要件）
- Issue #294（GUI-CONTRACT/GUI-SEMANTIC lane 語彙・tuple 証拠）
- Issue #257（共通 UI 設計正本・ViewSpec/ComponentKey/SurfaceSlotId）
- Issue #385/#386/#387/#388/#389（未マージ契約の Owner）
- RF386 branch `scarecrow173/rf386-gui-v2` head `d907bfc` の
  実装（重複を避けるための参照、merge しない）

## Evidence

- `git diff --stat origin/main...d907bfc`（RF386 差分: driver.rs
  368 行・fake_driver.rs 414+449 行・ruviform-ui-gpui 976 行・
  `example-ui-gpui-minimal` mise task）
- `.github/workflows/consumer-verify.yml` L497-502・L849-855:
  `GUI-CONTRACT`/`GUI-SEMANTIC`/`real-window`/`PLUGIN-NATIVE-DOCK`/
  `macOS` の固定行（not_run/unsupported と Owner・再実行条件）
- `crates/ruviform-ui/tests/dock.rs`（7 test 関数名）
- `crates/ruviform-ui/tests/runtime.rs`（job/lifecycle teardown 系）
- `crates/ruviform-ui-test/src/{fake,headless,proto}.rs`
  （`FakeHost`/`Capabilities`/`DeniedCapabilities`）
- 現行 workspace で GUI toolkit 依存を持つのは
  `ruviform-ui-webview` のみ（`wry`/`tao`/`http`/`dpi`/`rust-embed`
  が `webview` feature 下の optional dep）。
