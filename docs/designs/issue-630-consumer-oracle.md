---
id: "DSN-630consumerOracle01"
type: "design"
status: "approved"
title: "Issue #630 AC7 consumer-oracle slice — 設計"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-consumer-oracle.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-consumer-oracle.md"
  implements:
    - "docs/specs/issue-630-consumer-oracle.md"
  implemented-by:
    - "docs/test-specs/issue-630-consumer-oracle.md"
  depends-on:
    - "docs/specs/issue-630-consumer-oracle.md"
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "POLICIES.md"
  related:
    - "docs/test-specs/issue-630-consumer-oracle.md"
    - "docs/discovery/issue-630-consumer-oracle.md"
    - "evidence/rf630/consumer-oracle-verification.md"
    - "evidence/rf630/consumer-oracle/manifest.json"
---

# Issue #630 AC7 consumer-oracle slice — 設計

## 設計判断

### 照合形の決裁（全体構成）

AC7「照合」を ruviform 側で完結する 3 部構成に分解した:

1. **fixture contract manifest** — consumer oracle が fixture に
   要求する contract 面（descriptor identity・param・DSP・state・
   fault・editor token）の正本表。
2. **consumer oracle 対応表** — consumer Issue の独立 oracle
   anchor と fixture の写像。manifest 内 `consumerOracleMap`。
3. **verdict 表** — contract-shape 一致行と external-owner/
   NotRun 行の分離記録。

「照合」を ruviform 側の実走ではなく「consumer oracle が必要と
する contract 面が、正本に宣言済みであることの確認」と定義した
のは、consumer 実走 legs が各 consumer Issue の AC/lane 責務で
あり、ruviform 側から肩代わりすると consumer Issue 側の受入と
二重化するため（探索記録の採用解釈）。責務境界の明記は spec の
解釈節に記録する。

### manifest は pin 方式（値の正本 = crate README）

fixture contract の値の正本は両 crate の README とする。
`test-only` 宣言・Independence boundary・Realtime contract を既に
持つ README は suite の利用者が読む面であり、contract 値を
manifest 側に複写すると、fixture 変更時に manifest との乖離が
構造的に検出できない二重正本になる。

したがって manifest の contractFaces は `(README path, 節 anchor)`
の pin のみ保持し、pin 先節が「何を宣言するか」の scope 記述
（`declares`）だけを添える。pin の版固定は README ファイルの
`sha256` で行う。sha256 乖離は pin の自動失効ではなく再照合
トリガとする — README 編集のたびに manifest 更新を強制すると
docs-only 変更にも evidence 再発行が要り、変更コストに見合わない。

例外として identity key（VST3 class 名・CLAP plugin id）だけは
manifest に記す。これらは contract law ではなく consumer が
fixture を join するキーであり、law 複写の禁止範囲に含めない。

### contract 面の粒度

consumer oracle が参照する面を、README の既存節構成に合わせて
`paramSurface`/`dspLaw`/`faultModes`/`stateBlob`/`presetSurface`/
`editorSurface`/`popup`/`eventTokens`/`midiTransport`/`voiceLaw`/
`transportClick`/`spawnRejection`/`topology`/`rtContract` に分けた。
1 class・1 ABI に対し「どの節が何を宣言するか」が一意に解ける
粒度で、fixture class × contract face が対応表の最小単位になる。

### README の contract 面補完

pin 先として使える contract 面が README に欠けていたため、先に
README を整えた（lease 内の README 更新権を使用）:

- CLAP README が `plugin_count() == 5` で `test_spawn_fail` を
  列挙していなかった（実装は 6 descriptor）。表に加えて訂正。
- 両 README に `test_instrument`/`test_spawn_fail` の semantics
  節がなかった — fixture class 全員が pin 可能な節を持つよう
  追加。
- fault mode の 8 帯域表は spec・module doc にのみ存在した —
  README `test_fault` 節に表として追加（normalized arm 値付き）。
