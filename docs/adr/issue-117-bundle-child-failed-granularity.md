---
id: "ADR-034Qb8Lg3MqZx1SuVcEj6k"
type: "adr"
status: "accepted"
title: "L1 bundle 子の `Failed` 粒度・`InstanceFailed` fan-out・隔離選択面（Issue #117）"
created: "2026-10-28"
updated: "2026-10-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-117-bundle-child-process.md"
    - "docs/discovery/issue-117-bundle-child-process-discovery.md"
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/adr/plugin-isolation-strategy.md"
  derived-by:
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/tasks/l1-child-instance-split.md"
    - "docs/tasks/l1-route-pool-and-isolation-surface.md"
    - "docs/tasks/l1-bundle-e2e-and-granularity-verification.md"
    - "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/119"
  defers: []
  deferred-by: []
---

# L1 bundle 子の `Failed` 粒度・`InstanceFailed` fan-out・隔離選択面（Issue #117）

## Context

Issue #117 は L1 を「1 インスタンス = 1 子プロセス」から **1 子プロセスが N インスタンスを host する bundle 形態**へ拡張する（設計: `docs/designs/issue-117-bundle-child-process.md`）。現行は子死亡 → その 1 インスタンスの `Failed` という 1:1 写像であり、`Failed` は「この論理インスタンスは二度と応答しない」という公開契約として `LifecycleState`・`HostError::Failed`・`HostNotification::InstanceFailed` に固定されている。bundle 化すると「子単位の共有資源（プロセス・stdin/stdout pipe・シーケンス相関空間）の破壊」と「インスタンス単位の障害」の区別が契約上問題になる:

- pipe が壊れる・frame がデコード不能・帰属不能な相関違反が起きたとき、**その破壊がどのインスタンスに属するかを host は正直に判定できない** — 「恐らく instance X 宛」と推測して部分 `Failed` にするのは虚偽であり、逆に全員を巻き込む根拠のない違反もある
- observer が受け取る `InstanceFailed{handle, exit}` の粒度と fan-out 規則を確定しないと「自分の handle がいつ `Failed` になるか」が曖昧になる
- bundle は crash 隔離の粒度を退化させるため、**強い隔離を選ぶ手段を公開 API が残す必要**がある（階層化分離 ADR `docs/adr/plugin-isolation-strategy.md` の「opt-in 分離」精神）

この 3 点は公開語彙（`LifecycleState::Failed`・`HostError`・`HostNotification`・`OutOfProcessSpec`）の意味に関わるため ADR として記録する（discovery が ADR 候補として指定した 3 論点 — `docs/discovery/issue-117-bundle-child-process-discovery.md` L294-298）。

## Decision

### 1. `Failed` 粒度 — 子スコープ terminal を基礎とし、帰属可能な違反のみ instance 単独 `Failed` を許す

- **子スコープ収束**（同居全 live インスタンスを `Failed` へ）: 子死亡・kill・stdout EOF・stdin write 失敗・デコード不能 frame（`instance` 範囲外/scope 不整合の `Malformed` を含む）・未知 kind・重複 `Hello`・**live instance に解決する** waiter に無い `Reply`・`Reply.instance` と送信記録の不一致・コマンド応答 timeout。**帰属不能な破壊を部分 `Failed` に切り分けることはしない** — 「この破壊は instance X 起因」と主張できないのに他を「健全」と宣言する方が不正確だから
- **instance スコープ収束**（対象のみ `Failed`、子と兄弟は生存）: frame が well-formed で demux キー `(instance, block seq)` が一意に live instance へ帰属する違反のみ — 定義上は「live instance に対し `block_in_flight` に無い block seq の `BlockDone`/flagged `Reply` が届く」こと。runner registry は instance 毎に lifecycle・segment・block seq を分離しており、その組の破壊はそのインスタンスの channel 破壊と確定できる
- **inert — 違反ではなく静かに drop**（いずれの収束も起こさない）:
  - `instance` が **registered-but-not-live**（`converge_instance` 済み `Failed` 未 `Destroy`・`Destroying`）な entry に解決する frame — waiter 不在の `Reply`・`block_in_flight` に無い `BlockDone`/flagged `Reply`。記録は収束・破棄経路が正当に drain 済みであり、後追い frame を違反とみなすと正常経路の遅延完了で子・兄弟を殺してしまう（典型: `converge_instance(i)` が doorbell に `mark_failed` を合成した後の `BlockDone{i,s}`・waiter drain 後の `Reply{i}`）。**「waiter-less `Reply` ⇒ 子スコープ」の厳格規則は live instance にのみ適用する**
  - `instance ∉ Inner.instances`（post-removal or 未登録値）宛の `Reply`/`BlockDone` 完了 — `DestroyInstance` reply で entry が除去された後の遅延完了・未使用値の捏造 frame。**決して protocol violation にしない**（追跡対象が存在せず、捏造 handle による兄弟殺し = DoS を拒む）
  - `Shutdown` 送信済み（`shutdown_initiated`）の子から届く frame・EOF — non-event。clean teardown を変えない（現行 `failed = !shutdown_initiated` 意味の維持）
