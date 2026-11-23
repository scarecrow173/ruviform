# ruviform#891 検証記録 — plugin→host state dirty 通知経路（CLAP mark_dirty・VST3 setDirty/performEdit）

実施日: 2026-11-23 / worktree: `rf891-dirty-notification` /
branch: `scarecrow173/rf891-dirty-notification`。OS: Windows /
workspace pin toolchain。base SHA `fcb35203`（branch 起点・作業
開始時点の origin/main）。

本 slice は Issue #891 の plugin→host state dirty 通知経路:
plugin-core に中立 `DirtyNotice`/`DirtyNoticeQueue`（flag latch
coalesce + bounded edit FIFO）と pull verb
`Plugin::drain_dirty_notices`、CLAP adapter の `HostState` 登録
＋ `mark_dirty` enqueue、VST3 adapter の
`IComponentHandler`/`IComponentHandler2` bridge handler
（`performEdit`/`setDirty`/`restartComponent(kParamValuesChanged)`
写像）、L0 `InProcessHost`/composite drain、wire v7 の
`DrainDirtyNotices`(tag28)/`DirtyNotices`(tag9) + chunked backlog、
両 test suite の stateful fixture 発火と `#[ignore]` e2e oracle。

## canonical lane — `mise run test-l1-real-plugin`（task 定義 step 順）

lane は sequential 8 step。step 3（adapter e2e）と step 6
（test-plugin e2e）に **base HEAD からの既存破損**があり lane
全体は exit 0 で完走しない（後述「既存破損」）。各 step の結果:

| step | 内容 | 結果 |
| --- | --- | --- |
| 1 | `cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host` | PASS |
| 2 | 同上 `--lib --bins` | PASS（79 + 2 + 91 + 2 tests green） |
| 3 | `--test l1_real_adapter_e2e -- --ignored` | **2/3 pass・`real_adapter_l1_full_series` FAIL（既存破損）** |
| 4 | `cargo build -p ruviform-plugin-test-vst3 -p ruviform-plugin-test-clap --features l1-runner` | PASS |
| 5 | 同上 `--lib --bins` | PASS（77 + 3 + 116 + 3 tests green） |
| 6 | `--test l1_real_test_plugin_e2e -- --ignored` | **10/11 pass・`real_test_plugin_l1_full_series` FAIL（既存破損）** |
| 7 | `--test l1_fault_test_plugin_e2e -- --ignored` | PASS（21 tests green） |
| 8 | `--test l1_stateful_test_plugin_e2e -- --ignored` | **PASS（6/6 — 両 ABI の `real_stateful_test_plugin_l1_state_restore_announces_dirty` を含む）** |

step 4–8 は step 3 の fail-fast で lane が中断したため task 定義の
同一 command を `mise exec --` で個別実行（fixture cdylib・runner
bin は step 4 が生成）。

## 既存破損（本 slice 起因でない — 修正せず報告）

- `l1_real_adapter_e2e.rs:417` と `l1_real_test_plugin_e2e.rs:585`
  の `HostCommand::CloseGui { handle }` assert が
  `Err(HostError::Unsupported)` を期待するが、child 側
  `Runner::dispatch_gui`（`ruviform-plugin-export`）は surface-less
  instance への `CloseGui` に settled vocabulary として
  `Err(HostError::InvalidValue)` を返す（`gui_handle.is_none()` →
  `InvalidValue`、コメント上「CloseGui's settled no-surface
  vocabulary, InvalidValue, generalized to the family」— #907 系）。
- `git show HEAD:` で確認: 両 test の assert 文と export の
  `InvalidValue` 分岐は **base `fcb35203` で同一** — 本 slice の
  diff は GUI leg・`gui_handle` bookkeeping・capability 広告に
  触れていない（export diff は `dirty_backlog` 追加と
  `DrainDirtyNotices` arm のみ）。base HEAD でも同一 signature で
  再現する deterministic failure。
- 対象 file は本 slice の lease 外のため変更せず、Issue には別途
  報告する。

## その他 lane（全て PASS）

| lane | 結果 |
| --- | --- |
| `mise run fmt` → `cargo fmt --all -- --check` | PASS |
| `mise exec -- cargo check`（plugin-core / plugin-host / plugin-export / plugin-clap+host / plugin-vst3+host / plugin-test-clap+l1-runner / plugin-test-vst3+l1-runner、各 `--all-targets`） | PASS |
| `mise run check-l1-real-plugin` 相当（上記 feature 和の `cargo check --all-targets`） | PASS |
| `mise run lint-l1-real-plugin` 相当（同 feature 和の `cargo clippy --all-targets -- -D warnings`） | PASS |
| `mise run test` 系: plugin-core・plugin-host・plugin-export・plugin-clap・plugin-vst3・plugin-test-clap・plugin-test-vst3 | PASS（235/57/18/5/8/190/115+6ignored/23/36/15/3 等 — 本 slice 追加 test を含む全件 green） |
| `editor_session_l1` 再実行 | PASS（初回の 1 件 fail は再現せず transient 判定） |

## 本 slice の新規 oracle（全て PASS）

- `DirtyNoticeQueue` 単体 test 5 件（coalesce・clean edge・edit
  FIFO→flag・edit⇒dirty latch・overflow drop+count）。
- wire codec: `DrainDirtyNotices`(28)/`DirtyNotices`(9) round-trip、
  `DirtyNotice` field codec（variant round-trip・未割当 tag
  `Malformed`・非有限 `Malformed`・短 payload `Truncated`）、
  `WIRE_MAX_DIRTY_NOTICES` 超過 `LimitExceeded`、`WIRE_VERSION` = 7 pin。
- VST3 handler 写像 test（`plugin-vst3 host.rs::tests`）:
  `performEdit`→`ParamEdited`+flag、`setDirty(0/1)`→`Clean`/`Dirty`、
  `restartComponent(kParamValuesChanged)`→`StateDirty`、非有限
  `performEdit`→`StateDirty` degrade、他 flag・`requestOpenEditor`・
  group-edit は `kNotImplemented`、begin/end edit は `kResultOk`。
- CLAP fixture `mark_dirty` 発火 test（`plugin-test-clap
  stateful.rs::tests` — `HostState` 登録 host で restore 値変更
  edge が `mark_dirty` 1 回・同一値 restore は増えない）。
- composite routing test（`drain_dirty_notices_routes_by_slot_kind`
  — L0 `Shared` Drained+payload・L1 audio 未接続 `Ipc`・L1 video
  `Unavailable`・stale `InvalidHandle`）。
- L1 stateful e2e 両 ABI（step 8 — `RestoreState` → drain で
  `[ParamEdited{0,0.5}, StateDirty]`（VST3）/`[StateDirty]`（CLAP）
  到達を oracle）。

## 設計上の注意（VST3 fixture の announce 契約）

- `setComponentState` は verb 自体が change signal — 無条件に
  `performEdit`+`setDirty` announce（`adopt_and_announce`）。
  controller の adopted mirror は process-path の
  `IParameterChanges` を見ない（fixture は `IConnectionPoint`
  messaging を持たない）ため stored-value 比較では component が
  実際に変化した restore を抑止する — 初期版の change-gated
  `store_and_announce` が e2e で `[ ]` を返した原因であり、
  本 slice 内で修正済み。
- `setState`（controller-private state）は値変更 edge のみ
  `setDirty` で発火、`performEdit` echo なし（`store_and_mark`）。

## 未実施（対象外）

- `*-full`・`*-msrv`・3OS 横断・ABI validator lane — 本 slice の
  対象外。doc test は doctest 記述のある crate に付随。
