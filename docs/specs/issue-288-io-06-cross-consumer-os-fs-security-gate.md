---
id: "SPEC-034UtEnxrMMCVpVRENHQ19"
type: "spec"
status: "approved"
title: "Issue #288 IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/plans/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/tasks/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
      - type: "doc"
        path: "docs/test-specs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
    - "docs/specs/issue-306-io-composition-e2e.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/282"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/296"
    - "https://github.com/scarecrow173/ruviform/issues/297"
    - "https://github.com/scarecrow173/ruviform/issues/298"
    - "https://github.com/scarecrow173/ruviform/issues/299"
    - "https://github.com/scarecrow173/ruviform/issues/300"
    - "https://github.com/scarecrow173/ruviform/issues/301"
    - "https://github.com/scarecrow173/ruviform/issues/302"
    - "https://github.com/scarecrow173/ruviform/issues/303"
    - "https://github.com/scarecrow173/ruviform/issues/304"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/audio-lens/issues/139"
    - "https://github.com/scarecrow173/audio-lens/issues/140"
    - "https://github.com/scarecrow173/audio-lens/issues/141"
    - "https://github.com/scarecrow173/audio-lens/issues/143"
    - "https://github.com/scarecrow173/synvelis/issues/82"
    - "https://github.com/scarecrow173/synvelis/issues/84"
    - "https://github.com/scarecrow173/synvelis/issues/25"
  defers: []
  deferred-by: []
---

# Issue #288 IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計

## What Should Be Built

IO-01〜05（File / Structured / Compression / Archive / Migration）の
実装が main に揃った現在、両 Consumer（AudioLens・Synvelis）への
導入可否を判定するための **Gate 設計文書** を `docs/` チェーン
（spec → design → test-spec → plan → task）として作成する。
具体的には `docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md`
を新設し、Issue #288（合意済み設計 v0.1・2026-11-04）の要求を
次の構成要素として文書化する。

1. **受入 matrix 設計**: IO-01〜05 の正常系・否定系を、入力条件・
   障害条件・Capability/OS/FS の軸で検証するための判定表。
   各行に判定基準・結果記録フォーマット・根拠となる既存
   Issue/PR/test-spec/検証コマンドへの参照を持たせる。
2. **否定系判定表**: publish/recovery・安全性（Security）・
   resource budget・Media/PCM/RT 非回帰・Consumer 非回帰の
   各領域について、何をもって pass/fail/blocked/not run/
   unsupported とするかの判定規則。
3. **Owner・境界接続表**: IO-01〜05 のテスト要件を実装 Issue
   （#295〜#306）・OS/FS/脅威モデルの境界・Consumer Issue
   （AudioLens #139/#140/#141/#143、Synvelis #82/#84/#25）へ
   対応付ける索引。
4. **Consumer 要件の明文化**: Ruviform/AudioLens/Synvelis それぞれの
   影響分類（impacted/no impact/unknown）・Golden・回帰・性能・
   Dependency 更新の受入要件。
5. **Breaking Change 追跡導線**: 旧 `StagedOutput`/旧
   `PublishOptions` の意図的廃止（IO-05・#287・実装 #305/PR #391）
   の採用手順と戻し方を、#287 と各 Consumer Issue で追跡する
   ための参照導線。
6. **証拠導線**: 既存 GitHub Actions・PR・Issue の参照だけで
   必要な検証を追える構成。専用 YAML・新 Dashboard・重複
   レポートは作らない。
7. **設計合意とテスト完了の分離**: 「実装段階では各必須 Gate の
   実走 PASS を要求する」ことを明記し、本 slice の文書合意を
   テスト完了と混同しない。

## Why Now

- IO-01〜05 の設計・実装 Issue（#283〜#287 → #295〜#306）は
  main に merge 済みで、IO-05 の旧 API 撤去（#305・PR #391）と
  合成 E2E クレート `ruviform-io-e2e`（#306・PR #395）も main に
  存在する。次の関門は「この実装を両 Consumer が採用してよいか」
  の判定であり、その判定基準を固定する IO-06 が未文書化のまま
  残っている。
- Cross-Repo CI の実行機構は #219（互換契約）と #294
  （consumer-verify.yml 候補 tuple Gate）が Owner として確立
  済みだが、「どの Gate を・どの OS/FS で・何をもって PASS と
  するか」の受入側の定義がないと、run の証拠を採否判定へ
  接続できない。
- 旧 `StagedOutput`/`PublishOptions` は互換層なしで削除済み
  （意図的 Breaking Change）。Consumer 側の移行完了を追跡する
  導線が文書化されていないと、非互換の見落としや恒久 SHA 固定
  による先送りが起き得る。

## Users and Value

- **Ruviform maintainer / IO-06 Owner**: IO-01〜05 の導入 Gate の
  合否を、曖昧さなく判定するための正本を得る。
- **#219/#294 CI Owner**: consumer-verify.yml の lane 結果・
  tuple 証拠を「どの受入項目の証拠か」に対応付ける索引を得る。
- **AudioLens / Synvelis Owner**: 各 Consumer が満たすべき
  影響分類・Golden・回帰・性能・Dependency 更新の要件と、
  Breaking Change の採用・戻し手順を追跡できる。
- **後続の実装 Issue 担当**: 「実走 PASS が必要な Gate」と
  「本 slice で確定した設計」を混同せず、検証計画を立てられる。

## Scope

### In Scope

- `docs/` 配下の文書チェーン（spec/design/test-spec/plan/task）
  と生成 index のみ。**コード変更なし**。