- per-command の `Err(HostError)`（`PluginFailed`/`Lifecycle`/`InvalidHandle`/`InstanceLimit`/`Unsupported`）は現行どおり **instance 単位の定義済み拒否** — well-formed reply は常に帰属可能であり `Failed` ではない
- 完全インスタンススコープ（子死亡でも他は `Failed` にしない）は **物理的に不可能** — プラグインコードは子プロセスと共に死ぬ

（注: 「registered」= `Inner.instances` に登録済み・未除去 — runner registry slot を占有する集合で、`Failed` 未 `Destroy`・`Destroying` を含む。「live」= registered のうち converge 未収束・`Destroying` でないもの。判定表の全行は design `docs/designs/issue-117-bundle-child-process.md`「`Failed` 粒度・通知の確定契約」が正本。本 ADR は review 指摘で確定した inert 規則・`DestroyInstance` doorbell window・post-`Shutdown` non-event の行を織込済み）

### 2. `HostNotification::InstanceFailed` — per-instance fan-out を維持

bundle crash では**影響を受けた各 live インスタンスへ `InstanceFailed{handle, exit}` を 1 つずつ** emit する（`exit` は全て同一の `ChildExit` — 「同一子の死」の相関は通知列から推測可能）。現行の公開通知契約（per-instance handle・一度だけ・exit は子の終了状態）を変えない。instance スコープ収束（子は生存）は `exit = {code: None, signal: None}` で 1 通知 — 子 exit が未観測である事実を正直に運ぶ。`BundleFailed` 等の相関 variant は `#[non_exhaustive]` で後追加可能なため初期導入しない。

### 3. 隔離選択面 — `OutOfProcessSpec.isolation: OutOfProcessIsolation`（`Dedicated` / `Bundled`・既定 `Bundled`）

- `Dedicated`: 従来の per-instance spawn — `Failed` 粒度がインスタンス単位になる強隔離形として残す
- `Bundled`（既定）: 同一 route（`plugin_id`/`OutOfProcessSpec` 登録単位）のインスタンスを子共有 — spawn/メモリコストを削減するが子死亡は同居インスタンスを同時に `Failed` にする
- 選択は **登録時**（route 属性）— `HostCommand`/`Instantiate` 呼出し面を変えない（caller が層・隔離形を意識しない原則の維持）。enum は `#[non_exhaustive]`（将来の明示 pool キー・強化隔離 variant を additive に足す余地）
- 既定を `Bundled` とする根拠: Issue の主目的がコスト削減・API は `publish = false` の 0.1.0 未公開で互換契約の破壊にならない・`Failed` 粒度は本 ADR と enum doc で公開契約として明示する（静かな退化ではなく選択可能な契約）

## Considered Options

