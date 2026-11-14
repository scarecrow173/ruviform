---
id: "SPEC-034X9FvLNoXvw7vU7G3fvH"
type: "spec"
status: "proposed"
title: "Linux 配布設計の既存 CI 照合と文書確定（Issue #127）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/127"
    - "docs/specs/issue-127-linux-dist-design.md"
    - "docs/designs/issue-127-linux-dist-design.md"
  changes:
    added:
      - type: "doc"
        path: "evidence/rf127/linux-ci-crosscheck.md"
      - type: "doc"
        path: "docs/specs/linux-distribution.md"
      - type: "doc"
        path: "docs/designs/linux-distribution-design.md"
      - type: "doc"
        path: "docs/test-specs/linux-distribution.md"
      - type: "doc"
        path: "docs/plans/linux-distribution-plan.md"
      - type: "doc"
        path: "docs/tasks/linux-distribution-task.md"
      - type: "doc"
        path: "docs/impl/ir/linux-distribution.md"
      - type: "doc"
        path: "docs/impl/exp/linux-distribution.md"
    modified:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/linux-distribution-design.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-127-linux-dist-design.md"
    - "docs/designs/issue-127-linux-dist-design.md"
    - "docs/test-specs/issue-127-linux-dist-design.md"
    - "docs/plans/issue-127-linux-dist-design.md"
    - "docs/tasks/issue-127-linux-dist-design.md"
  refines:
    - "docs/specs/issue-127-linux-dist-design.md"
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/127"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/linux-distribution.md"
  references:
    - "evidence/rf127/linux-ci-crosscheck.md"
    - "docs/designs/issue-127-linux-dist-design.md"
    - "docs/test-specs/issue-127-linux-dist-design.md"
    - "docs/plans/issue-127-linux-dist-design.md"
    - "docs/tasks/issue-127-linux-dist-design.md"
  defers: []
  deferred-by: []
---

# Linux 配布設計の既存 CI 照合と文書確定（Issue #127）

## What Should Be Built

Issue #127「Linux 共通配布設計」の残件スライスとして、(1) 既存 Ubuntu CI lane の定義と実績 run の照合レジャー `evidence/rf127/linux-ci-crosscheck.md`、(2) 先行設計文書（PR #529/#573 由来の `docs/*/issue-127-linux-dist-design.md` チェーン）を既存 CI 証拠で更新・確定した docs チェーン（`docs/{specs,designs,test-specs,plans,tasks}/linux-distribution*.md` + `docs/impl/{ir,exp}/linux-distribution.md`）を作る。docs と証拠記録のみであり、コード・CI・依存・パッケージ成果物の変更は行わない。

## Why Now

先行設計は GPUI Linux lane（`ui-gpui-verify`/`ui-gpui-offscreen-verify`、PR #566 で追加）の hosted 実行を `NotRun` と記録したまま確定されている。その後 Issue #582 の修正（PR #593）を経て main で全 lane が成功しており（run 43439434737 @ `365c74ac`）、ubuntu-24.04 hosted 上で GPUI の headless 契約テストと Xvfb+openbox+lavapipe の offscreen `real_window`（3 passed）が実測された。Issue #127 の受入条件「既存 CI の対象 revision と成功 Run を照合し、CI 定義済みのみなら NotRun と区別する」を満たすには、この実績を証拠として固定し、文書の `NotRun`/`P(L0)` 記述を実測に合わせて確定する必要がある。

## Users and Value

- Ruviform の共通基盤利用者（AudioLens・Synvelis の製品側設計者）が、Linux 配布の前提条件・能力・制約を「何が実測済みで何が未実測か」を誤認なく参照できる。
- Coordinator / 後続 Issue（#419・#262・#388・#389・各 Consumer の製品 packaging Issue）が、実機・実 Plugin 検証の入力となる capability matrix と Owner 境界を確定版から引ける。

## Scope

### In Scope

- 既存 `ci.yml` / `daily-ci.yml` / `consumer-verify.yml` の Ubuntu lane と、main 上の成功 run の照合（対象 SHA・run URL・job・結果・テスト集計）。
- `Pass` / `Fail` / `Unsupported` / `NotRun` / `Blocked` / `P(L0)` の語彙に基づく能力別判定の確定。
- Ruviform 共通 runtime 最低要件・distro/配布方式の採否/未確証・AL/SV 製品 packaging Owner 境界の文書確定。
- docs チェーン 5 文書 + IR/exp 記録 + 管理 index の再生成。

### Out of Scope

- 実 deb/AppImage/Flatpak の作成・署名・install 検証、実機 Linux GUI/Plugin/Device 検証、CI lane の増設・変更。
- 対象 distro の最終決定、配布形式の最終採用（全形式 `保留` のまま）。
- AudioLens/Synvelis 側の製品 packaging 選定・署名・リリース workflow。
- macOS/Windows leg の受入（別 slice/別 Issue が所有）。
- 旧 WebView/WebKitGTK 依存の物理撤去（Issue #465 が所有）。

