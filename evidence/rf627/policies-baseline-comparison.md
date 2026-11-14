# rf627 — 三 repo `POLICIES.md` baseline 比較記録

Issue #627 の検証契約が照合対象とした三 repo の `POLICIES.md` について、読んだ版・取得方法・行単位の差分を記録する。`POLICIES.md` 本文はいずれの repo でも本 slice では変更していない（読み取りのみ）。

## 1. baseline の版と取得方法

| repo | checkout path | 旧 baseline（改訂前 commit） | 新 baseline（改訂 commit・照合対象） | 照合時点の main |
| --- | --- | --- | --- | --- |
| ruviform | `G:/repositry/audio-products/ruviform` | `70dfb2a5`（PR #633 merge = 改訂直前 HEAD） | `3b8b73224f0f792d70f260e764a7b5b8f4c0aa4e`（PR #634 `docs(policy): align virtual-first verification and Ruviform test harness`） | `3b8b7322` が HEAD |
| synvelis | `G:/repositry/audio-products/synvelis` | `f12aa66`（PR #448 merge = 改訂直前） | `8aa5cb0b5921b25c1dd990d646fed59539655088`（PR #449、同名 commit） | `b6a079a6086e84f4674c75e47fc5bec3dccb32cf`（PR #450 = player 変更、POLICIES.md 無関係） |
| audio-lens | `G:/repositry/audio-products/audio-lens` | `dafd895`（PR #211 merge = 改訂直前） | `eb62440cc6bab5625139d16c9b661fd878eeba88`（PR #212、同名 commit） | `eb62440` が HEAD |

取得コマンド（再現用）:

```sh
# ruviform（ローカル checkout がそのまま baseline）
cd G:/repositry/audio-products/ruviform
git show 3b8b7322 -- POLICIES.md

# synvelis / audio-lens（fetch 後に改訂 commit を指定）
cd G:/repositry/audio-products/synvelis && git fetch origin main && git show 8aa5cb0 -- POLICIES.md
cd G:/repositry/audio-products/audio-lens && git fetch origin main && git show eb62440 -- POLICIES.md
```

三 repo の改訂 commit はいずれも `POLICIES.md` のみを `+21/-10` で変更。共通節（§1〜§5）の diff は同一内容で、repo 間差異はヘッダ repo 名・§6 固有節・audio-lens の行末スタイルのみ。

## 2. 変更箇所の行単位対応（旧 → 新）

以下は三 repo 共通の変更。repo 固有の §6 変更は §4 に記録。

### メタデータ（先頭5行）

| 行 | 旧 | 新 |
| --- | --- | --- |
| `Effective:` | `2026-11-13` | `2026-11-13（OS別検証・CI）、2026-11-15（仮想優先受入）` |

