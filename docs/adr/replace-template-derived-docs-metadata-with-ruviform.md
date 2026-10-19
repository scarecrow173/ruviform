---
id: "ADR-034Ivm746Q38Wa21HeqUiI"
type: "adr"
status: "accepted"
title: "テンプレート由来ドキュメント・識別子のRuviform固有化方針"
created: "2026-10-19"
updated: "2026-10-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/3"
  changes:
    added: []
    modified:
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "apm.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".pre-commit-config.yaml"
      - type: "file"
        path: ".gitignore"
      - type: "file"
        path: ".gitattributes"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "rust-dev-foundation-toolchain-and-ci-profile.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../README.md"
    - "../../apm.yml"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/replace-template-derived-docs-with-ruviform.md"
    - "https://github.com/scarecrow173/ruviform/issues/3"
  derived-by:
    - "../designs/replace-template-derived-docs-with-ruviform.md"
    - "../plans/replace-template-derived-docs-with-ruviform.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/2"
    - "https://github.com/scarecrow173/ruviform/issues/1"
    - "../impl/ir/verify-reproducible-setup-and-work-memo.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# テンプレート由来ドキュメント・識別子のRuviform固有化方針

## Context and Problem Statement

GitHub Issue #3 と上流 spec `SPEC-034Ivc5nj1AE6P2PcZU6Ki`（[../specs/replace-template-derived-docs-with-ruviform.md](../specs/replace-template-derived-docs-with-ruviform.md)）に基づき、リポジトリ初期化時に引き継いだ汎用プロジェクトテンプレート由来の説明・識別子・作業規約を、Ruviform の現状と開発方針に合わせて更新する。現状、`README.md` は `# Project Template` で「Use this template」による新規プロジェクト作成手順を持ち、`AGENTS.md` は「言語・フレームワーク非依存テンプレート」「派生プロジェクト」前提の記述と「テンプレートに言語固有の構成・CI・docs を追加しない」等の禁止事項を含み、`apm.yml` は `name: project-template` のままである（`git grep` で確認済みの `project-template`・`Project Template` のリテラル一致は `README.md` と `apm.yml` の 2 箇所。テンプレート前提の記述は `AGENTS.md`・`mise.toml` にも残存）。これらは Rust を中心とした再利用可能なマルチメディア基盤としての実態と矛盾し、訪問者・コントリビューター・AI エージェントを誤導する。ドキュメント定位・暫定/確定の境界・APM 識別子・ライセンス表記・LFS ポリシーという相互に関係する決定を、1 つの ADR として記録する。

## Decision Drivers

* 利用者・コントリビューター・AI エージェントが Ruviform の実態（Rust 中心の再利用可能なマルチメディア基盤、設計・初期セットアップ段階）を誤解しない
* 未実装・未決定・未検証の項目を実装済み・決定済み・検証済みと誤認させない（虚偽記載の防止）
* #2 で実機検証済みの再現手順（`mise install --locked`、`mise run setup`、mise 検証タスク）と矛盾しない
* テンプレート由来の有用な仕組み（mise / APM / prek / Git LFS の運用、設計原則）は維持する
* #1（詳細設計）に属する決定（最終クレート分割・公開 API・対象プラットフォーム・ライセンス）を本 ADR で既成事実化しない

## Considered Options

**ドキュメント定位（README/AGENTS）:**

* テンプレートの文言を維持する
* 名称・識別子のみを置換し、構成・説明はテンプレートのままにする
* README/AGENTS を Ruviform 固有の作業文書として改稿し、有用な設計原則のみ保持する

**暫定 vs 確定の境界（アーキテクチャ・構想の記載方法）:**

* アーキテク構想を README/AGENTS に一切書かず、#1 の詳細設計に全面委譲する
* 現時点の意図を「構想・検討中・未決定」と明示した上で記載する
* `ruviform-audio` / `ruviform-video` / DSP・SIMD / GPU・UI・プラグイン等を確定方針として記載する

**APM 識別子（`apm.yml`）:**

* `name: project-template` を維持する
* `name` を `ruviform` に変更し `description` を実態に合わせる（`targets`・依存パッケージ・MCP 設定は維持）
* `name` 変更と併せて依存・`targets` を再編成する

**ライセンス表記:**

* `Cargo.toml` の `license = "MIT"` に合わせ、MIT 決定済みとして記載する
* ライセンスに一切言及しない
* `LICENSE` 不在・ライセンス未決定であることを明示し、`Cargo.toml` の値は仮置きと注記する

**LFS ポリシー（`.gitattributes`）:**