## Requirements

- 既存 Ubuntu CI run の成功 SHA・URL・job 名・結果を `gh` で照合し `evidence/rf127/` に台帳化する。CI 定義済みだが未実行のものは `NotRun` として区別する。
- 実環境未測定・`Fail`・`NotRun` を `Pass` にしない。`P(L0)`（Xvfb/openbox/lavapipe offscreen）と headless CI を実機 Linux `Pass` に昇格させない。
- Ruviform 共通 runtime 最低要件・distro/配布方式の採否/未確証・AL/SV 製品 packaging Owner 境界を文書で確定する。実 deb/AppImage/Flatpak 作成や署名は本 Issue に含めない。
- Windows L2/L3（使用機能）は本 Issue で直接 Own しない。macOS/Linux L1（必須）は Native Build/Link・必要 Feature・Headless/RT の既存 CI を同候補 SHA で確認する。実 GUI/実 Device/実 Plugin の L2/L3 は利用時のみ追加する。
- 能力別判定で対象 OS/機能/Device/Format/実行 URL、`NotRequired` と `Deferred` の理由を分け、CI を増設しない。

## Acceptance Criteria

- [ ] Linux の既存 CI（Ubuntu）で Native Build/Link・Headless・Feature・依存・ABI の対象 revision と成功 Run を照合し、CI 定義済みのみなら `NotRun` として区別した台帳が `evidence/rf127/` にある。
- [ ] Ruviform 共通 runtime 最低要件、distro/配布方式の採否・未確証、AL/SV の製品 packaging Owner 境界が文書で確定している。実 deb/AppImage/Flatpak 作成・署名を主張していない。
- [ ] macOS/Linux L1（必須）: Native Build/Link、必要 Feature・Headless/RT の既存 CI を同候補 SHA（`365c74ac`）で確認済み。現行 main HEAD（`621490e8`）の CI は未実施として `NotRun(Pending)` と記録した。
- [ ] 能力別判定で実環境未測定・`Fail`・`NotRun` を `Pass` にしていない。対象 OS/機能/Device/Format/実行 URL と `Deferred`/`NotRequired` の理由が分離されている。
- [ ] docs チェーン（spec→design→test-spec→plan→task）と IR/exp が front matter・relation・index の監査を通過している。

## Deferred Design Concerns

- 実機 Linux GUI（実 display・実入力・実 session）・実 Plugin Dock・実 Device の L2/L3 | 利用要件未発生 | AudioLens/Synvelis または Ruviform 側で実利用要件が立った時点 | 要件発生時に実機 matrix（design §7）で検証しないと capability matrix の `P(L0)`/`NotRun` が製品判断に使えない | #419/#262/#388/#389 の実機 lane と各 Consumer の利用 Issue
- 対象 distro の最終決定 | 実測なしに支持対象を宣言できない | 実機 matrix の実測後 | distro 宣言なき配布 package は Consumer が対象 OS を絞れない | design §1.1 の評価軸を入力に実機検証
- deb/AppImage/Flatpak の最終採用 | 全形式 `保留`。Native Plugin Dock 適合・署名・対象ユーザ層が未確定 | 対象 OS/Backend 決定 + 実機 capability `Pass` + Consumer 製品要件の三点が揃った時点 | 形式未確定のまま製品 packaging を進めると手戻り | design §4.3 の必要証拠
- dlopen 実行時依存の網羅検査 lane | `x11-dl`/`ash` は `ldd` に現れない。CI 定義済み lane なし | 実機/配布 package 検証時に `LD_DEBUG`/`strace` 的手法で確認 | package 化時に実行時依存不足で `Fail` になる可能性 | design §2.5・§7
- 専用 ABI 検査 lane | 現行は domain ABI 契約が `test-full` に内包。専用 lane は未定義 | 必要性が立証された時点（POLICIES §4: CI 増設は要件ベース） | ABI 破壊の検出が test 失敗に依存する | `test-full` 内包の ABI 契約 test |

## Sources and Relations

- Issue #127 本文（受入条件の正本）。先行 spec/design/test-spec/plan/task は `issue-127-linux-dist-design.md` チェーン（PR #529・#573）。
- CI 証拠: `evidence/rf127/linux-ci-crosscheck.md`（run 43439434737 @ `365c74ac`・daily-ci/consumer-verify の最新 run を含む）。
- 上流設計: `docs/designs/issue-127-linux-dist-design.md`、`docs/designs/issue-386-driver-neutral-gui-host.md`、`docs/designs/issue-388-native-surface-slot.md`、`docs/designs/issue-257-driver-neutral-ui-contract.md`。
- 関連 Issue: #57・#28・#262・#294・#313・#314・#384・#386・#388・#389・#419・#465・#582・#594・#598。
- ADR: `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`、`docs/adr/platform-and-toolchain-targets.md`、`docs/adr/licensing-and-distribution-policy.md`。
- Consumer: AudioLens #127、Synvelis #28。