注: ruviform/synvelis は行末が半角スペース2つ（Markdown 強制改行）。audio-lens は行末がリテラル `\`（`Status: Accepted\` 等）で、これは改訂前からの既存スタイルであり本改訂による退行ではない。

### §2 OS別検証レベル

| 箇所 | 旧 | 新 |
| --- | --- | --- |
| 見出し | `## 2. OS別検証レベル（2026-11-13決定）` | `## 2. OS別検証レベル（2026-11-13決定・2026-11-15仮想優先改訂）` |
| EXT-L3 定義行 | `実際に使用する物理Audio/MIDI機器、実VST3/CLAP/AU等のPlugin/Editor、Native Dock、外部実環境と結合する機能の実測` | `提供する外部接続能力について、本番OS Audio/MIDI API・実Plugin ABI/Editor・Native Dock・外部Provider Protocol等を通す結合検証。初回から仮想OS Endpoint・正規ABIのTest Pluginを標準利用し、物理機材・市販Pluginは通常必須としない` |
| OS別表 Windows EXT-L3 | `実際に提供・利用する機器・Plugin等に限り必須` | `実際に提供・利用するOS API・Plugin形式・Provider等に限り必須（仮想優先）` |
| OS別表 macOS/Linux | `利用要件発生までDeferred`（変更なし） | 同左（変更なし） |
| 補足 bullet「使わないDevice…」 | 変更なし | 変更なし |
| 補足 bullet「Windowsで利用する機能のGUI-L2/EXT-L3がNotRun/Fail…」 | `Fake/Headless/Offscreen成功は実GUI、Generic/Floating Editorは実Native Dock、Mock Pluginは実Plugin動作の代用にならない。` | `Fake/Headless/Offscreen成功は実GUIの代用にならず、Generic/Floating Editorは実Native Dockの代用にならない。Mock Pluginは実Plugin ABI動作の代用にならないが、正規ABIを実装した実バイナリのTest Pluginはその対象形式のHost/Process/Editor結合検証に利用できる。`（後段「Native Dockの画素合成など…Passにしない」は維持） |
| 補足 bullet「macOS/LinuxはOS-L1までが現行必須…」 | `実GUI/物理機器/実PluginをそのOSで利用する必要が生じた時点で…Requiredへ昇格し…` | `実GUI/本番OS Device API/実Plugin ABI等をそのOSで利用する必要が生じた時点で…Requiredへ昇格し…物理機材・市販Pluginの通常必須化は行わない。` |
| 補足 bullet「本方針は3OSを設計対象から除外…」 | 変更なし | 変更なし |
| **§2.1 新設** | （なし） | `### 2.1 仮想優先の受入と共通テスト基盤（2026-11-15決定）` + 7 bullet（後述） |

### §2.1 の7項目（新設・三 repo 同一文）

1. **初回から仮想優先** — 物理機材・市販 Plugin の入手を初回受入/通常回帰の必須条件にしない。決定的 Test Backend で基礎精度、Windows の外部機能は本番 OS API/実 ABI を通す仮想環境で GUI-L2/EXT-L3 を検証。環境未構築なら Required Case は `NotRun`。
2. **OS-L1とOS API L3を分離** — Ruviform 製 Test Backend/Virtual Clock/生成 PCM・MIDI/Fault Injection は OS-L1。WASAPI/WinMM を通す L3 では OS が列挙・Open できる仮想 Endpoint から本番 Backend へ入力し Format/Callback/Timestamp/Recovery を独立 Oracle で判定。後段直接注入の Mock を実 OS API の Pass としない。
3. **実Plugin ABIとMockを分離** — 正規 ABI の実バイナリ Test Plugin を本番 Host の探索/Load/Process/State/Unload 経路で実行。独立 Validator/Reference Host や仕様由来の期待値で検証。Fake Plugin は OS-L1 契約試験に限る。
4. **実GUIとNative Dockを分離** — Windows GUI-L2 は実 Window/入力/Focus/画素/Receipt。Native Dock は実 ABI Test Plugin の Native Editor（実 Child HWND 等）と本番 Window 合成を通し画素/Resize/Focus/Crash/RT を検証。Fake/Offscreen/Generic/Floating を Pass に読み替えず既知の不具合を残す。
5. **物理適合性の限界** — 仮想 Endpoint/Test Plugin の Pass は物理機材 Driver・ADC/DAC アナログ遅延・物理 Direct Monitoring・市販 Plugin 互換を保証しない。原則必須 Gate から外し、正式サポート宣言・報告不具合の調査時に Case を定義。問題は共通仮想 Fixture へ回帰再現を還元。
6. **共通化とDogfooding** — 共通 Test Runtime/仮想 Endpoint 制御/Test Plugin/実 GUI Runner/Console は Ruviform の再利用可能契約で構築。OS 固有 Driver/Kernel/Plugin SDK/GPUI 具象型は Adapter 内部に閉じる。Test-only バイナリは製品配布に含めない。AudioLens の f64/長尺/CLI と Synvelis の PCM/DAW/RT/Project/Plugin/GUI は各 Consumer の独立 Oracle で回帰検証し Pass を転用しない。
7. **外部Provider** — 実 Git/LFS/WebDAV 等は本番 Protocol・認証・永続化境界を通す制御可能な Test Server を使用可。単なる Mock では実 Network/Protocol の L3 を満たさない。

