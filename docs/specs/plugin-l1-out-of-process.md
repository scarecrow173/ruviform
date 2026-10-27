---
id: "SPEC-034LNeyep4t7pIzFlpPyTK"
type: "spec"
status: "proposed"
title: "プラグイン L1 アウトプロセス分離（plugin-host L1 プロキシ + plugin-export 薄ランナー）"
created: "2026-10-25"
updated: "2026-10-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/76"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/plugin-host-l0.md"
    - "docs/specs/implement-plugin-foundation.md"
    - "docs/specs/plugin-gui-poc.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/plugin-l1-out-of-process.md"
    - "docs/specs/plugin-isolation-design.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/specs/plugin-host-l0.md"
    - "docs/designs/plugin-host-l0.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/discovery/engine-plugin-host-integration.md"
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/discovery/issue-114-real-adapter-l1-e2e.md"
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
    - "docs/plans/implement-issue-119-l1-wire-version-policy.md"
    - "docs/discovery/issue-117-bundle-child-process-discovery.md"
    - "docs/specs/issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references: []
  defers:
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
  deferred-by: []
---

# プラグイン L1 アウトプロセス分離（plugin-host L1 プロキシ + plugin-export 薄ランナー）

## What Should Be Built

Issue #76 — Issue #31 のプラグイン分離設計（ADR `docs/adr/plugin-isolation-strategy.md`・design `docs/designs/plugin-isolation.md`）「実装単位（後続 Issue の分割）」に基づく後続実装。Issue #74 で確定した `ruviform-plugin-host` の境界契約（`PluginHandle` 不透明ハンドル + `HostCommand`/`HostReply`/`HostError` 直列化可能コマンド面）を変えずに、**opt-in の L1 アウトプロセス経路**を実装する:

1. **L1 プロキシ（`ruviform-plugin-host` 内）** — 子プロセス spawn・監視、非 RT コマンドのプロセス間転送、`process` 等価の共有メモリ SPSC 経路を持つバックエンド。呼出し側は L0/L1 を意識しない同一公開面
2. **`ruviform-plugin-export` クレート新設** — L1 子プロセス側エントリとなる薄いランナー（ベースライン A.2 の予定クレート: 制作側エントリ基盤）
3. **非 RT `process` 等価経路** — 非 RT ドライバが共有メモリリングで audio block を送受信。RT スレッドは結果を `try_recv` で読むのみでプロセス間通信を直接待機しない。遅延は先行ブロック数 × ブロック長
4. **クラッシュ検出** — 子プロセス監視（wait/exit status）でクラッシュを検出し、インスタンスを `Failed` 状態へ遷移・ホストへ通知。自動再起動はしない
5. **フォールバック** — クラッシュ・応答なし時は pass-through（直前の既受信データ or 無音）

IPC 機構の具体選択・ワイヤプロトコル形式・`Failed` 表現の所有・runner のプロセス形態・L1 GUI の扱いは対の design doc で確定する（discovery に比較を記録済み）。

## Why Now

Issue #76。#31 分割で L0（#74・CLOSED）の次に位置づけられた実装単位。分離 ADR の動機は「外部プラグインは未検証コードでありクラッシュがホスト全体を巻き込む」— L0 既定のままでは信頼できないプラグインを隔離できない。`ruviform-plugin-export` はベースライン A.2 で責務予約済みだが未実装であり、本 Issue がその初回実装（薄いランナー形）を兼ねる。

## Users and Value

- ホストアプリケーション / エンジン実装者 — インスタンス単位でアウトプロセス隔離を opt-in できる。呼出し側の API・コマンド系列は L0 と同一で、層選択のみが新しい判断
- 未検証・外部プラグインを扱う利用側 — プラグインのクラッシュがホストプロセスを巻き込まず、定義済みの `Failed` 遷移と pass-through で音切れせず継続する
- RT パス実装者 — L1 でも RT 契約が維持される（RT スレッドは IPC を待機しない・`try` ベースの結果読取りのみ）ことが API 契約として明示される

## Scope

### In Scope

