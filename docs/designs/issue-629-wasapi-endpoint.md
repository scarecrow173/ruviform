---
id: "DESIGN-034YaVs6WbghWj1JKJNZjZ"
type: "design"
status: "approved"
title: "Issue #629 WASAPI streaming verifier and endpoint probe extension design"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/specs/issue-629-wasapi-endpoint.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-629-wasapi-endpoint.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-wasapi-endpoint.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-wasapi-endpoint.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-629-wasapi-endpoint.md"
    - "docs/test-specs/issue-629-wasapi-endpoint.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-midi-loopback.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 WASAPI streaming verifier and endpoint probe extension design

## Context

Issue #629 の 4th slice。Audio 側仮想 Endpoint に対し「本番
Backend 入口を実際に open/stream して signal が往復するか」を
検証する経路を headless 範囲で実装する。3rd slice までに確立した
`#[ignore]`-gated verifier 型（`tests/loopback.rs`）と capability
probe（`examples/endpoint_probe.rs`）を Audio streaming に適用する。

利用する既存契約（調査済み）:

- `ruviform_audio_device::default_host()` → `Host` —
  `DeviceEnumerator` 実装。`output_devices()`/`input_devices()`/
  `default_*_device()` が `Box<dyn Device>` を返す。host name は
  `"WASAPI"`（Windows）。
- `Device`（`ruviform_audio_core::device`）— `name()`/`id()`/
  `default_{output,input}_format()`/`supported_{output,input}_
  formats()`/`open_{output,input}_stream(config, data_cb,
  error_cb)`。F32 のみ open 可能（adapter が交渉済み
  `AudioFormat` を要求）。
- `OutputStream`/`InputStream` — `start()`/`stop()`/`format()`/
  `buffer_size()`/`now()`/`clock_domain()`（+ input は
  `input_latency()`、output は `output_latency()`）。
  `Drop` で停止。`close()` は存在しない — `StreamInvalidated` は
  device removal 時に error callback へ届く typed error であり、
  非対話では再現不能。
- callback: `OutputDataCallback` =
  `FnMut(AudioBufferViewMut<'_, f32>, OutputCallbackInfo)`、
  `InputDataCallback` =
  `FnMut(AudioBufferView<'_, f32>, InputCallbackInfo)`。view は
  channel-major planar（`channel(i)`/`channel_mut(i)`）。
  `InputCallbackInfo` は `timestamp.callback`/`timestamp.capture`
  （`Option<MediaTimestamp>`）/`clock_domain`/`frame_position`
  （WASAPI native path では `Some`）。
- `tests/real_input_e2e.rs` — `#![cfg(feature = "real-device")]` +
  `#[ignore]` + watchdog + atomic 観測 + format 交渉 helper の
  実機 lane 先例。本 verifier は同じ feature を流用する（新規
  feature 追加なし）。
- `tests/loopback.rs`（3rd slice）— pair candidate 選定・
  verdict 出力・fail-closed `NotRun`・独立 oracle の先例。
- `examples/endpoint_probe.rs` — `VIRTUAL_NAME_HINTS` heuristic・
  `is_virtual_name`・`#[cfg(windows)]` module + `Command` による
  `cmd /c ver` 等の best-effort 外部呼出先例。
- `examples/midi_endpoint_probe.rs` — `powershell.exe -NoProfile
  -NonInteractive -Command` による WMS availability 検出先例。

重要な設計上の発見・制約:

- **name match は candidate であり virtual の証明ではない** —
  VB-CABLE 系は render "CABLE Input" ↔ capture "CABLE Output"
  のように方向語を含む名前で対になる。probe の `is_virtual_name`
  heuristic を流用し、方向語を除いた正規化 stem の一致で pair
  candidate を結ぶ。pair が見つかってもそれが仮想 endpoint で
  ある証明にはならず、Pass は OS-L1「実 OS API 往復」証拠に留まる
  （`environment_kind=virtual-os-endpoint` は provisioning の人間
  確認が別途必要 — fail-closed 境界は spec/test-spec で固定）。
- **cable 内部 SRC（VB-CABLE 系は内部 48kHz・SRC あり）を前提に
  bit-exact 比較は禁止** — oracle は marker 検出・level 比・
  基音スペクトル集中度の tolerance 系とし、期待値はテスト内の
  独立 deterministic generator が生成する（被試験 adapter は
  期待値を生成しない — 自己参照 Oracle 禁止）。