- VST3 editor event token（`f:`/`kd:`/`ku:`/`po:`/`pc:` と
  window title の event log 形式）を `test_editor` 節に表として
  追加 — source 上の観測面を contract として明文化。
- preset 面（VST3 の `IUnitInfo`/`IProgramListData`/`IUnitData`
  + `"Factory"` program list、CLAP の `clap.ext.preset-load` +
  `clap.factory.preset-discovery` provider）を `test_stateful`
  節に追加 — state quality oracle が参照する面で README に
  未記載だった。
- CLAP の features 記述を訂正（`audio-effect`+`stereo` は
  instrument 以外。instrument は `instrument`/`synthesizer`/
  `stereo`）。

値の記述は実装の read-only 実測に基づく（discovery doc に記録）。

### verdict 語彙

POLICIES §3 の要求状態（Required/Deferred/NotApplicable）と実行
結果（Pass/Fail/NotRun/Unsupported）の別軸をそのまま使い、
manifest の `dispositions` で本 slice 固有の中間語を 3 種だけ
定義する:

- `headless-contract-match` — manifest pin と consumer oracle
  anchor の contract-shape 一致を ruviform 側 headless で照合した
  行。consumer 実走の成否とは別の記録軸。
- `external-owner` — consumer Issue・別 Issue・後続 slice が所有
  する leg。本 manifest は verdict を付けない。
- `gate` — 外部修正 Issue（`scarecrow173/ruviform#532`）の着地を
  待つ受入 gate。本 slice は判定しない。

#631 real-dock lane の語彙・契約語彙は #631 側正本に揃える
（探索記録 relations[0] の指示どおり、本 slice では語彙を新設
しない）。

### 対応表の粒度

`consumerOracleMap` の 1 行 = 1 consumer Issue。行は
`oracleAnchors`（実測確認した anchor の path+symbol+repo SHA）、
`issueNamedFixtures`（Issue 本文が名指す fixture）、
`primaryFixtures`（oracle が実際に消費する fixture — 対照 fixture
を含めた運用集合）、`contractFacesUsed`（消費する contract 面）、
`contractShape`・`consumerRun` の 2 軸 disposition で構成する。
`issueNamedFixtures` と `primaryFixtures` を分けるのは、Issue 本文の
fixture 列挙と oracle の実 needs が一致しない場合（sv#128 が
`test_fault` を名指すのは state capability の対照であり、
state blob の消費ではない）を正直に表すため。

### anchor の記録方式

consumer 側 anchor（関数・型・evidence 表）は read-only 参照で
実測し、path・symbol・測定時の consumer repo SHA を記録する。
探索記録と差異があれば捏造せず実測値を記録する方針 — 今回の
測定では探索記録どおり全 anchor が存在した（discovery doc）。

### dep 非汚染の証明形

「新依存を強制しない」の証明は 2 半分で構成する:

- 構造半分: 両 test crate が test-only 宣言済みで production
  crate が test fixture crate に非依存（README の Independence
  boundary・dev-dep のみ）。
- 実測半分: 両 consumer `Cargo.lock` への fixture crate 名 grep
  で非含有を実測（exit 1 = 期待値）。コマンド・対象 repo SHA・
  exit code を evidence に記録。

構造半分だけでは「consumer が今後入れる可能性」を否定できず、
実測半分だけでは「現状」のみを示す — 両方を記録して現在の
非汚染と強制でないことの構造的理由を分離する。

## 代替案と却下理由

- **manifest に contract 値を複写する**: 機械可読性は上がるが
  README との二重正本になり fixture 変更時の乖離を検出できない。
  pin + sha256 + drift 再照合にした。
- **verdict を manifest に書かない**: 対応表と verdict を別物に
  すると「どの行が headless 照合済みか」を manifest だけでは
  再現できない。disposition は対応表行の属性として manifest に
  持たせ、実測の実行記録（コマンド・exit code）は
  verification evidence に分離した。
- **docs index の登録**: `docs/**/README.md` は Coordinator が
  統合時に整備する（lease 対象外・worker は触れない）。
