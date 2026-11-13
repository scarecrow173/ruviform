---
id: "IMPL-034WTILJv8M98cnqJJ8cqJ"
type: "impl"
status: "completed"
title: "Issue 251 audio input OS L1 evidence"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/251"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-251-audio-input-os-l1-evidence.md"
      - type: "file"
        path: "evidence/rf251-audio-input/report.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-251-audio-device-real-backend-input-plumbing-headless-slice.md"
    - "docs/impl/ir/issue-251-stage-w-real-device-closeout-slice.md"
    - "docs/impl/ir/issue-251-stage-w-output-latency.md"
    - "docs/impl/ir/issue-388-os-l1-ci-evidence.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 251 audio input OS L1 evidence

## Summary

Issue #251 の OS-L1 証拠照合 slice として、対象 SHA `b9eaf4880681734b2d65dbf200ca8c49c6e4c81e`（main HEAD・PR #602 merge）に対する既存 CI 証拠を `gh` で照合し、`crates/ruviform-audio-device` 周辺の backend 実装をコード精読で確認した上で、OS×Backend×Device×Case の判定行列を `evidence/rf251-audio-input/report.md` に記録した。対象 SHA の completed CI run は存在しない（`ci.yml` は workflow_dispatch 専用）ため OS-L1 統合検証は **NotRun（Pending）** と正直に記録し、直近 completed run `43428172820`（SHA `e93c438a`、20/20 job success・ci-summary verdict=PASS）を「audio 関連 crates 同一コード」の補助証拠として SHA 差分とともに分離した。PR #568/#581 の Windows/WASAPI/Audient iD4 実機証拠（opt-in lane・CI 非含有）は過去証拠として引用し、macOS/Linux 実機・物理 USB 抜去・実 OS 権限拒否・xrun 誘発は NotRun/Deferred として Pass に繰り上げなかった。コード変更はなく、文書のみの slice である（文書チェーンは本 IR に集約）。

## Context

Issue #251（audio input の OS-L1 受入・実機/未実装能力の正直な記録）のうち、本 slice が担うのは 4 つの受入条件: (1) 既存 CI の Windows/macOS/Linux で Audio Device 関連コードの Build/Link・Mock Input/Output・Device capability/Error/Timeout/RT の Headless 回帰を対象 SHA で確認（Feature opt-in の実行範囲を明示）、(2) 非 Windows 実 Backend の実装有無と Unsupported 契約を正確に記録、(3) 実測済みと NotRun/Unsupported の OS×Backend×Device×Case 一覧を証拠 URL とともに確定（Synvelis 固有は SV #115/#117 へ委譲）、(4) 物理 USB 抜去/実 OS 権限拒否/xrun・macOS/Linux 実機を Deferred として NotRun を Pass にしない。対象外は macOS/Linux 実機測定・物理 fault 実再現・SV 製品側受入・#315 異 Clock Drift で、Issue は open 継続が前提。検証レベルの規範は `POLICIES.md` §2〜§5 に従う。

## Implementation

### Changes

#### `evidence/rf251-audio-input/report.md`（新規）

- 対象 SHA の CI 状態を冒頭で明記: `b9eaf488`（および直前の `49043603`）に completed run が存在しないことを `gh api .../actions/runs?head_sha=...` の 0 件で確認し、`ci.yml` が workflow_dispatch 専用であること・次回検証機会が daily-ci（cron `17 21 * * *` UTC）であることを記録。
- 採用 run `43428172820`（`e93c438a`）の選定根拠と、`git diff e93c438a..b9eaf488` で `ruviform-audio-device`/`audio-core`/`audio` の差分ゼロを確認した上で「補助証拠・対象 SHA の Pass には繰り上げない」と判定。
- audio-device 関連 job/log 証拠表: `verify`（ubuntu 59+4+12+0+6 tests、windows 75+4+12+0+6 tests — `wasapi_input::tests` 16 件含む）、`verify-macos`（`coreaudio-rs` link 確認・59+4+12+0+6）、`rt-verify`（`rt_` filter の audio-device 該当 tests）、`msrv-verify`（audio-device 非対象であることの記録）、`l1-real-verify`（実 Plugin lane で audio-device 非カバー）を job URL 付きで一覧化。`real_input_e2e.rs` が feature 未設定で 0 tests compile されることを log 上の証拠として記録。
- 非 Windows backend の実装有無と Unsupported 契約: Windows=WASAPI-native input（`frame_position=Some`/`input_latency=Ok(Some)`/`MONOTONIC`/`Carried`）+ cpal output、macOS=CoreAudio cpal portable（`frame_position=None`/`Ok(None)` latency/`MONOTONIC` audited claim）、Linux=ALSA cpal portable（`ClockDomainId::UNKNOWN` → same-clock fallback）+ opt-in `pipewire-backend` を `cfg`・`native_input_route`・port 契約のコード精読で確認。OS-level unsupported gate が存在しないこと、`Unsupported`/`UnsupportedConfig`/`Backend("permission_denied")`/`Backend("host_unavailable")` の typed error 面、port v1 が `open_timeout` を持たない設計判断を記録。
- Feature opt-in の実行範囲表: `default`（CI で compile/test）、`real-device`（CI 非実行・opt-in manual lane のみ・PR #568/#581 が Windows+iD4 で実施）、`realtime`・`pipewire-backend`（CI 未踏・専用 mise task なし）。
- OS×Backend×Device×Case 判定行列: Windows/WASAPI/Audient iD4（実機・過去証拠）・Windows/WASAPI/CI runner（headless）・macOS/CoreAudio/CI runner・Linux/ALSA/PipeWire/CI runner・横断（対象 SHA CI・MSRV・realtime feature・SV 委譲）の各 Case に Pass/Unsupported/NotRun/Deferred/NotApplicable を証拠 URL 付きで付与。NotRun を Pass にしないことを全行で維持。
- 実機証拠の引用節: PR #568（4/4 Pass・capture/latency/domain/format/device-loss 代替 trigger）と PR #581（5/5 Pass・E_ACCESSDENIED parity・output latency 実測値）を merge commit・環境・結果とともに CI headless 証拠と区別して記録。
- 判定まとめ: 対象 SHA CI = NotRun(Pending)、補助証拠の範囲、非 Windows 実機 = NotRun、物理 fault = Deferred、Issue #251 open 継続の根拠（(a) 対象 SHA 必須 CI、(b) macOS/Linux 実機 Stage M/L、(c) 物理 fault の Owner/Trigger 付き Gate、(d) SV #115/#117・#315）。

