---
id: "SPEC-kT8mQ2wX9pL4nR7vC1zB5e"
type: "spec"
status: "approved"
title: "Issue #306 IO E2E — OS-L1 既存 CI 証拠検査と不足 Case 補完（Slice V+T）"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "file"
        path: "evidence/rf306/ci-os-l1-report.md"
      - type: "file"
        path: "docs/specs/issue-306-io-e2e-cases.md"
      - type: "file"
        path: "docs/designs/issue-306-io-e2e-cases.md"
      - type: "file"
        path: "docs/plans/issue-306-io-e2e-cases.md"
      - type: "file"
        path: "docs/tasks/issue-306-io-e2e-cases.md"
      - type: "file"
        path: "docs/test-specs/issue-306-io-e2e-cases.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-io-e2e-cases.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/staged_lifecycle.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-306-io-composition-e2e.md"
    - "docs/specs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/specs/issue-306-io-msrv-dag-rt-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
  refines:
    - "docs/specs/issue-306-io-composition-e2e.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-io-composition-e2e.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-io-e2e-cases.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 IO E2E — OS-L1 既存 CI 証拠検査と不足 Case 補完（Slice V+T）

## What Should Be Built

Issue #306（IO-IMP-12 共通 I/O 合成 E2E）の証拠 slice。
対象 SHA `e93c438adc4af5b498e10926fe859ac1338ac390` の既存
Full CI run の証拠を検査して `evidence/rf306/` に記録し、
通常 L1 で不足する staged publish の外部干渉・recovery
retry Case を `ruviform-io-e2e` へ補完する。production
code・公開 fault seam・CI 定義は変更しない。

追加するもの:

1. **Slice V（既存 CI 証拠検査）**: run `43428172820` の
   job 結果だけでなく job 生ログを解析し、io 系 crate
   （io-e2e/io-file/io-archive/io-structured/io-compression/
   io-wav/io-symphonia/media-io）の Native Compile/Link・
   Headless Test が 3 OS の verify lane と feature/MSRV/RT
   lane で実際に実行されたことを SHA・Run URL・job 単位で
   記録する。`deflate` が io-e2e 非消費で ubuntu lane のみ
   実走である差異を正直に分離する。
2. **Slice T（Case 補完）**: `staged_lifecycle.rs` に
   seal→publish 間の外部 temp 改竄・削除、および
   RecoveryHandle cleanup 失敗→retry の e2e を追加する。
   実ファイルシステムで再現可能な Case のみを対象とし、
   公開 API で到達不能な fault class は mock 正本との
   責務分離を維持する。
3. **判定材料の記録**: RT Fake Fault 解釈（io-e2e は非 RT
   で rt_ 契約 test 不要）と G07 public fault seam の
   到達不能性を evidence に整理する（実装・公開判定は
   別 slice）。

## Why Now

Issue #306 受入条件の「既存 CI での OS-L1 実走検査」と
「通常 L1 で不足する Case の補完」は headless で誠実に
実行可能であり、consumer 受入（#219/#294）や Windows
L2/L3 判定の前提となる L1 証拠を固める段階である。

## Users and Value

- Issue #306 受入監査: CI 定義の green と個別 Case の
  実走を区別した証拠を `evidence/rf306/` に集約する。
- 後続 consumer 受入: OS-L1 の Pass/NotRun/Deferred が
  Issue AC 語彙で分離された判定材料を提供する。

## Scope

### In Scope

- `evidence/rf306/ci-os-l1-report.md` の新規作成
  （run/job/SHA/feature/結果の記録と NotRun 分類）。
- `crates/ruviform-io-e2e/tests/**` への L1 Case 追加と
  `crates/ruviform-io-e2e/README.md` の検証節更新。
- 本 doc チェーン 6 文書と managed docs index の再生成。
- `mise run fmt-check` / `check` / `lint` / `test
  ruviform-io-e2e` のローカル検証。

### Out of Scope

- `ruviform-io-e2e` 以外の crate（特に `ruviform-io-file`
  の G07 seam 公開/改変）、`.github/`、`mise.toml`、
  `ci/`、`POLICIES.md`、`evidence/rf288/` の変更。
- G07 public fault seam の実装・公開判定そのもの、
  Indeterminate/TempState::Unknown の公開 API 再現 fixture。
- 実 4 GiB・macOS/Linux 実機・Network/Cloud FS・持続
  Fuzz・性能比較・#294/#288 cross 集約・Windows L2/L3
  実 FS 特殊条件（全て Deferred/外部）。

## Requirements

- CI 証拠は job conclusion ではなく job 生ログの test 実行
  行で裏付け、SHA・Run URL・OS・toolchain・feature・
  command・結果を記録する。
- 追加 test は公開 API と実ファイルシステムのみで構成し、
  `#[allow]`・検証弱化・test 無効化を行わない。
- Required/Deferred/NotApplicable と Pass/Fail/NotRun/
  Unsupported を POLICIES 語彙で分離し、未実施を Pass
  と記録しない。
- post-seal temp 改竄が publish で再検証されない観測は
  現行契約の正直な記録とし、io-file の設計変更は行わない。

## Acceptance Criteria

- [ ] `e93c438a` の CI run について io 系 crate の
      Native Compile/Link・Headless Test 実走が SHA・
      Run URL・job 単位で `evidence/rf306/` に記録され、
      `deflate` の lane 差異が分離されている。
- [ ] seal→publish 外部 temp 改竄/削除・RecoveryHandle
      cleanup 失敗→retry の Case が `ruviform-io-e2e`
      tests に追加され `mise run test ruviform-io-e2e`
      で全件実走 Pass。
- [ ] macOS/Linux L1 は既存 CI 証拠付きで検証され、
      GUI/物理 Device/実 Plugin は Deferred/NotRun の
      まま Pass にしていない。
- [ ] Windows で使う機能の必要 L2/L3 が Fail/NotRun の
      場合に当該機能を未受入とし、未使用 Capability を
      必須化していない。
- [ ] 本 doc チェーン・managed index が再生成され、
      `review-protocol` で blocker なし。

## Deferred Design Concerns

- post-seal temp 改竄の publish 時再検証 | io-file の
  integrity scope 設計判断 | 別 slice での契約検討 |
  現行は seal 時点保証として記録。
- G07 public fault seam 公開（Option A/B） | 設計判断 |
  別 slice | Indeterminate 系の e2e 化は seam 公開後。
- 実 4 GiB・Network/Cloud FS・実機 macOS/Linux |
  headless 不可 | 実機/環境 dispatch | 容量・環境依存の
  未検証が残る。

## Sources and Relations

- Issue #306 本文（受入条件の正本）・Issue #288 IO-06・
  Issue #282 横断設計正本。
- 前 slice 文書チェーン `docs/{specs,designs,plans,tasks,
  test-specs,impl/ir}/issue-306-*.md`・既存証跡
  `evidence/rf306/{report,fault-fuzz-report,
  feature-matrix-report}.md`。
- `POLICIES.md` OS-L1/GUI-L2/EXT-L3・Required/Deferred/
  NotApplicable 規範。