- **#628 `ruviform-audio-test` への dev-dep は coordinator
  不承認**（2026-11-15 承認済み依存リスト外。audio-test が引く
  `ruviform-audio`/`plugin-bridge`/`graph` 系が default の
  `cargo test -p ruviform-audio-device` compile graph に載る
  問題もある）。よって verifier はテスト内完結の独立 oracle で
  実装し、`PcmFixture`/`oracle::pcm_reference_block`/
  `RunMetadata` との結線 IF を docs に記録して後続 slice へ
  hand-off する。
- **`StreamInvalidated` は非対話では再現不能** — audio stream に
  `close()` は無く、invalidation は device removal の error
  callback 経由のみ。verifier は lifecycle（stop→query→再
  start→stop→drop）を確認し、invalidation 自体は NotRun として
  記録する（interactive unplug は AC5/後続 slice）。
- **RT 契約**: data callback は allocation-free・bounded が契約。
  capture 側は callback 内で pre-allocated buffer への bounded
  copy のみ行い、溢れは observable な drop counter にする
  （`Mutex::try_lock` — block しない）。手動検証 lane であり
  engine への handoff ではないため、この範囲の copy は契約の
  「bounded copy into pre-allocated handoff block」に相当する。
- **3OS lane**: verifier は production port trait のみを使い
  cfg 分岐なしで全 OS compile — 非 Windows・virtual name 不在は
  NotRun。probe の PowerShell/registry 依存は `#[cfg(windows)]`
  module 内。

## Scope

### In Scope

- `tests/virtual_endpoint.rs` 新設（`#![cfg(feature =
  "real-device")]`、`#[ignore]` verifier 1 本 + 補助の
  enumeration/format 確認を同 test 内に含める）。
- `examples/endpoint_probe.rs`: pair candidate 表・AC4 best-effort
  記録（driver 署名・microphone 権限・cleanup）・verdict 文言
  更新・module doc 更新。
- docs chain・evidence・discovery 更新・lane 設計記録。

### Out of Scope

- spec の Out of Scope と同一（実 provisioning・AC5 実測・AC6
  lane 登録・Test Runtime 完全結線・MIDI attach・consumer 照合・
  production 契約変更・新規依存/feature/unsafe）。

## Components and Boundaries

### `tests/virtual_endpoint.rs` — WASAPI streaming verifier

`#![cfg(feature = "real-device")]` で全体を gate
（`real_input_e2e.rs` 先例 — default `cargo test` では空
crate として compile）。`#[test] #[ignore]` の
`virtual_endpoint_render_capture_round_trip` 1 本。

流れ（全て control thread、production port API のみ）:

1. **watchdog**: `MAX_RUN`（~45s）で `process::exit(3)` — 手動
   lane が hang しないための安全弁（`real_input_e2e.rs` 先例）。
2. **列挙**: `default_host()` → `output_devices()` /
   `input_devices()`。`Err`/空は `verdict: NotRun` で return。
   各 device の `name()`/`id()`/`default_*_format()`/
   `supported_*_formats()` を記録（format 交渉の観測証拠 —
   `negotiate_*_format` 相当の helper で F32-openable format を
   選定。`real_input_e2e.rs` の `select_f32_*`/`layout_for_channels`
   と同規則を本ファイルに実装）。
3. **pair candidate 選定**: `is_virtual_name(name)`（probe と同一
   `VIRTUAL_NAME_HINTS` — example は test から import できない
   ため複製し、同期義務をコメントで明記）で virtual flag を立て、
   render 側と capture 側で方向語を除いた正規化 stem
   （lowercase・非英数字を空白化・`{input,output,render,capture,
   in,out}` の standalone token を除去・空白正規化）が一致する
   組を pair candidate とする。0 件 → `verdict: NotRun —
   NOT_PROVISIONED`。複数 pair は全て列挙し先頭から順に試行。
4. **open**: pair ごとに input stream → output stream の順で
   `open_*_stream`（`BufferSize::Fixed(480)` 優先、
   `UnsupportedConfig` なら `Default` retry —
   `open_input_with_fallback` 先例）。open 失敗は verdict 付き
   panic（Fail — 実 endpoint があるのに開けないのは契約違反）。
   交渉 format・`stream.format()`・`clock_domain()`・
   `buffer_size()`・`now()`・`input_latency()`/
   `output_latency()` を記録。