### §3 要求状態と検証結果の分離

| 箇所 | 旧 | 新 |
| --- | --- | --- |
| NotRun 例示 bullet | `実機がない、CI jobが未実行、テストがSkip/ignored、Featureが有効でない、といったケースはNotRunでありPassでもUnsupportedでもない。` | `Requiredの実OS Window/仮想Endpoint/実ABI Test Plugin環境がない、CI jobが未実行、テストがSkip/ignored、Featureが有効でない、といったケースはNotRunでありPassでもUnsupportedでもない。物理機材がなくても、仮想方式で受入条件を満たしたCaseを機材不在のみで未達にしない。` |
| 証拠 bullet | `証拠には対象Repo/SHA（Consumerの場合Ruviformのresolved SHA）、OS/Arch、Feature/テスト名、期待結果・観測結果、実行環境、CI Run URL/実機ログ等を記録する。` | `証拠には対象Repo/SHA（Consumerの場合Ruviformのresolved SHA）、OS/Arch/OS版、Feature/Case ID/テスト名、要求状態と実行結果、環境種別（deterministic backend / OS仮想Endpoint / 実ABI Test Plugin / 実Window / 物理機材等）、Driver・Endpoint・Plugin・FixtureのVersion/Hash、独立Oracle、期待結果・観測結果、CI Run URL/実OSログ等を記録する。`（末尾の「Jobが定義されていることやgreenであることだけでは個別CaseのPassを示さない」は維持） |
| 直交構造の定義 bullet 3本 | 変更なし | 変更なし（`Deferred`かつ`NotRun`は妨げない、`Required`の`NotRun`/`Fail`は受入未達、環境不在を技術的非対応と誤認しない、を維持） |

### §5 CI/検証運用

| 箇所 | 旧 | 新 |
| --- | --- | --- |
| Windows L2/L3 受入 bullet | `Windowsで提供・利用するGUI-L2/EXT-L3の実機受入は、Daily OS-L1 CIへの委譲では代替できない。macOS/LinuxのGUI-L2/EXT-L3は従来のDeferred方針を維持する。全体テストのローカル省略は、受入要件の削除や性能・精度・Consumer互換性の緩和ではない。` | `Windowsで提供・利用するGUI-L2/EXT-L3の実OS Window・本番OS API・実Plugin ABI受入は、Daily OS-L1 CIのFake/Headless成功で代替できない。Windows専用の仮想Endpoint/実Window/実ABI Test Pluginの手動・opt-in・定期実行レーンを使用できるが、RequiredのCaseがNotRun/Failなら未受入とする。macOS/LinuxのGUI-L2/EXT-L3はDeferred方針を維持し、ローカル全体テスト省略によって精度・性能・Consumer互換性を緩めない。` |

### §6 repo 固有節

