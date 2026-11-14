---
id: "TSPEC-034XzvJurlMbzQjoWxrwvu"
type: "test-spec"
status: "proposed"
title: "GPUI Native First採用Gate verdict評価検証（Issue #262）"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/262"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-262-gpui-verdict.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-262-gpui-verdict.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-262-gpui-verdict.md"
    - "docs/designs/issue-262-gpui-verdict.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-262-gpui-verdict.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-262-gpui-verdict.md"
    - "docs/designs/issue-262-gpui-verdict.md"
  verified-by: []
  references:
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/report.md"
  defers: []
  deferred-by: []
---

# GPUI Native First採用Gate verdict評価検証（Issue #262）

## Purpose

Issue #262 の採用 Gate 評価スライスが、生み出す verdict 記録（verdict-matrix・report・handoff・verification）が満たすべき不変条件を固定する。本 test-spec は実行可能テストではなく、verdict 記録の**正しさの契約**を定義する。実装（文書・証拠）の正しさは本 spec の Rule/Example に照らして監査し、CI/実機の Pass 主張は別証拠で担保する。verdict-matrix が後続 Issue（#419/#465/#532/#388/#389）の判定材料として再利用される限り有効であり、#419 決裁後も証拠の解釈規則として残る。

## Feature

**GPUI Native First 採用 Gate verdict の証拠記録**

A〜E Gate の各 Case について、対象 Repo/SHA・OS/Arch・feature/capability・expected/observed・実行環境・証拠 URL を伴う verdict（Pass/Fail/NotRun/Unsupported）を能力別に記録し、要求状態（Required/Deferred/NotApplicable）と実行結果を分離して、推奨 verdict の根拠表として #419/#465 へ引き渡す。

## Rules

### R-EVIDENCE: 証拠の同一性

- **R-EVIDENCE-01**: 各 Case は対象 SHA（CI 証拠は checkout artifact 実測 SHA、ローカル証拠は worktree HEAD、本 slice の再実走は `7aab630207e96863c31203629774364df87aa543`）を記録しなければならない。ancestor SHA の証拠は「ancestor SHA の証拠」と明記し、最新 SHA の Pass に読み替えてはならない。
- **R-EVIDENCE-02**: CI run 証拠は run URL・対象 SHA・job 一覧を記録し、lane 定義の存在や job green だけを個別 Case の Pass としてはならない。個別 test の成功を run log で確認した場合のみ Pass とする。
- **R-EVIDENCE-03**: NotRun/Unsupported/Fail の Case は理由（実機不在・CI 未実行・Feature 無効・宣言済み非対応・実測不一致）を必須とする。理由なしの verdict は記録として無効。

### R-VERDICT: 判定語彙の分離

- **R-VERDICT-01**: 要求状態 `Required`/`Deferred`/`NotApplicable` と実行結果 `Pass`/`Fail`/`NotRun`/`Unsupported` は別軸で管理し、Deferred を Pass としてはならない。
- **R-VERDICT-02**: `Unsupported` は実装/環境が明示的に非対応を宣言した場合のみ使用する（例: `ime: Unsupported`、macOS/Linux `native_surface_slot` backend 未実装）。未実験・未測定は `NotRun` であり `Unsupported` にしてはならない。
- **R-VERDICT-03**: `NotRun` は実行結果軸の値であり、Deferred 要求の NotRun は Pass 扱いしない。POLICIES.md §3 に整合する。
- **R-VERDICT-04**: verdict matrix は Case ごとに requirement（Required/Deferred/NotApplicable）と result（Pass/Fail/NotRun/Unsupported）を分離した列で記録しなければならない。

### R-GATE-A: L1 契約基盤

- **R-GATE-A-01**: 公開 UI 契約の Driver-neutral 性・Fake/GPUI semantic fixture・依存方向ガードは、`check-ui-driver-boundary`/`test-ui-driver-contract`/`test-ui-parity`/`test-ui-scenario-contract`/`test-ui-native-slot`/`test-ui-component-view`/`test-ui-gpui`/`test-rt` の lane 証拠で評価する。3 OS の CI run とローカル再実走の両方を照合する。
- **R-GATE-A-02**: 外部 gpui 型・OS handle・Toolkit API が公開契約/Consumer 通常コードに漏れていないことを `gui-boundary-guard` の verdict で確認する。境界漏出があれば Gate A は Fail。
- **R-GATE-A-03**: `gpui`/`vst3` 等の opt-in Feature の compile/link は標準 Full CI で検証されないため、`ui-gpui-verify` lane（`check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`、`windowed_test_app` bin link 含む）の証拠を別記録する。

