# RF-VTEST CLAP output-drain slice — 検証記録

- Issue: `scarecrow173/ruviform#842`（Issue #630 残りスコープ —
  CLAP plugin→host 出力イベント drain、#630 は未 close）
- Slice: `ClapPlugin::process` の `OutputEvents::void()` を永続
  構造的有界バッファへ置き換え、プラグイン出力イベントを
  canonical レポートとして `drain_output_reports` 経路へ実装
  （VST3 #253 parity）。
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints 正本）
- 記録日: 2026-11-21
- deterministic evidence と real-run evidence を分離。

## 実装概要

- `crates/ruviform-plugin-clap/src/host.rs`:
  - `BoundedOutputBuffer`（`OutputEventBuffer` impl）— 件数
    `MAX_EVENTS_PER_BLOCK`(256) と単一イベントバイト
    `size_of::<TransportEvent>` の二重上界で `try_push` を
    `Err(TryPushError)` に落とす。clack stock `EventBuffer` は
    `Vec` 再確保するため構造的バウンドは wrapper 層の責務。
    `prepare` で `with_capacity(256)` 予約 — RT パス再確保なし。
  - `pending_reports: Vec<Event>`（`prepare` で 256 予約・未 drain
    溢れは defined drop として `skipped` 計数）、
    `ReportStats{translated, skipped, refused}`（`prepare` では
    reset せず累積 — VST3 `MidiBridgeStats` と同法則）。
  - `collect_output_reports`（`process` 後、`frame_offset`
    `sort_unstable`）+ `drain_output_reports` override
    （非 RT、caller `Vec` へ `append`）。
  - `prepare` で `output_events`/`pending_reports` をサイズ、
    `process` 冒頭で `output_events.clear()`（capacity 保持）。
- `crates/ruviform-plugin-clap/src/adapt.rs`:
  `pub(crate) fn translate_output_event`（coordinator 承認の
  lease 拡張、既存 helper は不変）— `ParamValue` はパラメータ
  探索 + `translate_param_event`、`Midi`/`Midi2` は
  `translate_midi_event` 委譲、`NoteOn`/`NoteOff` は
  `translate_note_event`（concrete channel/key のみ、
  `port_index Specific(p)` → `MidiGroup(p)`・`All` → `ZERO`、
  velocity は `norm_f32_to_u32` 法則の逆写像）。`u32::MAX`
  スタンプは忠実転送、NaN/未知 param/未サポート kind は
  defined skip。
- `crates/ruviform-plugin-test-clap/tests/host_path.rs`: L0 2 件
  （`bad_time` 観測 + `invalid` defined skip）。
- `crates/ruviform-plugin-test-clap/tests/l1_fault_test_plugin_e2e.rs`:
  `real_test_fault_l1_output_events_drain`（`--ignored`、実
  runner opt-in lane）+ 既存 `bad_time` コメントの更新。
- `crates/ruviform-plugin-{clap,test-clap}/README.md`,
  `fault.rs` モジュールドキュメント: `void()` 時代の記述を
  drain 実装後の挙動へ更新。
- wire / plugin-core / export / composite / VST3: **zero-diff**
  （`git diff` で確認 — 変更は上記 7 ファイル + docs/evidence のみ）。

## 実行した検証（全て実走、未実行は記録しない）