- **Option A — 純粋子スコープ**: 全ての `Failed` を子単位に統一。最も単純だが、帰属が確立する違反（live instance の未知 block seq）まで N-1 の健全インスタンスを殺す — bundle/dedicated の差を意味なくし隔離を過剰に退化させる → 基礎として採用し帰属可能ケースだけ exception を認める形に限定
- **Option B — 広い instance スコープ**: 「instance フィールドが読めればその instance だけ `Failed`」。`instance` 値自体が偽物・seq 空間が壊れたケースの帰属を過信する → 帰属不能は常に子スコープに倒すことで「正直に言える範囲」だけに限定した本決定を採用
- **Option C — 完全インスタンススコープ**: 子死亡でも他 instance を生かす → **物理的に不可能**（却下）
- **Option D — `Dedicated` 既定**: 安全性優先の既定として妥当だが、Issue の主目的（コスト削減）を opt-in に留め効果を限定する。0.1.0 未公開 API の仕様確定期であり `Failed` 粒度が本 ADR で明示されるため `Bundled` 既定を採用
- **Option E — instantiate 時 hint**: インスタンス毎に隔離を選ぶ → 登録時選択の設計原則（route claim）とずれ、`HostCommand::Instantiate` の公開面を壊す → 却下（spec 側 field で代替）

## Consequences

- `LifecycleState::Failed` の意味は「この論理インスタンスは二度と応答しない」で一貫 — **それが子の死を意味するか instance channel の死を意味するかは `exit` payload と通知列からのみ読める**。observer は `exit: {None,None}` を「子は生きているが instance は終端」の印として解釈できる
- bundle crash では observer が N 個の `InstanceFailed`（同一 `ChildExit`）を受け取る — 「1 通知 = 1 子死亡」ではなく「1 通知 = 1 インスタンス終端」の契約
- **`Bundled` は crash 隔離だけでなく liveness 隔離も trade する**: 1 インスタンスの plugin hang（`process`/hook が返らない）は子の単一スレッド `serve` ループを wedge させ、`command_timeout` 経由で同居**全** live インスタンスが `Failed` 収束する。共有 stdin/stdout は大きな frame（multi-MB `RestoreState` payload 等）を serialize し、他インスタンスの doorbell・コマンド応答を遅らせ得る（block 経路は RT leg の pass-through で継続するが、コマンド面の HoL blocking は実害）。強い liveness 隔離が要る route は `Dedicated` を選ぶ — 選択肢が残るのはこの trade が公開契約として明記されるからこそ
- inert 規則の副作用: registered-but-not-live・post-removal 宛の frame を違反扱いしないため、dishonest/破損した子が drain 済み領域へ frame を撃ち続けても検出されない（無害だが監査痕跡は残らない — 診断用の debug counter/log は実装詳細として許容、契約上の拒否にはしない）
- `OutOfProcessSpec` に pub field が 1 つ増え公開 API は additive に拡張（`#[non_exhaustive]` enum）。`InstanceLimit` の `HostError` 追加も同方針
- dedicated 経路の `Failed` 粒度は不変（1 子 = 1 instance で子スコープ収束がそのインスタンスのみに帰着するため、実装上同じコードパスで自然に 1:1 になる）

## Verification

- [ ] bundle crash（`kill_child`・crash fixture）で同居全 live インスタンスが `LifecycleState::Failed` へ収束し、各 handle に `InstanceFailed` が 1 つずつ届く（`exit` 同値）
- [ ] 帰属可能違反（live instance の `block_in_flight` に無い `BlockDone`）で対象 instance のみ `Failed`・兄弟は生存継続
- [ ] 帰属不能違反（live instance 宛の waiter なし `Reply`・`Reply.instance` 不一致・`instance` 範囲外/scope 不整合・EOF）で全 live インスタンス `Failed`
- [ ] registered-but-not-live 宛 frame（`converge_instance` 後の `BlockDone{i,s}`・waiter drain 後の `Reply{i}`）・`i ∉ instances` 宛完了が inert drop — 収束・通知・兄弟への影響なし、違反として記録されない
- [ ] `DestroyInstance` doorbell window — `Destroying` 化以後の新規 submit は `mark_failed` 合成、in-flight doorbell は通常完了、reply で entry 除去・残存 seq は `mark_failed` 解放・除去後の `(i,seq)` 完了は inert（runner 側対称契約: 未登録 key は flagged `Err(InvalidHandle)` で runner が死なない）
- [ ] `Shutdown` 送信後の子 frame・EOF が non-event — clean teardown（mirror `Destroyed`・通知なし）を変えない
- [ ] per-instance 収束の通知が `ChildExit{None,None}` で 1 度だけ届く
- [ ] `isolation: Dedicated` の route で 1 インスタンスのみ `Failed` になる既存契約が無退行