* 拡張子ベースの音声・動画 LFS ルールを維持し、小さなテストフィクスチャは非メディア形式で Git に置く方針を文書化する
* メディア資産がまだ存在しないため LFS ルール自体を削除する
* フィクスチャを含む全メディアファイルを一律 LFS 対象とする

## Decision Outcome

以下の組み合わせを採用する。

* **ドキュメント定位**: README/AGENTS を Ruviform 固有の作業文書として改稿する。「Use this template」手順と、派生プロジェクト前提の禁止事項（「テンプレートに言語固有の構成・CI・docs を追加しない」等）を削除し、変更前の診断・責務境界での修正・実行検証と未実行検証の区別といった設計原則は保持する。テンプレート文言の維持は利用者を誤導するため不採用、名称のみの置換は本文と実態の矛盾を残すため不採用。
* **暫定/確定の境界**: RT/オフラインの設計意図、クレート方向（`ruviform-audio` / `ruviform-video` と下位の DSP・解析・codec・container・SIMD）、`wgpu`・WebView UI・ネイティブ UI・プラグイン連携は、いずれも「構想・検討中・未決定」と明示した上で記載し、実装済み・決定済みとして記さない。全面委譲は README が #1 に先行する構想説明としての役割を失うため不採用、確定方針としての記載は #1 の判断を先取りして既成事実化するため不採用。
* **APM 識別子**: `name` を `ruviform`、`description` を Ruviform に沿う内容に更新する。`ruviform` は小文字・短尺の slug であり、現行 `project-template` と同形式で APM の命名慣習に適合する。APM 0.33.0 には `apm validate` 相当の専用検証コマンドが存在しないため、命名制約の適合は `apm install` / `apm lock` の成功と `apm.lock.yaml` の整合で確認する。`targets`（copilot/codex/claude/cursor/windsurf）・依存パッケージ・Serena/mise MCP 設定は変更の根拠がないため維持する。
* **ライセンス表記**: `LICENSE` ファイルがリポジトリに存在しない事実に基づき、ドキュメントにはライセンス未決定を明示し、`Cargo.toml` の `license = "MIT"` は仮置き（配布方針の確定ではない）と注記する。本 Issue でライセンス自体は決定しない。MIT 決定済みとしての記載は未確定の配布方針を既成事実化するため不採用、一切の言及を避ける方針はマニフェスト値と文書の不整合を読者が発見できなくするため不採用。
* **LFS ポリシー**: `.gitattributes` の音声（`*.wav` `*.flac` `*.mp3` `*.ogg` `*.opus` `*.aac` `*.m4a` `*.aif` `*.aiff`）・動画（`*.mp4` `*.mov` `*.mkv` `*.avi` `*.webm` `*.m2ts`）の拡張子 LFS ルールを維持し、小さなテストフィクスチャは非メディア形式で Git に置く方針を文書化する。#2 の結論（小さいフィクスチャは Git 管理、CI はフィクスチャが必要になるまで LFS オブジェクトを取得しない）と整合する。ルール削除は将来メディア資産が混入した際に Git 履歴を肥大化させるため不採用、全メディア一律 LFS 化は小さなフィクスチャの diff・レビューを不可能にするため不採用。

### Consequences

* 良い点: README/AGENTS/`apm.yml` が Ruviform の実態と一致し、テンプレート手順への誘導や、AI エージェントが未実装・未検証の項目を実在/実行済みと報告するリスクが減る。決定済み・暫定・未決定の境界が文書上明示され、#1 の設計者が前提を把握できる。mise/APM/prek/LFS のインフラ基盤は温存される。
* 悪い点: README の構想記述は #1 の決定後に追従更新が必要で、追従漏れのリスクが残る。ライセンス未決定の明示は暫定状態であり、公開・配布を始めるまでに別途決定が要る（放置すると `Cargo.toml` 仮置き値との不整合が続く）。`apm.yml` の `name` 変更は APM 配布生成物に波及するため、変更後に `apm install` 相当の再生成・整合確認が必要になる。
* 中立: `.pre-commit-config.yaml` / `.gitignore` / `.gitattributes` はレビュー対象であり、Ruviform に不適切な箇所のみ修正する（変更なしも許容）。

### Confirmation

下記 Verification のチェックリストを Issue #3 の受け入れ条件（spec の Acceptance Criteria）と併せて確認し、実装 PR で `README.md` / `AGENTS.md` / `apm.yml` / `mise.toml` / `.pre-commit-config.yaml` / `.gitignore` / `.gitattributes` の差分と `prek` フック実行結果をレビューする。

## Implementation Plan

