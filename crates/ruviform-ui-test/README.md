# ruviform-ui-test

`ruviform-ui` Host に対するテスト専用 Lifecycle/Control ポート
(`rvf.uitest.v1`) と、それを検証する headless 基盤(Issue #312)。
実 Window E2E と CI の両方で使える「制御ポート + fake host +
driver + dummy consumer」の一式を提供する。**テスト/開発の明示
opt-in 専用**であり、本番クレートは本クレートに依存しない。

## 責務

- 制御ワイヤ `rvf.uitest.v1`(`proto` モジュール): `ruviform_ui::ipc`
  envelope(`Intent`/`Event`/`ErrorBody`) を再利用し、op は閉じた
  `test.*` 名前空間(`test.hello` / `test.capabilities` /
  `test.fixture.prepare` / `test.clock.advance` / `test.fault.set` /
  `test.fault.clear` / `test.window.request_close` /
  `test.observe.windows` / `test.observe.receipts` /
  `test.observe.screenshot` / `test.notices` / `test.shutdown`)。
  任意 eval・shell・file アクセスは表現不能。
- トランスポート(`transport` モジュール): u32le length + JSON の
  有界フレーム(`MAX_INTENT_BYTES` 超過は本体を drain して
  ストリーム同期を維持)。既定は stdio パイプ(OS 権限継承)。
  opt-in で loopback TCP — ephemeral port・run-bound secret・
  loopback peer 限定・単一接続・`RVF_TEST_DISABLE` で実行時無効化。
- エージェント(`agent` モジュール): app 内で動く制御面。
  `test.hello` の ready/failed handshake、session/run id、
  capability 強制、未認証・不正 op・disabled の型付きエラー、
  `Notify` イベントによる lifecycle 通知、driver EOF/書込み失敗時の
  graceful cleanup。
- `TestIo` ポート(`io` モジュール): エージェントと AUT(app under
  test)の橋 — emit/windows/faults/fixture/receipts/shutdown。
  `HeadlessIo`(`headless` モジュール)は `FakeHost` 上の実装で、
  実 window なしで契約全体を CI 検証できる。
- `FakeHost`(`fake` モジュール): `WindowHandle` の headless 実装。
  `ruviform-ui-webview::session` と同じ意味論 — 未 Ready eval の
  有界 backlog・破棄済み宛て drop・veto 可能な `CloseRequested`
  (既定は対象 window のみ approve)・終端 `Destroyed`・app 所有の
  `exit`。
- `FakeDriver`(`fake_driver` モジュール・Issue #386): driver-neutral
  `WindowDriver` の deterministic 実装。`FakeHost` と同一 semantic
  を中立契約で再現 — capability gating(`UiError::Unsupported`)・
  `Action` ingress・monitor snapshot 付き。semantic fixture は
  `tests/fake_driver.rs`。
- 観測(`journal` モジュール): host→app / app→host 双方の receipt を
  有界 ring に記録。payload は byte 長のみ(bulk 本文は複写しない)。
  `seq` cursor + `truncated` で pagination。`deliver_routed` は
  `UiRuntime` 向けの routed 配送ヘルパ。
- `TestDriver`(`driver` モジュール): テスト側 client。子プロセス
  spawn(stdio)/認証 TCP 接続、handshake、型付き op、typed
  `DriverError`、異常終了の reap。
- `dummy_app`(`src/bin/dummy_app.rs`): 受入用ダミー consumer。
  通常の `UiRuntime`/`WindowEvent` 配線で、fixture(複数
  window・IPC emit)・close policy(unanswered/approve/veto/defer)・
  fail-init・stdio/TCP/disabled の各モードを提供。stdout は制御
  channel 専用で診断は stderr。

## feature / 依存

feature なし。依存は `ruviform-ui`(公開 API のみ)と serde/serde_
json、Issue #390 boundary guard 用の cargo_metadata/syn/
proc-macro2、Issue #314 golden 用の png/base64。`wry`/`tao`/GUI/RT
依存は持ち込まない。
`ruviform-ui-webview`・RT クレート・他 consumer は本クレートに
依存しない — production release で制御ポートは到達不能。

## セキュリティ / fail-closed

- 制御は env opt-in(`RVF_TEST_CONTROL=stdio|tcp`)なしには serve
  しない。未設定・`off`・不正値は `Off` に解決(fail-closed)。
- `RVF_TEST_DISABLE` の truthy 値は全 serve 経路を拒否。
- TCP は `RVF_TEST_ADDR` 未設定なら `Off`、secret 未設定なら serve
  を拒否、`test.hello` の secret 不一致は `unauthenticated`。
- handshake 前の `test.*` op は `unauthenticated`、未宣言
  capability/op は `unavailable`/`unknown_op`、release 相当の env
  では制御ポートは存在しない。

## 正直な能力宣言

- `clock_domains` は `host_tick`(`WindowEvent::Tick` 促進)のみ。
  OS タイマー・壁時計・GPU/vsync・webview 内部時計は制御しない。
- `faults` は `window_crash`・`ipc_disconnect` のみの allowlist。
- `input_injection`/`dom_access` は常に `false` — 実操作注入と
  DOM 操作は RF-GT-04(OS/Browser driver、Issue #313)の port。
- `screenshot` は headless backend では `unavailable` — 合成
  screenshot は生成しない。実 window 面の未検証項目は
  RF-GT-04/#313・#243 へ引き渡す。

**RT 契約:** 全 API 非 RT(制御/観測/fixture は UI スレッド外の
非 RT 基盤)。RT クレートからは参照されない。

## Driver Boundary Guard / Driver Lab（Issue #390）

共通 UI 契約と Consumer 通常 View/Domain/Headless が
GPUI/Toolkit/OS handle に依存しないことを機械検査する guard と、
UI-DVR-01〜09 の case/story registry + Fake driver semantic
fixture を提供する。

- 入口: `mise run check-ui-driver-boundary`（guard、exit
  0=pass/1=violation/2=tool error）、
  `mise run test-ui-driver-contract`（manifest 整合 + 実行 fixture）。
- ruleset: `fixtures/driver-lab/boundary.json` — forbidden dep/path
  root・allowlist grant（`ruviform-ui-webview` =
  `wry`/`tao`/`dpi` optional+`webview` feature、`ruviform-ui-gpui` grant
  は `gpui`/`gpui-test` feature 経由の optional のみ・`required:true`）。
- 検査: dep-graph（direct/feature/dep.features/transitive/
  conditional/allowlist 整合/adapter-consumer）+ 公開 signature
  漏出（syn、exposure-aware 2-pass・alias・impl header）。
  検査限界（glob/macro/cfg/registry optional target 等）は
  `docs/designs/issue-390-driver-contract-lab.md` §限界参照。
- registry: `fixtures/driver-lab/cases.json`（requiredCases で
  UI-DVR-01〜09 を pin）と `stories.json`。判定語彙
  `pass/fail/not_run/unsupported/blocked` — Fake semantic pass を
  GPUI/実 OS/Native Plugin Dock/Visual Golden の成功としない。
  未実装 layer は owner Issue + rerun 条件付き `not_run`/`blocked`。
- evidence: `RUVIFORM_DRIVER_LAB_EVIDENCE` 指定時に JSONL を
  append（`expected`=manifest 宣言、`observed`=実実行結果）。
- 固定事項（Issue #390 phase 2）:
  - **Case manifest**: `fixtures/driver-lab/cases.json`（UI-DVR-01〜09、
    `requiredCases` で 01〜09 を pin）。**Story manifest**:
    `fixtures/driver-lab/stories.json`（LAB-SV-FADER-DOCK /
    LAB-AL-WAVEFORM-SEEK、2 theme × 2 scale factor）。
  - **Oracle の層分離**: `fake` = ViewHost/ThemeResolver/
    SurfaceSlotRegistry/DockWorkspace 等の semantic 契約（GPUI・OS 非
    依存）。`gpui` = `ruviform-ui-gpui` の `gpui-test`
    （TestAppContext + simulated input）上の semantic 検証で、
    実描画 pixel・OS 子 HWND 描画は対象外。`real-os` = OS Driver
    （#313）・Native Plugin Dock（#388/#389）の実機結果で、Fake/GPUI
    の pass で代替しない。Pixel Golden は色/配置のみで semantic
    oracle を置換しない。IME/a11y・各製品の precision/RT は別 Gate。
  - **mechanism の機械検査**: `cargo-test`（package・
    `tests/<target>.rs`・filter fn の実在）と `mise-task`
    （`mise.toml` の `[tasks.<name>]` 定義）を cases/stories 両方で
    検査する（`manifests_are_consistent`）。
  - **実行方法**: fake = `mise run test-ui-driver-contract` /
    `mise run test-ui-component-view`、gpui = `mise run test-ui-gpui`
    （headless `gpui-test`、Windows 11 x64 で 34 test pass を確認）、
    boundary = `mise run check-ui-driver-boundary`。real-os は手動
    （`example-ui-gpui-minimal` 等）で CI pass としない。
  - **環境差**: `gpui-test` は GPU/フォント/DPI に依存しない
    semantic lane。GPU/DPI/font/device の実差は real-os 層の
    証拠にのみ残す（OS 別 metadata は未取得）。
  - **証拠保存先**: `target/driver-lab/evidence.jsonl`（生成物、
    commit しない）。`RUVIFORM_DRIVER_LAB_EVIDENCE=target/driver-lab/evidence.jsonl`
    で有効化し、`mise run test-ui-driver-contract` の結果を追記する。
  - **依存境界の名指し pin**: `protectedPackages`（`ruviform-ui`・
    `ruviform-ui-theme`）が workspace member に存在し allowlist 例外を
    持たないことを検査（`protected-missing` / `protected-allowlisted`）。
    全 member の dep-graph 走査とは別に Common Core/Theme を名指しする。

既知の限界として、cargo metadata の resolve graph は未 activation
optional dep を省略するため conditional 到達は manifest 宣言を
再帰走査する（registry dep の未 activation target は skip）。

## Pixel Golden Utility（Issue #314 slice 1）

headless な pixel golden コア(`golden` モジュール)。実 pixel
capture は slice 3 の対象であり、本モジュールは取得済み
screenshot(`proto::Screenshot` の `png`/`rgba-raw`)を RGBA8 に
正規化し、baseline 比較・差分承認・evidence を提供する。

- key/env: `GoldenKey`(story/theme/state/driver・OS・GPU class/
  scale/font/revision の canonical key)と `EnvironmentMeta`
  (driver-neutral DTO、GPUI 型を含まない)。
- 画像: `RgbaImage`(RGBA8 正規化。alpha=0 は RGB を 0 に正規化
  し encoder 差分を吸収)、`proto::Screenshot` からの decode
  (base64 + png 0.18 / raw RGBA、寸法・長さ・上限を検証)。
- diff: `DiffPolicy`(per-channel `ChannelTolerance` + `MaskRect`
  mask)による決定的比較。delta == tolerance は match、超過のみ
  mismatch。stats(mismatch 数/ratio/max delta/bbox)と決定的
  diff 画像を返す。`compared_pixels == 0` や size mismatch は
  `pass` にしない(`blocked`/`fail`)。
- store: `BaselineStore` が `<root>/<story>/<theme>/<state>/
  <stem>.png|.json` に baseline + sidecar(schema
  `rvf.golden.baseline/v1`)を保持。更新は
  `write_candidate`(staging のみ)→ 人間レビュー →
  `promote`(`BaselineApproval` 必須)の 2 相。直接上書き API は
  無く、破損・key 不一致は `StoreError`。
- report/evidence: `GoldenReport`/`GoldenComparison`/
  `GoldenEvidence`。verdict 語彙は driver-lab と同一
  `pass/fail/not_run/unsupported/blocked` + machine-readable
  `reason`。`RUVIFORM_GOLDEN_EVIDENCE` 設定時のみ JSONL(schema
  `rvf.golden.evidence/v1`)を append — 通常 test は副作用なし。
- 境界: semantic oracle・DrawPlan・story fixture(#464/#426)・
  実 OS window capture(#313)・capture 経路(slice 3)は対象外。
  pixel golden は semantic oracle を置換しない。
- 検証: `mise run test-ui-golden`(統合 test)、unit test は
  `cargo test -p ruviform-ui-test` に含まれる。baseline
  layout・承認手順の正本は `fixtures/golden/README.md`。

**RT 契約:** golden module も非 RT(画像処理・store I/O)。
