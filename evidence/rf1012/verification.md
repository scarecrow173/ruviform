# rf#1012 検証記録

## 環境と対象

- 記録日時: 2026-11-25T17:38:48Z（2026-11-26 02:38 JST）
- OS: Microsoft Windows 11 Pro build 26300（x86_64/AMD64、MSYS2 上の bash から mise を実行）
- CPU: Intel(R) Core(TM) i9-9900K CPU @ 3.60GHz、16 logical processors
- Rust: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1 (5f94df478 2026-08-27)
- worktree: `G:/orca/workspaces/ruviform/rf1012-bus-enumeration`
- 検証時のベース SHA: `b090fd476b3a`（`origin/main` と一致。検証対象はこの上の未コミット diff）
- real ABI fixture artifact（検証後の worktree artifact の sha256）:
  - `target/debug/deps/ruviform_plugin_test_vst3.dll`: `dfe5af6855b650fff9fcbb2148fd91893943f48d83ec352531ebb299ebc038eb`
  - `target/debug/deps/ruviform_plugin_test_clap.dll`: `5893ce9335491120e8073a51f3da3360948d4684459c2f4fb1cc45e71c649235`

## 正規検証コマンド（canonical = mise）

集計表: `verification-results.tsv`。最終 sweep の個別 raw log は `logs/` 配下。real fixture/L1 lane をまとめて実行した raw log は `logs/rf1012-lane-runner.txt`。

| lane | command | exit | result | 備考 |
| --- | --- | ---: | --- | --- |
| fmt-check | `mise run fmt-check` | 0 | Pass | 最終 sweep でも 0 |
| check plugin-core | `mise run check ruviform-plugin-core` | 0 | Pass | |
| lint plugin-core | `mise run lint ruviform-plugin-core` | 0 | Pass | |
| test plugin-core | `mise run test ruviform-plugin-core` | 0 | Pass | |
| check plugin-host | `mise run check ruviform-plugin-host` | 0 | Pass | |
| lint plugin-host | `mise run lint ruviform-plugin-host` | 0 | Pass | |
| test plugin-host | `mise run test ruviform-plugin-host` | 101 → 0 | Pass（rerun） | 初回は既存の L1 runner artifact 不足で失敗。runner binary を build して rerun した最終結果は 0。本変更とは無関係のローカル artifact 前提 |
| check bridge | `mise run check ruviform-audio-plugin-bridge` | 0 | Pass | |
| lint bridge | `mise run lint ruviform-audio-plugin-bridge` | 0 | Pass | |
| test bridge | `mise run test ruviform-audio-plugin-bridge` | 0 | Pass | |
| test VST3 adapter | `mise run test ruviform-plugin-vst3` | 0 | Pass | 最終 source で rerun して 0 |
| test CLAP adapter | `mise run test ruviform-plugin-clap` | 0 | Pass | 最終 source で rerun して 0 |
| check L1 real plugin | `mise run check-l1-real-plugin` | 0 | Pass | |
| lint L1 real plugin | `mise run lint-l1-real-plugin` | 101 → 0 | Pass（rerun） | 初回は本変更の clippy 指摘で失敗。修正後の rerun は 0 |
| test VST3 fixture | `mise run test ruviform-plugin-test-vst3` | 0 | Pass | `reports_gain_bus_table_through_inprocess_host` と `attach_gain_bus_validation_reaches_endpoint_preparation` を含む 28 passed |
| test CLAP fixture | `mise run test ruviform-plugin-test-clap` | 0 | Pass | 同 2 テストを含む 28 passed |
| test L1 real plugin | `mise run test-l1-real-plugin` | 0 | Pass | real adapter / fixture L1 ignored lanes すべて pass |
| compile sweep | `mise run check-full` | 0 | Pass | trait signature 変更の workspace-wide compile sweep |
| lint sweep | `mise run lint-full` | 0 | Pass | 実行可能だったため NotRun ではなく Pass |
| prek | `mise exec -- prek run --all-files` | 0 | Pass | 全 hook pass、変更なし |

macOS/Linux の real ABI lane は Windows Worker に該当 lane がなく実行できないため NotRun（lane 新設は本 slice 対象外）。Windows ローカルで macOS/Linux 結果を偽装していない。統合検証は merge 後の対象 SHA に対する必須 CI 証拠であり、上表はローカル変更範囲の検証である。

## 受入条件との対応

- real `test_gain` VST3/CLAP fixture を production host bridge と `InProcessHost` で Register/Scan/Instantiate/Initialize し、`host.buses(handle)` が input stereo Main → output stereo Main の `BusReport::Reported` を返すことを、両 `tests/host_path.rs` の `reports_gain_bus_table_through_inprocess_host` で固定した。VST3/CLAP fixture test laneはそれぞれ exit 0。
- consumer declared stereo input/output の `attach()` が bus validation で `MissingPort` に倒れず、現行では `create_sendable=None` 由来の後段 `EndpointUnavailable` まで到達することを、両 `attach_gain_bus_validation_reaches_endpoint_preparation` で固定した。両 lane exit 0。
- unavailable default と declared-empty `Some(&[])` の区別を `buses_distinguish_unavailable_and_declared_empty_{plain,shared}_slot` で固定し、`mise run test ruviform-plugin-host` rerun exit 0。
- L1 wire protocol と `composite.rs` は未変更。L1 slot は引き続き `Unavailable`。

## 実装上の正直な制限

- ABI bus/port name は dynamic で `BusDescriptor.name: &'static str` の ownership contract に安全に載らないため、v1 では空文字へ投影する。ownership contract redesign は別 Issue 候補。
- bus table の公開は topology declaration の公開であり、multi-bus processing capability ではない。VST3 host processing path は現行 v1 で exactly one input/output bus boundary のまま。
- CLAP audio-ports extension absent は CLAP 契約上 no audio ports とみなし `Some(&[])`（declared none）とする。extension present の inconsistent/unrepresentable table は `None`（unavailable）。
- VST3 では `enabled` を ABI の `kDefaultActive` と混同せず、table 内 bus の存在を available とみなして `true` に固定する写像を rustdoc に明記した。

## lease 外 compile impact / downstream impact

- `crates/ruviform-audio-plugin-bridge/tests/param_delivery.rs` は spec 本文では lease 外と記載されていたが、Coordinator が dispatch lease に既に追加済みであることを ask で確認し、承認済みの mechanical follow-up として test mock を `Some(&BUSES)` へ更新した。
- `check-full` / `lint-full` は最終 source で exit 0。追加の lease 外 implementor/callsite は残っていない。
- synvelis 側で `Plugin::buses`/`ErasedPlugin::buses` を実装する consumer は、pin 更新時に `Option<&[BusDescriptor]>` への mechanical follow-up が必要になる downstream impact がある。本 PR では synvelis/audio-lens/gpui を変更していない。