5. **render**: output data callback が `signal_sample(frame,
   rate)`（独立 deterministic generator — 後述）で
   `AudioBufferViewMut` を埋める。callback 内は cursor 加算と
   atomic 統計のみ（RT 契約）。`start()` 後 `CAPTURE_SECONDS`
   （~2.5s）待機。
6. **capture**: input data callback が観測統計（callback 数・
   frames・timestamp 単調性・domain 一致・capture≤callback・
   frame_position 連続性・非有限 sample・peak）を atomic で記録
   し、PCM 本体を `Mutex<CaptureBuf>`（try_lock・pre-allocated
   `Vec<Vec<f32>>` channel-major・capacity 超過は overflow
   counter）へ bounded copy。
7. **stop/lifecycle**: 両 stream `stop()` → `buffer_size()`/
   `now()`/`format()`/`clock_domain()` が応答することを確認 →
   `start()`→短時間→`stop()` の再起動確認 → drop。
   `StreamInvalidated` は removal 経路のみのため NotRun 記録
   （AC5/後続）。
8. **照合（独立 oracle、control thread）**:
   - **marker 検出**: capture ch0 を走査し `|x| > 0.5` の peak を
     refractory 距離（~20ms 相当）で検出。≥3 個あれば隣接間隔の
     平均を capture 秒に換算し、期待 marker 周期
     （`MARKER_PERIOD_SECONDS = 0.1`）と ±10% で比較。
   - **level**: capture RMS / 期待 RMS（generator を out_rate で
     2 周期分数値積算した独立値）を [0.2, 3.0] で比較
     （provisional — 実測後に締める）。
   - **基音スペクトル**: Goertzel（440Hz）で capture 中域の band
     power / 全 power を算出し ≥ 0.2 を要求（provisional）。
   - **callback 契約**: input/output とも callback>0・frames>0・
     channel 数一致・timestamp 非減少・`callback - capture ≥ 0`・
     `info.clock_domain == stream.clock_domain()`・非有限 0・
     （WASAPI 時）`frame_position` ギャップ ≤ `ClockDiscontinuity`
     数。
   - error counter（xrun 等）は記録のみ — virtual cable の xrun は
     AC5 の領域であり本 verifier の Pass/Fail 条件にしない。
9. **verdict**: pair の device 名/id・交渉 format・callback/
   frame 統計・marker/level/spectral の実測値・error 集計・
   lifecycle 結果を eprintln で出力（`evidence/rf629/` 転記可能な
   形）。全 hard assert 通過で `verdict: Pass — …`、契約違反は
   verdict 付き panic（Fail）、pair/環境不在は `verdict: NotRun`。
   Pass は OS-L1「実 OS API 往復」証拠であり
   `virtual-os-endpoint` を主張しない旨を明記。

### 試験信号と独立 oracle

```text
signal_sample(frame, rate):
  sine  = 0.20 * sin(2π * 440Hz * frame / rate)
  marker= frame % marker_period_frames(rate) == 0 ? 0.70 : 0.0
  out   = sine + marker          // |peak| ≤ 0.90 — クリップなし
```

- `marker_period_frames(rate) = round(0.1s * rate)` — render 側
  sample rate で 100ms 毎に 1 frame の impulse。capture 側は
  時間軸（秒）で比較するため SRC/format 差を吸収する。
- 期待値は全て本ファイル内の純粋関数から生成 — adapter
  （`ruviform-audio-device`/`ruviform-audio-core`）は期待値を
  一切生成しない（自己参照 Oracle 禁止）。
- channel は全 ch に同一信号（v1 は channel 分離を要求しない —
  各 channel が独立に signal を示すことを確認）。

### `endpoint_probe.rs` 拡張

`#[cfg(windows)] mod win` 内に追加（非 Windows は従来どおり
enumeration 部分のみ NotRun — probe 自体は compile 通過）:

- **pair candidate 表**: `EndpointRow` 列挙後、virtual
  candidate の render 行と capture 行を方向語除去の正規化 stem
  で突合し `== virtual pair candidates ==` 節を出力
  （verifier の選定規則と同一 — probe が「どの pair が試行
  されるか」を事前に見せる）。