| コマンド | 結果 |
| --- | --- |
| `mise run fmt` / `mise run fmt-check` | exit 0 — rustfmt 全件 clean |
| `mise run check ruviform-plugin-clap --features host` | exit 0 |
| `mise run check-full` | exit 0 — workspace 全体 `--all-targets` |
| `mise run lint ruviform-plugin-clap --features host` | exit 0 — `-D warnings` |
| `mise run lint ruviform-plugin-test-clap` | exit 0 |
| `mise run lint-l1-real-plugin` | exit 0 — 4 クレート × l1-runner/host feature 合成 `-D warnings` |
| `mise run lint-full` | exit 0 — workspace clippy `-D warnings` |
| `mise run test ruviform-plugin-clap --features host` | exit 0 — lib 76 pass（新規 unit test 5 件含む）・doc 3 pass |
| `mise run test ruviform-plugin-test-clap` | exit 0 — lib 77・`host_path` 23 pass（新規 L0 2 件含む） |
| `mise run test-l1-real-plugin` | 全 lane exit 0 — `l1_real_test_plugin_e2e` 両 binary 各 10 pass、`l1_fault_test_plugin_e2e` 20 pass（`real_test_fault_l1_output_events_drain` 含む） |
| `mise run test-rt` | exit 0 — workspace `rt_` lane、`host::tests::rt_process_drives_fixture_block_allocation_free` が `assert_no_alloc` 下で pass |
| `mise exec -- prek run --all-files` | exit 0 — 全 hook pass |

### 新規テスト（実走結果）

- `host::tests::bounded_output_buffer_refuses_past_count_bound` —
  256 受理 → 2 連続 `Err(TryPushError)`、`take_refused == 2` で
  block ごと reset、`clear` 後は受理再開。
- `adapt::tests::translate_output_event_*`（4 件）—
  `ParamValue` 翻訳（plain 1.0 → normalized 0.5、offset 保持）、
  `u32::MAX` スタンプ忠実転送、未知 id/NaN defined skip、
  `NoteOn`/`NoteOff` → `EventKind::Midi`（channel/key/velocity
  `0.5`→u7 `64`/group 写像、wildcard・非有限 velocity skip）、
  `Midi` 委譲 + `NoteChoke`/`ParamGestureBegin`/`ParamMod` skip。
- `host_path.rs`:
  `fault_bad_time_output_event_is_host_observable` —
  `drain_output_reports` が丁度 1 レポート（`u32::MAX`、
  `ParamValue`、`ParamId(0)`、0.0）を返し 2 回目は空;
  `fault_invalid_output_event_is_defined_skip` — `bad_time`
  で drain が live なことを証明後、`invalid` arm block で空。

### real-run evidence（L1、`--ignored` lane）

`real_test_fault_l1_output_events_drain`:
`PluginHost` + 実 `ruviform-plugin-test-clap-l1-runner` 経由。
`bad_time` arm で `TAIL_INFINITE` claim が 2 matched block に着弾
→ `drain_output_reports` → `Ok(ReportDrain::Drained)`、≥2 件、
全て `frame_offset == u32::MAX`/`ParamValue`/id 0/値 0.0。
`invalid` arm で 2 matched all-NaN block → flush drain → 追加
2 NaN block → drain が空。lifecycle は `Processing` のまま、
disarm → clean teardown。

### 既知の flake（本 diff とは無関係）

`real_test_fault_l1_hang_converges_on_timeout` が lane 初回実行で
`SpawnFailed { cause: Timeout }` を 1 度返した（`command_timeout`
500 ms が spawn Hello にも適用される設計上の環境感度。spawn 経路は
本 diff の範囲外で、単独再実行と lane 再実行では pass — 環境 flake
として記録）。

## 設計上の留意点（判断記録）

- **faithful-stamp**: `u32::MAX` は `frame_offset`（`u32`）で
  表現可能であり、#253 契約は offset を再解釈しない — 変形
  スタンプそのものが fixture の観測対象のため忠実に届ける。
  VST3 入力側の block 外 drop とは意図的に異なる選択
  （出力レポート面の法則）。
- **NaN skip**: `ParamInfo::normalize`/`ParamValue` が非有限を
  表現不能 — `0.0` 捏造せず defined skip + 計数。
- **armed ≠ matched**: L1 の `passthrough == 0` は「子が実
  レンダー」の証拠だが arm 発火後とは限らない（arm は子の
  `process()` で非同期発火）。L1 oracle は per-block 観測証拠
  （`tail_frames == u32::MAX` / 全 NaN）で armed block を数える
  既存 lane の法則に従う。遷移窓 straggler は flush drain で分離。