* 影響するパス:
  * `README.md` — 全面改稿（タイトル・概要・対象読者・実測済み Quickstart・リポジトリ構成・設計文書/Issue への導線・ライセンス未決定の明示）
  * `AGENTS.md` — 作業契約の Ruviform 固有化（有用な設計原則は保持、派生前提の禁止事項は削除、実ワークスペースと mise タスク経由の検証規約を反映）
  * `apm.yml` — `name`/`description` のみ更新（`targets`・依存・MCP は維持）
  * `mise.toml` — テンプレート前提コメント・task description の現状適合（タスク定義本体は変更しない）
  * `.pre-commit-config.yaml` / `.gitignore` / `.gitattributes` — レビューと必要箇所のみの修正
  * `Cargo.toml` — ライセンス表記の整合確認対象。本 Issue では変更しない
* 従うパターン: [../designs/overview.md](../designs/overview.md) のレイヤ境界（README/AGENTS 本文の改稿は #3 の責務）、[../designs/dev-foundation.md](../designs/dev-foundation.md) の暫定開発規約、IR [../impl/ir/verify-reproducible-setup-and-work-memo.md](../impl/ir/verify-reproducible-setup-and-work-memo.md)「作業メモ（#3 転記用）」の実測済み手順を一次記録とする
* 避けるパターン: 存在しない文書・実装・検証を実在/実行済みとして記載しない（`LICENSE`・公開 API・`ruviform-audio`/`ruviform-video`・RT-safe 保証・未実施の GitHub Actions ディスパッチ等）。テンプレート由来修正と無関係なインフラ基盤の再構成を行わない。不要な全置換を行わない
* 依存関係: spec `SPEC-034Ivc5nj1AE6P2PcZU6Ki`、ADR-034IcJdtqm6TP6MylyP5J7（Issue #2 開発基盤、`depends-on`）の toolchain・CI・lockfile 追跡方針。`apm.yml` の `name`・依存・`targets` を変更する場合は `apm install` / `apm lock` で `apm.lock.yaml` を再生成し、生成物との整合を確認する

## Pros and Cons

**ドキュメント定位 — テンプレート文言の維持:**

* 良い点: 変更作業が不要。
* 悪い点: 「Use this template」手順・派生プロジェクト前提の規約が Ruviform の実態と矛盾し、訪問者・コントリビューター・AI エージェントを誤導する。

**ドキュメント定位 — 名称・識別子のみの置換:**

* 良い点: 差分が小さい。
* 悪い点: テンプレートとしての説明文（言語非依存・派生プロジェクト向け制約）が残り、本文と実態の矛盾は解消しない。

**ドキュメント定位 — Ruviform 固有の作業文書への改稿（採用）:**

* 良い点: 実態に即した Quickstart・構成説明・作業契約となり、誤導を解消できる。有用な設計原則は保持される。
* 悪い点: 改稿量が大きく、#1 の決定後にアーキテクチャ記述の追従が必要になる。

**暫定/確定 — アーキテクチャ記述の全面委譲:**

* 良い点: 未決定事項の記載リスクがゼロ。
* 悪い点: README が #1 に先行する構想説明としての役割を失い、リポジトリの目的が外部から読み取れない。

**暫定/確定 — 「構想・検討中・未決定」の明示付き記載（採用）:**

* 良い点: 現時点の意図を伝えつつ、#1 の決定余地を侵害しない。決定済み・暫定・未決定の境界が読者に明確になる。
* 悪い点: 表現を誤ると構想が確定方針に読み替えられるため、記述のレビューが要る。

**暫定/確定 — 確定方針としての記載:**

* 良い点: 記述が断定的で読みやすい。
* 悪い点: 未実装・未決定の項目を既成事実化し、#1 の詳細設計を不当に拘束する。

**APM 識別子 — `project-template` の維持:**

* 良い点: `apm.lock.yaml`・生成物への波及がない。
* 悪い点: リポジトリ識別子が実態と一致せず、APM が生成する配布物・コンテキストにも誤った名前が残る。

**APM 識別子 — `ruviform` への変更（採用）:**

* 良い点: 識別子がリポジトリ名・実態と一致する。小文字 slug で命名慣習に適合し、`targets`・依存・MCP を維持するため影響が最小。
* 悪い点: APM 配布生成物の再生成が必要。命名制約の検証手段が専用コマンドではなく install/lock 成功の確認に限られる。

**APM 識別子 — 依存・`targets` の再編成:**

* 良い点: 不要なターゲットを削れる可能性がある。
* 悪い点: #3 の範囲を超え、動作するエージェント環境を壊すリスクがある。正当な根拠がない。

**ライセンス表記 — MIT 決定済みとして記載:**