- **driver 署名（best-effort）**: `powershell.exe -NoProfile
  -NonInteractive -Command` で virtual audio driver 候補
  （VB-CABLE/VAC/VoiceMeeter 系）の `Win32_SystemDriver` /
  `Get-AuthenticodeSignature` を照会。発見できなければ
  `NotRun — <reason>`。SetupAPI/driver-store の windows feature
  は無いためこの迂回路を取る（discovery gap #4 の記録どおり）。
- **権限（best-effort）**: microphone privacy
  （`HKCU\...\DeviceAccess\Global\{E5323777-…}` の `Value`）を
  PowerShell `Get-ItemProperty` で照会し Allow/Deny/未設定を
  記録。capture が権限拒否される環境の診断材料。
- **cleanup**: verdict に「probe は query-only —
  `IAudioClient::Initialize`/stream open/registry 変更を行わず
  永続状態を作らない」旨を明記（AC4 の cleanup 記録要件）。
- **verdict 更新**: 「open/stream proof は next slice」系の記述を
  「streaming verifier は `tests/virtual_endpoint.rs` に存在
  （`real-device` feature + `#[ignore)`、pair 未 provision では
  NotRun）— `virtual-os-endpoint` evidence は依然主張しない」
  に更新。provisioning runbook 文言を pair candidate 表・driver
  署名・権限記録と整合させる。

### Data and Control Flow

```
test (control thread)
  ├─ default_host() → output_devices()/input_devices()
  ├─ is_virtual_name + stem 正規化 → pair candidates
  ├─ pair ごと: open_input_stream → open_output_stream
  │     （Fixed(480) → UnsupportedConfig なら Default retry）
  ├─ start() both → CAPTURE_SECONDS sleep
  │     output cb: signal_sample(cursor) → AudioBufferViewMut
  │     input  cb: stats(atomic) + PCM → Mutex<CaptureBuf>(try_lock)
  ├─ stop() → post-stop queries → start()/stop() restart → drop
  ├─ oracle: marker 検出/間隔・RMS 比・Goertzel band 比・
  │     callback 契約 assert（control thread、allocation 自由）
  └─ eprintln verdict → evidence/rf629/ 転記
```

probe 側は `main` → `print_environment` → `win::run()`:
`enumerate_all` → `print_port_view` → `print_pair_candidates` →
`print_driver_signature`/`print_capture_permission`（best-effort）
→ `print_verdict`。

## Non-Functional Constraints

- **fail-closed**: host 不在・pair 0 件・非 Windows・format 交渉
  不能は全て NotRun early return。Pass を出力する経路は pair
  open→stream→oracle 照合の全通過のみ。`virtual-os-endpoint` を
  主張しない。
- **RT 契約**: 両 data callback は allocation-free・bounded。
  capture は try_lock + pre-allocated buffer への bounded copy +
  overflow counter（block しない・silent loss しない）。
- **3OS lane**: verifier は cfg 不要（port trait のみ）。probe の
  Win32/PowerShell/registry は `#[cfg(windows)]` module 内。
- **lint**: workspace clippy（pedantic + `-D warnings`）。
  `unsafe` 追加なし。
- **依存**: 新規依存・feature 追加なし。`Cargo.toml`/
  `Cargo.lock` 変更不要（dev-dep 不承認を spec で確定済み）。
  `mise.toml` は 5th slice で opt-in lane task の additive 追加
  のみ（依存・feature の変更ではない）。
- **lane**: `test-audio-real-input` は `--test real_input_e2e`
  固定で本 test を収集しない — canonical 実行は専用 opt-in lane
  `mise run test-audio-virtual-endpoint`（5th slice で登録済み。
  設計は本 design の Verification Notes「AC6 lane 設計記録」参照）。

## Risks and Trade-offs