| repo | 変更 |
| --- | --- |
| ruviform | 「実Window・Native Plugin Dock・Audio/MIDI実機等」→「実Window・Native Plugin Dock・**本番WASAPI/WinMMを通す仮想Audio/MIDI Endpoint・実ABI Test Plugin**等」。末尾に「共通Test Runtime/仮想Endpoint/正規ABI Test Plugin/GUI Test Runnerおよびテスト用GUIはRuviform自身の汎用契約で構築し、独立Oracleと両Consumer回帰で自己検証バイアスを抑える。OS固有のDriver/KernelはPrivate Adapterに閉じ、Test-onlyバイナリを製品配布対象に含めない。」を追加 |
| synvelis | 「録音/モニター/**実MIDI**」→「録音/モニター/**OS MIDI**」。「初回からRuviformの実OS仮想Audio/MIDI Endpointと正規ABIのTest Plugin/Native Editorを使用できるが、実Native Dockに画素・Focus・入力の未解決不具合がある場合、当該提供機能を未受入として追跡する。」「Ruviform共通Test Runtime/仮想Endpoint/Test Plugin/実GUI Runnerを使い、Recording PCM/RT/Project、MIDI、Plugin State、DAW GUI/UXはSynvelis独自Oracleで受入する。Test RuntimeのPassを製品Passへ自動転用しない。」を追加 |
| audio-lens | 「物理Audio/MIDI/実Plugin等を**製品機能として提供しない限り**そのEXT-L3を一律要求しない」→「Audio/MIDIの本番OS APIやPlugin Hostを製品機能として提供しない限りそのEXT-L3を一律要求しない。**提供する場合も物理機材・市販Pluginを通常必須とせず、対応する仮想L3で検証する。**」「Ruviform製の決定的PCM/Clock Fixtureと実GUI Runnerを再利用しつつ、f64/長尺/PTS/欠測/CLI/可視化の期待値・性能はAudioLens独自Oracleで判定する。SynvelisのPassやRuviform Test Fixture自身の結果を流用しない。」を追加 |

## 3. 非変更箇所（三 repo 共通で維持）

- §1 方針の適用と責務（repo 責務分離、「共通基盤変更は両Consumerの互換性・精度・性能・RTを独立評価」）
- §2 OS-L1・GUI-L2 の定義行、macOS/Linux Deferred、「使わないDevice・Plugin形式・Host mode・特殊FS等の全組合せは必須にしない」、「本方針は3OSを設計対象から除外する判断ではない」
- §3 の要求状態4値・実行結果4値の直交定義、「`Deferred`かつ`NotRun`は妨げない」「`Required`の`NotRun`/`Fail`は受入未達」「環境不在を技術的非対応と誤認しない」
- §4 着手・CI・Issue・PR運用の全項
- §5 のローカル検証≠統合受入、fail-closed 集約、「CIに実装されていない検証を委譲済みと扱わない」、新規 Workflow 無条件増設の禁止

## 4. repo 間残差分

| 差分 | 内容 | 評価 |
| --- | --- | --- |
| ヘッダ repo 名・Scope 行 | `POLICIES.md — ruviform` / `— synvelis` / `— audio-lens`、Scope の repo 名 | 意図的。問題なし |
| §6 固有節 | repo 別の Oracle 領域・固有ルール | 意図的。契約設計 §5 のマトリクスと整合 |
| audio-lens 行末スタイル | メタデータ行が `Status: Accepted\` のようにリテラル `\` 終端（他2 repo は行末スペース2つ） | **既存差異**（改訂前から）。本 slice は POLICIES.md 非変更のため修正せず、残件として記録 |
| `environment_kind` 値の表記 | ポリシー本文は「deterministic backend / OS仮想Endpoint / 実ABI Test Plugin / 実Window / 物理機材等」の和英混在列挙（末尾「等」で開集合） | 契約 schema は `deterministic-backend`/`virtual-os-endpoint`/`test-plugin`/`real-window`/`physical-device` の kebab-case 閉集合を canonical として定義。意味論は一致、記録形式の統一は契約側で吸収。ポリシー本文の改訂は不要 |

## 5. 照合の限界

- Synvelis は照合後に main が `b6a079a`（PR #450、player の RCD-02 変更）へ進行したが、`POLICIES.md` に後続差分がないことを `git show 8aa5cb0` の diff 範囲と log で確認済み。baseline として `8aa5cb0` の版を採用した。
- audio-lens/ruviform の main HEAD はそれぞれ `eb62440`/`3b8b7322` で、改訂 commit が最新の `POLICIES.md` 変更である。
- 本記録は `git show` の diff に基づく行単位整理であり、ポリシー本文の**意味論的正しさ**の審査ではない（それは review-protocol と三 repo レビューの責務）。