- IO-01〜05 の受入 matrix・否定系判定表・Owner/境界接続表・
  Consumer 要件・Breaking Change 追跡導線・証拠導線・
  設計/実装分離の明記を含む設計文書。
- Issue #288 本文（合意済み設計 v0.1・2026-11-04・最終整合性
  レビュー追記）の要求事項の文書化。

### Out of Scope

- matrix の実走・テスト本体の実装（後続実装: #306 系の実 OS/FS
  試験、#219/#294 の consumer gate、各 Consumer repo の実測）。
- AudioLens/Synvelis 側の実測・コード変更（cross-repo gate、
  NotRun）。
- macOS/Linux 実機での検証（本環境は Windows のみ。CI 既存 lane
  の結果参照に委ね、未実測は Unsupported/NotRun と記録する）。
- 新規 CI Workflow・証拠 YAML/JSON スキーマ・集約 DB・
  Dashboard の作成（Issue #288/#294 の既存機構を使う）。
- `crates/`・`Cargo.toml`・`mise.toml`・`.github/` の変更。
- Issue #306 が所有する合成 E2E fixture の重複実装。

## Requirements

- 受入 matrix は IO-01（File）・IO-02（Structured）・
  IO-03（Compression）・IO-04（Archive）・IO-05（Migration）の
  正常系・否定系を、入力条件・障害条件・Capability/OS/FS 別に
  判定できる粒度で定義する。
- 各 matrix 行は判定基準（pass 条件）と結果記録方法を持ち、
  根拠となる既存 Issue/PR/test-spec/検証コマンドへ辿れる。
- publish/recovery・安全性・resource budget・Media/PCM/RT・
  Consumer 非回帰の否定系を個別の判定表で定義する。
- `impacted / no impact / unknown` と `pass / fail / blocked /
  not run / unsupported` の語彙を Issue #219/#294 と整合させ、
  `unknown`・`not run` を PASS 扱いしない規則を明記する。
- OS/FS の保証範囲は実測済みのものだけを主張し、未実測の
  OS/FS/Network/Cloud FS を Supported としない。
- 旧 API Breaking Change の採用手順・戻し方の追跡先を
  #287・#305・各 Consumer Issue へ接続する。
- 既存 Actions/PR/Issue 参照のみで検証を追えることを確認し、
  専用 YAML・重複レポートを新設しない。
- 「設計の合意 ≠ テスト完了」を明記し、実装段階の各必須 Gate
  の実走 PASS を別要件として分離する。

## Acceptance Criteria

- [ ] IO-01〜05 の正常系/否定系 × 入力・障害条件 ×
      Capability/OS/FS 別の結果を判定できる受入 matrix が
      設計文書に存在する。
- [ ] publish/recovery・安全性・resource budget・Media/PCM/RT・
      Consumer 非回帰の否定系判定表が存在する。
- [ ] IO-01〜05 のテスト要件と Owner・OS/FS/脅威モデルの境界が
      実装 Issue（#295〜#306 等）へ接続されている対応表がある。
- [ ] Ruviform/AudioLens/Synvelis の影響・Golden・回帰・性能・
      Dependency 更新要件が明文化されている。
- [ ] 旧 `StagedOutput`/旧 `PublishOptions` の意図的 Breaking
      Change の採用手順・戻し方の追跡導線が #287/#305 と各
      Consumer Issue へ接続されている。
- [ ] 既存 Actions/PR/Issue の参照だけで必要な検証を追える構成
      であり、専用 YAML・新 Dashboard・重複レポートを追加して
      いない。
- [ ] 「実装段階では各必須 Gate の実走 PASS を要求する」旨が
      明記され、設計合意とテスト完了が分離されている。
- [ ] 変更が `docs/` のみであり、`crates/`・`Cargo.toml`・
      `mise.toml`・CI workflow に差分がない。

## Deferred Design Concerns

- matrix の実走結果の集約・可視化 | Issue #288 は独自の証拠
  基盤を作らない方針であり、実走は既存 CI/PR/Issue に記録する
  | #294 の consumer-verify 拡張・#219 の Cross-Repo CI 運用が
  確定した時点 | 実走結果が Issue/PR に散在し追跡コストが
  問題になった場合。
- Network/Cloud FS（NFS/SMB 等）の保証範囲の実測 | 実機・
  環境が未確保 | Consumer が該当 FS をサポート対象に含める
  判断が出た時点 | 未実測のまま Supported と誤認される
  リスク。
- macOS/Linux 実機での OS/FS 別 Gate 実走 | 本 slice の作業環境は
  Windows のみ | CI の既存 OS lane または実機環境が利用可能に
  なった時点 | OS 別差異（APFS/ext4 の DirSync 等）が未検証の
  まま残る。

## Sources and Relations

- 正本 Issue: [#288](https://github.com/scarecrow173/ruviform/issues/288)
  （IO-06 合意済み設計 v0.1・2026-11-04・最終整合性レビュー追記）。
- 親設計正本: [#282](https://github.com/scarecrow173/ruviform/issues/282)
  （汎用 I/O 基盤 横断設計）。
- 実装 Issue 群: IO-IMP-01〜12 = #295〜#306（main merge 済み）。
- Cross-Repo/CI Owner: #219（互換契約）・#294（consumer-verify
  候補 tuple Gate）。
- Consumer Issue: AudioLens #139/#140/#141/#143、Synvelis
  #82/#84/#25。
- 参照設計文書: `docs/designs/issue-296-io-file-os-backend.md`
  （OS/FS capability matrix）、各 `docs/specs|test-specs/
  issue-29x-io-*.md`・`issue-30x-*.md`。