| Risk | 影響 | 対応 |
| --- | --- | --- |
| name/stem match が偽 pair を拾う | signal が観測されず Fail に見える | pair を全試行し観測値を全記録 — 往復しない pair は正直な Fail だが、verdict に全 candidate を列挙して人間判別を可能にする。stem は方向語除去のみで過剰正規化しない |
| cable SRC/format 差で marker が検出不能 | Fail | marker は時間軸比較・refractory は capture rate 比例・閾値 0.5（render amp 0.7 — SRC 減衰を見込んでも検出可能な余裕）。検出数・実測値を verdict に残し provisioning 実測で閾値を締める |
| level/spectral 閾値が実 cable 特性と乖離 | 誤 Fail/誤 Pass | 閾値は provisional（広め）と明記し実測後に締める。実測値は全て verdict に記録 |
| `Mutex::try_lock` が頻繁に失敗 | capture 欠損 | overflow counter で観測可能化。連続失敗時は marker 数減で Fail — silent loss にしない |
| virtual endpoint が存在するが物理 device と name 衝突 | 誤 candidate | `is_virtual_name` は name heuristic であり確定ではない — verdict に「candidate (unconfirmed)」を明記（probe 先例） |
| PowerShell/registry 照会が環境で失敗 | AC4 記録欠落 | best-effort `NotRun` note — probe を落とさない（WMS 検出先例） |

## Deferred Design Concerns

- 実 provisioning（VB-CABLE/VAC）と `virtual-os-endpoint`
  evidence — 後続 slice。再開条件: 専用 runner での provisioning
  確立。
- #628 `PcmFixture`/`oracle`/`RunMetadata` との実結線 —
  `ruviform-audio-test` dev-dep 承認が前提（本 slice は coordinator
  不承認で見送り）。再開条件: 依存承認 + lane 設計確定。
- AC6 lane 登録本体のうち mise task 登録（`test-audio-virtual-
  endpoint`）は 5th slice で解消済み — 専用 lint lane は追加せず
  既存 `lint-audio-real-input`（`--all-targets --features
  real-device`）が本 test target をカバーする（Coordinator 確定）。
  `.github/**` CI/runner 統合（定期 opt-in 実行）は引き続き後続の
  外部 gate として残る。
- reconnect/xrun/permission 実測（AC5）・`StreamInvalidated`
  の interactive unplug 確認・consumer 照合（AC7）— 後続。
- pair 複数時の選択ポリシー（現状: 全列挙 + 先頭から順次試行）—
  provisioning 実測後に必要なら env var/引数化を検討。
- marker/level/spectral 閾値の実測による締め直し — provisioned
  runner 初回実走の結果待ち。

## Verification Notes

- [ ] `mise run fmt-check` / `mise run check
  ruviform-audio-device` / `mise run lint
  ruviform-audio-device` / `mise run test
  ruviform-audio-device` — Pass（verifier は `#[ignore]` のため
  default test では compile のみ）。
- [ ] `mise run lint-audio-real-input`（`--all-targets --features
  real-device` の `-D warnings`）— Pass（新規 test target は
  `real-device` feature 配下）。
- [ ] `mise exec -- prek run --all-files` — Pass。
- [ ] Windows 実機 `endpoint_probe` 再実行 exit 0 + evidence 更新。
- [ ] 補助証拠: `cargo test -p ruviform-audio-device --features
  real-device --test virtual_endpoint -- --ignored
  --test-threads=1` を本ホストで実行 — pair 未 provision のため
  `verdict: NotRun` を記録（Pass を主張しない）。
- [ ] MSRV lane — NotRun（daily CI 管轄）。

### AC6 lane 設計記録（mise task は 5th slice で登録済み / `.github/**` は後続）

- `test-audio-virtual-endpoint`: `cargo test -p
  ruviform-audio-device --features real-device --test
  virtual_endpoint -- --ignored --test-threads=1` を実行する
  opt-in manual task（`test-audio-real-input` 先例 — description
  に「provisioned virtual endpoint 必須・manual lane・never CI」
  を明記）。→ **5th slice（`issue-629-virtual-endpoint-lane`）で
  `mise.toml` に登録済み**。
- `lint-audio-virtual-endpoint`（または既存
  `lint-audio-real-input` の対象拡張）: `cargo clippy -p
  ruviform-audio-device --all-targets --features real-device --
  -D warnings` — 新規 test target は既に `real-device` feature
  配下のため既存 lint lane でカバー済み。専用 lane を増やすかは
  Coordinator 判断。→ **5th slice で「専用 lane は追加しない」
  と確定**（既存 `lint-audio-real-input` が coverage）。
- CI/runner 統合（`.github/**`・provisioned runner での定期
  実行）は後続 slice — POLICIES §5「Windows専用の仮想
  Endpoint/実Window/実ABI Test Pluginの手動・opt-in・定期実行
  レーン」に対応。