- `ruviform-plugin-host` への L1 バックエンド追加 — 子プロセス spawn・監視（wait/exit status）・コマンド直列化送受・共有メモリ SPSC による `process` 等価経路。公開面（`PluginHandle` + コマンド語彙）は L0 と同一
- `ruviform-plugin-export` クレート新設 — L1 子プロセス側の薄いランナーエントリ（workspace メンバー追加・`publish.workspace = true` 継承）
- 非 RT ドライブの `process` 等価経路 — 共有メモリ上で audio block + イベント/transport を受け渡し、非 RT スレッドが駆動。RT 側は `try` ベース読取りのみ
- クラッシュ検出 → `Failed` 遷移 + ホスト通知、以後のコマンドに対する定義済み拒否。自動再起動なし
- 遅延契約（先行ブロック数 × ブロック長）と pass-through ポリシー（既受信データ or 無音）の実装・doc 明記
- 単体/結合テスト — コマンド往復・process 経路・クラッシュ模擬（子プロセス強制終了）・pass-through・`rt_` 命名の RT 契約テスト（RT 側読取り経路の no-block/no-alloc）
- crate doc（`lib.rs`）・クレート README・`crates/README.md` 索引の更新

### Out of Scope

- 自動再起動・再起動ポリシー（再起動判断・状態復元は呼出し側とプラグイン責務 — 設計確定済み）
- L2+ サンドボックス（seccomp・job object・権限分離等の OS 隔離強化）
- 規格アダプタ（`ruviform-plugin-clap`/`-vst3`/`-ofx`）側の子プロセス組込み・実プラグインでの E2E — runner はテスト/モックプラグインで検証可能な形に留め、アダプタ統合の形は design が境界を定める
- エンジン（`ruviform-audio`/`ruviform-video`）・利用側への L1 選択配線 — consumer 側責務
- L0（`InProcessHost`）の挙動変更 — 既存 API・RT 経路は維持
- L1 での GUI 実装（プロセス境界越しの親ウィンドウ埋め込みは規格・プラットフォーム固有課題。defined rejection とするか明示除外するかは design が確定）
- `process` 経路の RT 化（L1 は構造上非 RT — RT スレッドからの直接駆動は契約違反）

## Requirements

- **境界維持**: `PluginHandle`・`HostCommand`/`HostReply`/`HostError` の公開面を L0 と同一に保ち、呼出し側が L0/L1 を意識しない（#31 境界契約）。新規公開 enum は `#[non_exhaustive]` 方針
- **opt-in**: L0 が既定のまま。L1 はインスタンス単位等で選択できる経路として追加する（選択面の API 形は design）
- **非 RT `process` 等価**: L1 の `process` 等価経路は非 RT スレッドが駆動し、RT スレッドは結果を `try_recv` で読むのみ — プロセス間通信・ブロッキングを RT パスへ持ち込まない（RT 契約 ADR 整合）。RT 側読取り経路は alloc-free・非ブロッキングとして `rt_` テストで検証する
- **遅延契約**: L1 へ送ったブロックの遅延は先行ブロック数 × ブロック長。先行ブロック数の既定値・設定面は design で確定し doc に明記する
- **クラッシュ契約**: 子プロセスの wait/exit status でクラッシュを検出し、対象インスタンスを `Failed` 状態へ遷移・ホストへ通知する。`Failed` 表現の所有（plugin-core 拡張 vs host 側状態）は design で確定。`Failed` 以後の全コマンド・`process` 等価は定義済み拒否とする。自動再起動はしない
- **フォールバック**: クラッシュ・応答なし時の出力は pass-through — 直前の既受信データまたは無音。応答タイムアウトの判定面は design
- **依存方向**: `ruviform-plugin-export` の許可依存はベースライン A.2（Domain foundations + Primitives + 拡張層 `ruviform-plugin-core`）に従う。plugin-host 側の追加依存（IPC/直列化・`ruviform-rt` 等の Primitives）はベースライン層ルール内に限定し、外部依存の採用は暫定規約（メンテナンス状況・ライセンス・依存木サイズの確認と理由記録）に従う — shm IPC 機構の採否は design で確定
- **`unsafe` 原則禁止**: 共有メモリ・プロセス制御の実装は安全 API を持つ外部クレートを内部依存に閉じ込める方針を優先し、自前 `unsafe` は局所許可が必要な場合のみ（`rtrb` 先例）
- **ワイヤ境界**: コマンド直列化・共有メモリレイアウトはワイヤ仕様として定義し、所有境界（plugin-core / host / export のどこが持つか）は design で確定（ベースライン層順序との整合が課題 — discovery 記録済み）
- **ワイヤバージョン方針（Issue #119 で確定）**: サポートする wire プロトコルは常に最新の単一バージョンのみ — `Hello.version` は厳密一致で不一致は handshake 失敗（kill + `HostError::Ipc`）、shm header の `wire_version` も同一の厳密一致検証とする。バージョン交渉・上限交渉機構・旧バージョンの decode/変換/共存は実装しない。`WIRE_VERSION` は非互換なプロトコル変更時のみ更新する通信契約の世代番号（SemVer/Cargo バージョン非連動）で、bump 時はホスト/ランナーを同一版で更新する。`Hello.capabilities` は予約領域のまま非ゼロ `Malformed` 拒否を維持、`WIRE_MAX_*` は固定安全上限として維持する。複数バージョン対応は独立配布のホスト/ランナー互換が必要になった段階で再検討する
- `rt_` 命名の RT 契約テストを持ち `mise run test-rt` で選択可能。非 RT 系テスト名に `rt_` を含めない

