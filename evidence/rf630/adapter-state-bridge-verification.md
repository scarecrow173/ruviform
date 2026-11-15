# RF630 adapter-state-bridge slice 実施 report — 両 adapter host bridge の標準 ABI state 境界

- Issue: https://github.com/scarecrow173/ruviform/issues/630 (adapter-state-bridge slice)
- Branch: `feat/630-adapter-state-bridge`(worktree `G:\orca\workspaces\ruviform\rf630-state-bridge`)
- Base: main `abb9f372`(前提 slice: VST3 test_gain PR#641 / CLAP test_gain PR#657 / L1 runner PR#661 merge 済)

## 実施内容(本 slice)

| 区分 | 内容 |
| --- | --- |
| VST3 `host.rs` | `BridgeStream`(`IBStream` の `ComWrapper`/`Class` 実装 — `RefCell<{data: Vec<u8>, pos: u64}>`、read/write/seek/tell、部分転送の正直な報告、null/負引数 `kInvalidArgument`、borrow 競合・負/overflow seek `kResultFalse`、前方 seek 許可 + 0 埋め gap write、`WIRE_MAX_STATE_BYTES` cap)+ `frame_state`/`unframe_state`/`FRAME_HEADER_BYTES(8)` + `save_component_state`/`restore_component_state` + `Vst3Plugin::{save_state,restore_state}`(phase gate `Initialized` 以降、`kNotImplemented`→`Unsupported`/その他→`PluginFailed`)+ module doc 更新 |
| VST3 blob | `[u32 LE comp_len][comp][u32 LE ctrl_len][ctrl]`。single-component は `ctrl_len=0`・controller 呼び出しなし。restore は spec 順 `component.setState`→`component.getState`→`controller.setComponentState`(正規化 stream seek(0))→`controller.setState` |
| foreign blob 規則 | framing 最小生成長 8 byte 未満は本 adapter の serialization ではあり得ないため foreign state として plugin の `setState` へそのまま委譲 — 厳密 unframe 先行は `RestoreState{[0xAA,0xBB]}` を plugin 呼び出し前に `PluginFailed` とし lease 外 L1 e2e の `Unsupported` pin を破るための確定調整(design「foreign blob の扱い」)。8 byte 以上の corrupt frame は `PluginFailed` |
| CLAP `Cargo.toml` | `host` feature に `"clack-extensions/state"` 追加(既存 non-optional 依存への additive feature flag のみ。`Cargo.lock` 差分なし) |
| CLAP `host.rs` | `PluginState` import + module doc 更新 + `ClapPlugin::{save_state,restore_state}`(`instance=None`→`PluginFailed`、extension 無し→`Unsupported`、`StateError`→`PluginFailed`、blob は無 framing 中継)+ test 書き換え。host 側 `HostState`/`mark_dirty` は未登録(KISS) |
| `controller.rs` | side finding 修正: `GainController::setState`/`getState` が `kResultOk` の空 no-op で成功を偽装 → `kNotImplemented` に修正 + `state_stream_reports_not_implemented` test 追加(component.rs stub 検証の mirror) |

## 実機実行証拠(Windows OS-L1)

実行 tuple:

| 項目 | 値 |
| --- | --- |
| TestID | rf630-adapter-state-bridge-win |
| OS | Windows(x64、Git Bash 経由の mise/cargo) |
| toolchain | mise pin の rust(edition 2024)、`vst3 0.3.0` / `clack-* 0.2.0` |
| process boundary | adapter unit test は **in-process COM double**(実 vtable dispatch 経由の stateful `IComponent`/`IEditController` double)。L1 e2e は **out-of-process** runner spawn(変更なしの oracle) |
| artifacts | `ruviform_plugin_{vst3,clap}` cdylib fixture + 両 crate の lib test binaries |
| commands | 下記「検証」表の通り |

観測された実効果:

- VST3 in-process double(distributed): `save_component_state` が `component.getState`→`controller.getState` をこの順で呼び comp/ctrl payload を返却、`frame_state` で round-trip。`restore_component_state` が spec 順 4 呼び出しを実行し、`setComponentState` には `component.getState` の再シリアライズ結果が渡ることを call log + payload で pin。single-component flag では controller 動詞ゼロ・`ctrl_len=0`。
- VST3 `BridgeStream`: write→tell→seek(0)→read の round-trip、EOF 部分 read(`numBytesRead < numBytes`)、前方 seek 後 write の 0 埋め gap、null/負引数・不正 mode・負/overflow seek の defined refusal。
- error mapping: double の `state_result=kNotImplemented` → `Err(Unsupported)`、`kResultFalse` → `Err(PluginFailed)`、corrupt frame(trailing byte/len 超過)→ `Err(PluginFailed)` かつ plugin 呼び出しゼロ、`[0xAA,0xBB]` foreign blob は `component.setState` へ到達。
- fixture(stateless)`: `save_state`/`restore_state` は init 後 `Err(Unsupported)`(VST3 `kNotImplemented`、CLAP extension 無し)、pre-init は `Err(PluginFailed)` — **wire 越しの L1 e2e `real_adapter_l1_full_series` 3+3 pass で実証**。

## 検証(実 command/exit)

| command | 結果 |
| --- | --- |
| `mise run fmt-check` | exit 0 |
| `mise run check ruviform-plugin-vst3` | exit 0 |
| `mise run check ruviform-plugin-clap` | exit 0 |
| `mise run lint ruviform-plugin-vst3` | exit 0(`clippy -- -D warnings`) |
| `mise run lint ruviform-plugin-clap` | exit 0(同上) |
| `mise run test ruviform-plugin-vst3` | exit 0 — 42 unit pass(default feature。host feature 側は下記補助証拠) |
| `mise run test ruviform-plugin-clap` | exit 0 — 48 unit + doctest 3 pass |
| `mise run check-l1-real-plugin` | exit 0(4 crate・両 feature 組合せ) |
| `mise run lint-l1-real-plugin` | exit 0(同上、`-D warnings`) |
| `mise run test-l1-real-plugin` | exit 0 — adapter e2e `real_adapter_l1_full_series` 3+3、test-fixture e2e 3+3 + fault e2e 8+8、host-feature lib 67+84 pass 全て緑 |
| `mise run test-rt` | exit 0 — `cargo test --workspace rt_` 全 pass(192 suite、0 fail) |
| `mise exec -- prek run --all-files` | exit 0 |

補助証拠(canonical = mise): `cargo test --locked -p ruviform-plugin-vst3 --features host --lib` → 84 pass(新規 state/stream test 9 件含む)、`cargo test --locked -p ruviform-plugin-clap --features host --lib` → 67 pass。`git diff` で lease 外パス差分なし(Cargo.lock 差分なし)。

## 未検証(NotRun / Unsupported)

| 項目 | 状態 | 理由・後続 |
| --- | --- | --- |
| real-plugin stateful round-trip(save→restore で値復元) | **NotRun** | workspace fixture は両 format とも正直に stateless — real-ABI oracle は `test_stateful` fixture slice で初めて到着する。本 slice の in-process double 成功を real-plugin Pass と読み替えない |
| `PluginState::save`/`load` の positive arm(CLAP) | NotRun | fixture が `PluginState` extension を宣言しない。同上 |
| VST3 single-component 実 plugin での state 往復 | NotRun | fixture は distributed 形。flag 分岐は double test で検証済み、実 plugin は後続 slice |
| 外部 SDK validator(VST3 validator/clap-validator) | NotRun/Deferred | license・version 記録と共に後続 slice |
| Preset/preset-list | NotRun | 後続 slice(本 slice は state blob のみ) |
| `mark_dirty`/dirty 通知(CLAP `HostState`・VST3 `IComponentHandler`) | Unsupported(未実装) | neutral contract に対応 surface が無く後続 slice の判断材料として design に記録 |
| `test_instrument`/`test_editor`/GUI-L2 | NotRun | 別 slice(#631 連携) |
| 非 Windows OS 実走 | NotRun | 本 slice の実測は Windows OS-L1 のみ。lane は platform-neutral で CI が 3OS で拾う |
| MSRV lane(`*-msrv`) | NotRun | ユーザー指定で対象外(daily CI 委譲) |
| `*-full` lane | NotRun | 変更範囲比例の検証、Full CI に委譲 |
| consumer oracle(synvelis#101/#127/#128/#176、audio-lens#127)照合 | NotRun | 後続 slice |

## test-only / 境界

- `ruviform-plugin-test-*` は zero-diff(`test_stateful` fixture は後続 slice)。stateful double は `#[cfg(test)]` 内の test-only COM class で本番経路・fixture 宣言に影響しない。
- `BridgeStream` の `Vec`/`RefCell` は state 呼び出し内に限定生成・破棄される non-RT オブジェクト — `process` 経路に alloc/lock/wait は載らない(`test-rt` 全緑で不変条件を確認)。
- `Cargo.toml` の変更は `host` feature への `"clack-extensions/state"` 1 行のみ — 新規 crate 追加なし、`Cargo.lock` 差分なし。