* 良い点: `Cargo.toml` の仮置き値と文書が一致する。
* 悪い点: `LICENSE` 不在のまま配布方針を既成事実化し、本 Issue のスコープ外決定を先取りする。

**ライセンス表記 — 言及しない:**

* 良い点: 誤った記載を避けられる。
* 悪い点: `LICENSE` 不在と `Cargo.toml` 仮置き値の不整合を読者が発見できず、未決定状態が見えなくなる。

**ライセンス表記 — 未決定の明示と仮置き注記（採用）:**

* 良い点: 事実（`LICENSE` 不在・マニフェスト値は暫定）を正確に伝え、配布方針の決定を #1 以降へ明示的に委譲できる。
* 悪い点: ライセンス決定自体は未解決のまま残る。

**LFS ポリシー — ルール削除:**

* 良い点: 現時点でメディア資産がないため実害はない。
* 悪い点: 将来メディア資産が混入した際に Git 履歴が肥大化し、LFS 運用の前提（`git lfs install --local`・mise 管理）と矛盾する。

**LFS ポリシー — フィクスチャ含む全メディアの一律 LFS 化:**

* 良い点: ルールが単純。
* 悪い点: 小さなテストフィクスチャの diff・レビューが不可能になり、CI が不要な LFS 取得を強制される。#2 の結論と矛盾する。

**LFS ポリシー — 拡張子ルール維持＋非メディアフィクスチャ方針（採用）:**

* 良い点: 大きなメディア資産を LFS に逃がしつつ、小さなフィクスチャは Git で diff 可能。CI の LFS 取得不要方針と整合する。
* 悪い点: フィクスチャに音声/動画拡張子を使いたい場合は非メディア形式への置き換え等の運用判断が必要になる。

## Verification

* [ ] `git grep -i "project.template"` 相当で、`README.md`・`AGENTS.md`・`apm.yml`・`mise.toml`・`.pre-commit-config.yaml`・`.gitignore`・`.gitattributes` を含む追跡ファイルに `project-template` / `Project Template` / テンプレート前提の記述が残存しない（`docs/` 内の履歴記録は除く）
* [ ] `README.md` が「Use this template」手順を持たず、#2 実機検証済みの `mise install --locked`、`mise run setup`、`mise run fmt-check` / `check` / `lint` / `test` / `build` / `verify-ci` を記載する
* [ ] README/AGENTS で `ruviform-audio` / `ruviform-video` / DSP・SIMD / `wgpu`・UI・プラグイン・RT 保証が「構想・検討中・未決定」として記載され、実装済み・決定済みを示す表現がない
* [ ] `apm.yml` の `name` が `ruviform` で `description` が実態に合い、`targets`・依存パッケージ・Serena/mise MCP が維持されている。`apm install`（`name`・依存・`targets` の変更時は `apm lock` / `apm.lock.yaml` 再生成と生成物との整合確認を含む）が成功する
* [ ] README/AGENTS がライセンス未決定を明示し、`Cargo.toml` の `license = "MIT"` を仮置きと注記する。`LICENSE` ファイルは作成しない
* [ ] `.gitattributes` の音声・動画 LFS 拡張子ルールが維持され、小さなテストフィクスチャを非メディア形式で Git に置く方針が文書化されている
* [ ] `apm.yml` / `mise.toml` / `.pre-commit-config.yaml` の構文検証と `mise exec -- prek run --all-files` が成功する
* [ ] 未検証 OS・未実施の CI ディスパッチ等、未実行・未検証の条件が README/AGENTS に明示されている

## More Information

* 一次ソース: GitHub Issue [#3 [docs] README・AGENTS・APM等のテンプレート由来部分をRuviform向けに置き換える](https://github.com/scarecrow173/ruviform/issues/3)
* 上流ドキュメント: [SPEC-034Ivc5nj1AE6P2PcZU6Ki](../specs/replace-template-derived-docs-with-ruviform.md)（`relations.derives-from`）
* 前提決定: [ADR-034IcJdtqm6TP6MylyP5J7](./rust-dev-foundation-toolchain-and-ci-profile.md)（Issue #2 開発基盤、`relations.depends-on`）
* 関連ドキュメント: [AGENTS.md](../../AGENTS.md)、[README.md](../../README.md)、[apm.yml](../../apm.yml)（`relations.related`）
* 参考資料: GitHub Issue [#2](https://github.com/scarecrow173/ruviform/issues/2)（前提）、[#1](https://github.com/scarecrow173/ruviform/issues/1)（詳細設計、後続）、IR [verify-reproducible-setup-and-work-memo.md](../impl/ir/verify-reproducible-setup-and-work-memo.md)（#2 実機検証の一次記録）