#### `docs/impl/ir/issue-251-audio-input-os-l1-evidence.md`（本書）

- 実施内容・証拠・判定を IR として記録。

### Decisions During Implementation

- **対象 SHA の追従**: タスク仕様の対象 SHA は `49043603`（PR #601 merge）だったが、作業中に `git fetch origin main` で main が `b9eaf488`（PR #602 merge）へ進行したことを確認。タスク仕様の「それより新しい main 進行があれば最新に合わせて記録に明記」に従い、対象 SHA を `b9eaf488` に更新し、作業 branch `docs/251-audio-input-os-l1-evidence` を `origin/main` 起点で作成した。`49043603`→`b9eaf488` の差分は `crates/ruviform-ui-test` の test コードと docs/evidence のみで audio 関連 crates に差分なし。
- **CI 証拠の正直な格付け**: run `43428172820` は SHA `e93c438a` のものであり、audio 関連 crates が `e93c438a`/`49043603`/`b9eaf488` で同一であることを `git diff --name-only` で確認した上で「補助証拠」として採用したが、POLICIES §5（対象 main SHA の必須 CI 結果が統合検証の証拠）に従い対象 SHA の判定は NotRun(Pending) とした。同一コードだからと Pass に繰り上げない。
- **`l1-real-verify` の非カバー明記**: 名前が「real」を含むが実 Plugin L1 lane であり job comment に「adapter dependency tree contains no cpal」とあるため、audio-device の証拠としては使わず、その旨を report に明記した。
- **Timeout の扱い**: port v1 に `open_timeout` が存在しないのは欠落ではなく `device.rs` の設計判断（backend default に委譲・blocking し得る旨を doc comment で明示）であるため、Case 表では NotApplicable（契約上非存在）+ lane watchdog 30s として記録し、Unsupported/NotRun とは区別した。
- **文書チェーンの集約**: 本 slice は証拠記録が主体のため、spec/design/plan/task の完全チェーンは生成せず IR を正とした（タスク仕様の許容どおり）。実験 log も不要（探索的実装ではない）。
- **branch**: タスク指定の `docs/251-audio-input-os-l1-evidence` を `origin/main`（`b9eaf488`）起点で作成した。Orca worktree 既定 branch `scarecrow173/rf251-evidence` は `49043603` に残るが、push はタスク指定 branch のみ行う。

## Related Experiments

### Adopted

- なし（実験 log は作成していない — 証拠収集と文書作成のみの機械的 slice）。

### Rejected

- なし。

## Validation

実施済み:

- `git fetch origin main && git rev-parse origin/main` — main が `b9eaf488`（PR #602 merge）へ進行していることを確認し、対象 SHA を更新。`git log 49043603..origin/main` で差分 commit を確認。
- `git diff --name-only e93c438a..b9eaf488 -- crates/ruviform-audio-device crates/ruviform-audio-core crates/ruviform-audio` — audio 関連 crates の差分ゼロを確認（補助証拠の採用根拠）。
- `gh api repos/scarecrow173/ruviform/actions/runs?head_sha=b9eaf488…` / `?head_sha=49043603…` — いずれも `total_count=0`（対象 SHA の CI run 不存在を確認）。
- `gh run list --repo scarecrow173/ruviform` / `gh run view 43428172820 --json jobs` — run 選定と job 一覧。
- `gh run view --job <id> --log`（verify ubuntu/windows・verify-macos・rt-verify・msrv-verify・l1-real-verify・ci-summary）— `ruviform-audio-device` の compile/test 実測値（test result 集計・`wasapi_input::tests`・`coreaudio-rs` link・`real_input_e2e` 0 tests・`rt_` filter 該当件数）と checkout-sha 19 leg の一致・verdict=PASS を確認。
- `crates/ruviform-audio-device` の `src/lib.rs`/`src/device.rs`/`src/host.rs`/`src/stream.rs`/`src/wasapi_input.rs`/`Cargo.toml`/`tests/real_input_e2e.rs` の精読 — backend 構造・`cfg` 分岐・typed error 面・feature gate・Unsupported 契約をコードで裏付け。
- `gh issue view 251`・PR #568/#581 の本文/IR（`issue-251-stage-w-*`）— 実機証拠の過去記録を引用元として確認。
- `mise run fmt-check` — 文書のみの slice のため fmt 対象外だが hook 確認として実施。
- `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` — 本 IR の front matter/必須節の audit。
- `node .agents/skills/impl-doc/scripts/sync_impl_index.js --kind ir`（preview → `--apply`）— `docs/impl/ir/README.md` の reconcile。
- review-protocol によるレビュー（cooperative + adversarial の 2 pass をセッション内で分離実施、subagent 不使用 — タスク契約どおり）— Issue #251 の 4 受入条件を評価基準に、Issue Candidate を一次資料（job log・`crates/` コード・`ci.yml`/`daily-ci.yml`/`mise.toml`/`mise.ci.toml`・PR/IR）で検証。CONFIRMED と判定した修正を適用: (a) `bench` job が `bench-smoke`（`-p ruviform-math -p ruviform-rt -p ruviform-viz`）で audio-device を compile しない誤記載の訂正、(b) PR #568 の実機結果を「5/5 系」とした誤帰属を 4/4（#581 が 5/5）に訂正、(c) PulseAudio/JACK を NotRun→Unsupported（adapter が cpal feature を公開しない）に判定修正、(d) endpoint 名の大小文字統一、(e) daily-ci の dispatch 決定契約（有効結果なし時のみ・sha 毎 2 回上限）の正確化。検証過程で ubuntu lib 59 / windows lib 75 / mock 4+12 / real_input_e2e 0 / doctest 6、`wasapi_input` `#[test]` 16 件、cpal 0.18.2 feature 定義、`clock_profile`/`native_input_route` の `cfg` 構造、`#[ignore]` 5 test、PR merge commit・日付、msrv-verify 4 task、`rust-version = "1.89"`/mise.ci pin 1.99.0 を一次資料で再確認した。判定: ACCEPT WITH NON-BLOCKING NOTES（P2 修正適用済み・残件は対象 SHA CI の NotRun(Pending) のみで slice の性質上不可避）。

未実施（意図的）:

- Workspace build/test/lint — 文書のみの変更であり `POLICIES.md` §5 により軽量チェックが原則。cargo 証拠は CI run の実測を引用。
- `*-full`/`verify-full`/`*-msrv`・実機 lane（`test-audio-real-input` 等 opt-in lane）— タスク仕様でローカル必須検証から明示除外。実機 lane は過去証拠の引用対象。

## Risks

- **対象 SHA CI の未確定**: `b9eaf488` の必須 CI は daily-ci dispatch 待ちであり、本 slice 完了時点では統合検証が NotRun のまま。Coordinator が merge 後の対象 SHA CI で最終判定する必要がある。
- **証拠の経時劣化**: run `43428172820` の artifact retention は 30 日。report は job URL・test 集計値・SHA を本文に転記済みのため、artifact 失効後も判定根拠は残る。
- **補助証拠の誤読リスク**: audio crates が 3 SHA で同一であることは確認済みだが、`e93c438a`..`b9eaf488` 間で `ci.yml`（actionlint tool 追加）と `ruviform-ui-test` に差分がある。audio-device の証拠としては影響しないが、report では差分内容を明記して誤解を防いだ。
- **実機証拠の陳腐化**: PR #568/#581 の実機結果は特定環境（Windows 11・Audient iD4・44.1kHz F32）での観測であり、他 device/OS/format への一般化はできない。matrix でも device/format を明記して一般化を防いだ。

## Follow-ups

- Coordinator による Issue #251 の受入判定 — 対象 SHA `b9eaf488` の必須 CI 完了（daily-ci dispatch）を前提に、report の行列を証拠として評価。
- macOS/Linux 実機測定（Stage M/L closeout）— 別 Issue/Owner での実施が必要。実 InputStream・latency・frame_position・duplex・device loss・permission・xrun の実測。
- 物理 fault（USB 抜去・実 OS 権限拒否・xrun 誘発）の Owner/Trigger 付き後続 OS Gate。
- `realtime` feature・`pipewire-backend` feature の CI/手動 lane 整備の要否判断（現状どの lane でも未踏）。
- audio-device の MSRV（rust 1.89）検証 lane の要否判断（現行 `msrv-verify` 対象外）。
- SV #115/#117（Synvelis 固有録音/Frame 相関）・Issue #315（異 Clock Drift）の追跡 — 本 repo の OS-L1 証拠の範囲外。