## Acceptance Criteria

- [ ] L1 バックエンドが L0 と同一のコマンド系列（scan → instantiate → initialize → activate → start/stop processing → deactivate → save/restore state → destroy）を、子プロセス側で実行された上で同一のリプライ/エラー意味で処理できる（違法遷移は定義済み `Lifecycle` エラーとして返る）
- [ ] `process` 等価経路でブロック送信 → 定義された遅延後に `try` ベース読取りで処理済み出力が得られ、テストプラグイン（例: gain）でデータ正当性が検証できる
- [ ] RT 側読取り経路が no-block・no-alloc で、`rt_` 命名テストが `mise run test-rt` でグリーン
- [ ] 子プロセスの強制終了（kill）が wait/exit status で検出され、インスタンスが `Failed` へ遷移・ホスト通知が行われ、以後のコマンド/`process` 等価が定義済み拒否となる。自動再起動が発生しないことが観測できる
- [ ] クラッシュ/応答なし状態での `process` 等価呼出しが pass-through（直前の既受信データ or 無音）を返し、ハングしない
- [ ] `ruviform-plugin-export` が workspace メンバーとして存在し、薄いランナーエントリを持ち、spawn→コマンド応答→process 経路→終了の結合がテスト/モックプラグインで検証される
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス・doc 監査で本チェーン起因 findings 0

## Deferred Design Concerns

- L1 GUI（プロセス境界越し親ウィンドウ埋め込み）| 規格・プラットフォーム固有の大課題であり本 Issue 範囲を超える | L1 選択インスタンスでの GUI 要求の発生 | 隔離プラグインが GUI を表示できない（defined rejection 時）
- 自動再起動・監視ポリシー | 状態復元はプラグイン責務の設計判断（#31）| supervisor 型運用要求の発生 | 手動復旧の運用負荷
- 規格アダプタの子プロセス組込み（実 CLAP/VST3 の L1 実行） | runner 形態の設計確定が前提・別 Issue 級の配線 | 実プラグインでの L1 需要 | 検証がモック範囲に留まる
- エンジン/利用側への L1 選択配線 | plugin-host の engine 消費自体が未実装 | engine プラグイン統合 Issue | L1 が consumer から到達不能
- 複数 wire バージョン共存・交渉機構 | 初期開発期の単一最新バージョン方針を Issue #119 で確定（交渉機構・互換レイヤーなし — Requirements「ワイヤバージョン方針」参照） | 独立配布のホスト/ランナー間バージョン互換の要求発生 | 独立配布形態で新旧バージョンの混在が扱えない

## Sources and Relations

- Issue [#76](https://github.com/scarecrow173/ruviform/issues/76)（本 spec の対象 Issue）・Issue #31（分離設計）・Issue #74（L0 実装・CLOSED）
- docs/discovery/plugin-l1-out-of-process.md（本 spec の discovery — issue framing・確認済みギャップ・IPC 機構候補比較・クラッシュ検出/失敗表現/runner 形態の選択肢）
- docs/specs/plugin-isolation-design.md（#31 umbrella spec・approved — 実装単位分割の親）
- docs/designs/plugin-isolation.md（L1 契約の正本 — プロキシ配置・shm SPSC 分岐・非 RT 経路・Failed+通知・遅延/pass-through・approved）
- docs/adr/plugin-isolation-strategy.md（階層化分離の決定・accepted）
- docs/specs/plugin-host-l0.md / docs/designs/plugin-host-l0.md（#74 spec/design — 本 Issue の基盤となるハンドル + コマンド境界）
- docs/designs/ruviform-architecture-design-baseline.md（A.2 クレート表 — `ruviform-plugin-export` 行・層構造・依存方向ルール・RT 契約）
- docs/specs/implement-plugin-foundation.md / docs/specs/plugin-gui-poc.md（plugin 系の兄弟 spec — plugin-core 基盤・GUI PoC 前提）