### R-GATE-B: Windows L2 実 GUI 能力

- **R-GATE-B-01**: Windows の実 Window/Input/Focus 等の使用能力 L2 は、OS 実入力経路（SendInput 等）で intent 到達を実測した証拠のみ Pass とする。headless/offscreen の成功は L2 Pass にしない。
- **R-GATE-B-02**: IME・DPI 動的変化・a11y・物理入力等の能力は、製品が利用する場合のみ Required 化し、利用しない能力は NotApplicable/Deferred として総当りを必須にしない。
- **R-GATE-B-03**: `focus` release が `Err(Unsupported)` を返す等、driver が正直に Unsupported を返す能力は `Unsupported`（宣言済み）として記録し、Fail にしない。Fail は「期待と観測が不一致」の場合のみ。
- **R-GATE-B-04**: macOS/Linux の Window/Input/Focus 等 L2/L3 は Deferred であり、NotRun を Pass にしない。

### R-GATE-C: Component/Viz 正しさ・性能

- **R-GATE-C-01**: Component/Viz の semantic 正しさは `test-ui-parity`/`test-ui-component-view`/`test-ui-scenario-contract`/`test-ui-golden` の headless 証拠で評価する。visual/pixel は headless RGBA8 normalization または Xvfb+lavapipe 等の限定環境証拠を区別して記録する。
- **R-GATE-C-02**: 性能次元（FPS/frame-time/CPU/GPU/memory/latency vs 同条件 baseline）は定量比較証拠がない限り `NotRun` とする。性能予算 Issue が未確定のまま「性能問題なし」と記録してはならない。

### R-GATE-D: 実 Native Plugin Dock（EXT-L3）

- **R-GATE-D-01**: 実 Native Dock は HWND/NSView/X11/Wayland の実 native surface が plugin editor の画素を含めて GPUI window 内に表示されることを実測した場合のみ Pass とする。HWND 機構（parentage/geometry/clip/focus/key/redock）の Pass は機構層の証拠であり、Dock PASS（画素表示を含む完了条件全体）に昇格してはならない。
- **R-GATE-D-02**: Floating Window 重ね・Generic Parameter Editor・Fake/Mock Surface・headless/offscreen 成功を実 Native Dock の代替証拠としてはならない。
- **R-GATE-D-03**: GPUI DComp `topmost=true` による child HWND 非合成が未解決（Issue #532 open）の間、Windows Native Dock 画素表示は `Fail`（実測で非表示）+ 要件上の未受入として記録し、「条件付き/未受入」を明記する。#532 が open Blocker であることを併記する。
- **R-GATE-D-04**: Plugin 形式（VST3/CLAP/AU）と process isolation（L0 in-process / L1 cross-process）を format×OS×L0/L1 の別 Case として記録する。検証済みは Vinyl VST3 1.12.0 L0 のみであり、CLAP/AU・L1 cross-process・他社 plugin 広範互換は `Unsupported`/`NotRun` として別記録する。
- **R-GATE-D-05**: macOS/Linux の native surface backend は `BACKEND_AVAILABLE = cfg!(target_os = "windows")` により Windows のみ実装であり、macOS/Linux は `Unsupported`（宣言済み backend 欠落）+ L2/L3 `NotRun` として区別する。

### R-GATE-E: Consumer 独立 Oracle

- **R-GATE-E-01**: Synvelis（#176）・AudioLens（#166）の製品 GUI 受入は各 Consumer の独立 Issue/受入が Owner であり、本 slice の verdict は `NotRun`/`Deferred` とする。rf465 inventory の L1 互換記録を製品 Oracle の Pass に読み替えてはならない。
- **R-GATE-E-02**: `consumer-verify.yml` は workflow_dispatch 専用であり、対象 SHA の run が存在しない限り Gate E は `NotRun` とする。別 SHA の run を対象 SHA の証拠にしてはならない。

### R-HANDOFF: 引き渡し境界

- **R-HANDOFF-01**: #419 向け引き渡しは「L1 のみ評価済み、L2/L3 非保証」を明示し、Gate D 未達による「Native Dock 無条件本番採用不可」を条件付き判定材料として記録する。
- **R-HANDOFF-02**: #465 への境界として、Legacy 撤去は L1 機能/既存 Consumer 互換（rf465 inventory の分類）と未実 GUI 製品機能の破壊を独立判断する旨を明記する。L1 採用だけで製品 GUI 機能や実 Plugin 表示を保証しない。
- **R-HANDOFF-03**: verdict の決裁宣言（accepted/conditional/rejected）・Issue コメント投稿・Close 判定・ADR status 変更は本 slice の成果物に含めない。推奨 verdict と根拠表のみを記録する。

## Examples

### EX-1: Gate D Windows VST3 L0 の機構層と画素層の分離

- **Given**: `evidence/rf388-native-dock-windowing/report.md` が 33 checks（attach/clip/focus/key/redock）Pass を記録し、同時に GPUI DComp `topmost=true` で plugin 画素が非表示（`Unsupported（実測）`）を特定した。
- **When**: verdict-matrix で Case を分離する。
- **Then**: HWND 機構 Case は `Pass`（機構層）、native surface 画素 Case は `Fail`（実測で非表示）+ #532 Blocker 併記、Dock PASS Case は未達として記録される。機構 Pass が Dock Pass に昇格しない。

### EX-2: macOS/Linux `native_surface_slot` の Unsupported と NotRun の区別

- **Given**: `WindowCapabilities.native_surface_slot` は `BACKEND_AVAILABLE = cfg!(target_os = "windows")` により Windows のみ `Supported` を返し、macOS/Linux は `Unsupported` を宣言する。
- **When**: verdict-matrix の macOS/Linux 行を記録する。
- **Then**: backend 未実装は `Unsupported`（宣言済み技術的非対応 = Capability 欠落として追跡）、実 GUI L2/L3 は `NotRun`/`Deferred`（利用要件発生まで）として別々に記録され、NotRun を Pass にしない。

### EX-3: CI Fail の正直な記録

- **Given**: CI run 43540652567（SHA `2613b238`）の `verify (ubuntu-latest)` で `e2e_disable_env_kills_active_transport` が FAILED したが、修正 PR #626 が HEAD `7aab6302` にマージ済み。
- **When**: verdict-matrix の Gate A/CI 証拠行を記録する。
- **Then**: 当該 run の Fail をそのまま記録し（緑に塗り替えない）、修正が HEAD に入っていることと HEAD SHA の CI run 未実施を併記する。「最新 main の統合受入は merge 後対象 SHA の CI 証拠待ち」と記録する。

### EX-4: ancestor SHA 証拠の取り扱い

- **Given**: 先行 PoC 証拠（rf388-*/rf389-*/rf313/rf243）はそれぞれの実行 SHA が HEAD の ancestor であり、HEAD に対する CI run は未実施。
- **When**: verdict-matrix で各 Case の証拠 SHA を記録する。
- **Then**: ancestor SHA の証拠は「ancestor SHA の証拠」と明記し、`git merge-base` で ancestor 関係を確認した上で差分が契約層に影響しないことを確認する。HEAD SHA の Pass には読み替えない。

## Guarantees

- verdict-matrix の各 Case は CaseID・OS/Arch・Repo/SHA・feature/capability・expected・observed・requirement・result・証拠 URL/実機ログを持つ。
- Pass/Fail/NotRun/Unsupported は実測または宣言済み Capability に基づき、推測で付与しない。
- 推奨 verdict は「GPUI 標準 Driver 方針維持・OS-L1 条件付き採用可・Windows L2 能力別・Gate D 未達・macOS/Linux L2/L3 Deferred・Gate C 性能 NotRun・Gate E NotRun」の構造を持ち、決裁宣言を含まない。
- #419/#465 引き渡し文面は「L1 のみ評価済み、L2/L3 非保証」と独立判断境界を明示する。

## Non-goals

- 実 Native Dock・実 GUI・実 Plugin・Consumer 製品受入の新規検証（本 slice は既存証拠の照合と正規 lane 再実走のみ）。
- verdict の最終決裁、Issue 投稿、ADR status 変更、#532/#388/#389/#465 の実装・撤去。
- 性能測定（FPS/frame-time/CPU/GPU/memory/latency）の実施。
- CI workflow の新規作成・変更。

## Risk

verdict 記録の不変条件が破られると、#419 の ADR 決裁が「L1 のみ評価済み」の範囲を超えて L2/L3 を保証したと誤認される。特に Gate D（実 Native Dock）と Gate E（Consumer Oracle）の未達を Pass に読み替える誤りは、製品 GUI 機能や実 Plugin 表示の不存在を隠蔽する。R-VERDICT/R-GATE-D/R-GATE-E/R-HANDOFF の保証が落ちると、Issue #262 の Verdict 規則（「全必須 OS/format で D・E が未達なら無条件本番採用不可」）が形骸化する。
