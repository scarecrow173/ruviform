---
id: "DESIGN-034T9VaXD2KXqhGxaHawyH"
type: "design"
status: "approved"
title: "Issue #390 Driver Contract Lab — boundary guard 検査設計・registry schema・実行接続点"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/390"
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-ui-test/fixtures/driver-lab"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_boundary.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_lab.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/src/bin/gui_boundary_guard.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_boundary.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_contract.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-ui-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "target/driver-lab/evidence.jsonl"
  implements:
    - "docs/specs/issue-390-driver-contract-lab.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-312-test-gui-host-lifecycle.md"
    - "docs/designs/issue-182-dock-tree.md"
    - "docs/designs/issue-181-multi-window.md"
    - "docs/designs/issue-156-gui-host-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/386"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-390-driver-contract-lab.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-390-driver-boundary-lab.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #390 Driver Contract Lab — boundary guard 検査設計・registry schema・実行接続点

## Context

Spec: `docs/specs/issue-390-driver-contract-lab.md`。
Inventory: `docs/discovery/issue-390-driver-contract-lab-inventory.md`。

main(`355b7a1`)で GUI toolkit 依存を持つのは `ruviform-ui-webview`
のみ（`webview` feature 下の optional dep）。
`consumer-verify.yml` の `GUI-CONTRACT` 行は「RF #390 が Owner —
検査 task 未接続」と `not_run` 固定。本設計は (a) 依存グラフ +
public signature の 2 系統検査を行う Guard、(b) UI-DVR case
manifest と binding モデル、(c) Native Component Lab story
registry、(d) 正規 mise 入口を定義する。

## Scope

### In Scope

- `ruviform-ui-test` への `driver_boundary`/`driver_lab` 2 module・
  `gui-boundary-guard` bin・`fixtures/driver-lab/`（ruleset/cases/
  stories/正負 fixture）・2 test file の追加。
- `mise.toml` への `check-ui-driver-boundary` /
  `test-ui-driver-contract` タスク追加。
- 付随する README / development-environment.md / evidence 記録。

### Out of Scope

- spec の Out of Scope をそのまま継承（型定義・GPUI 実描画・
  real-plugin・Consumer 側実装・CI workflow 変更・rustdoc JSON）。

## Components and Boundaries

```
crates/ruviform-ui-test/
├── src/
│   ├── driver_boundary.rs        # ruleset types + 2系統検査 engine（pure: JSON/AST in → Report out）
│   ├── driver_lab.rs             # cases.json / stories.json の型 + loader + 整合性検査 + evidence 行
│   └── bin/gui_boundary_guard.rs # CLI: cargo metadata 実行 → engine 呼出 → verdict 出力
├── fixtures/driver-lab/
│   ├── boundary.json             # 境界 ruleset（唯一の正本）
│   ├── cases.json                # UI-DVR-01..08 manifest（semantic truth）
│   ├── stories.json              # Component Lab story registry（seed）
│   └── boundary/
│       ├── positive/{metadata.json,src/**}
│       └── negative/<case>/{metadata.json|src/**}
└── tests/
    ├── driver_boundary.rs        # 正負 fixture による guard 検証
    └── driver_contract.rs        # manifest 整合性 + 実行可能 UI-DVR fixture
```

配置理由: `ruviform-ui-test` は GUI 系 test-only crate であり、
RF386 の `FakeDriver` も同 crate に着陸予定。guard/registry を同じ
crate に置くと契約到着時の同一 meaning fixture 接続が自然。
`dummy_app` bin 先例があるため `src/bin` で CLI を提供する。

### 判定語彙（spec R6）

`pass` / `fail` / `not_run` / `unsupported` / `blocked` の 5 値。
- `not_run`: capability はあるが検査入口・契約が未接続。
- `unsupported`: その layer/OS/capability では実行不能。
- `blocked`: 上流の公開契約到着待ち（Owner Issue 必須）。

## Data and Control Flow

### A. 境界 Guard

入力: `boundary.json` + `cargo metadata --format-version 1` の
JSON（実 workspace）または fixture 同梱の擬似 metadata JSON
（検証用）。出力: Report（check 別 findings + verdict）。
`gui-boundary-guard` は `env!("CARGO") metadata` を実行して
JSON を得る（`--manifest-path`/`--metadata` override で fixture
にも適用可能）。exit: 0=pass / 1=violation / 2=tool error。

#### boundary.json schema

```jsonc
{
  "schemaVersion": 1,
  "ruleset": "rvf.driver-boundary/v1",
  "forbiddenDepRoots":    [ "gpui", "gpui-*", "wry", "tao", "winit", "dpi", "raw-window-handle", "web-view", "webkit*", "gtk*", "gdk*", "egui", "eframe", "iced*", "slint", "fltk", "druid*", "vizia", "dioxus*", "azul", "tauri*" ],
  "forbiddenPathRoots":   [ "gpui", "gpui_component", "gpui_base", "gpui_kit", "gpui_toolkit", "gpui_audio_kit", "gpui_design", "wry", "tao", "winit", "dpi", "raw_window_handle", "web_view", "webkit2gtk", "gtk", "gdk", "egui", "eframe", "iced", "slint", "fltk", "druid", "vizia", "dioxus", "azul", "tauri" ],
  "allowlist": [
    { "package": "ruviform-ui-webview", "role": "driver-adapter",
      "allowedDeps": ["wry", "tao", "dpi", "http", "rust-embed"],
      "viaFeatures": ["webview"], "required": true,
      "reason": "Issue #156 accepted WebView driver adapter" },
    { "package": "ruviform-ui-gpui", "role": "driver-adapter",
      "allowedDeps": ["gpui"], "viaFeatures": ["gpui"], "required": false,
      "reason": "Issue #386 GPUI standard driver（未 merge・到着時に検査開始）" }
  ]
}
```

- `allowedDeps` は `forbiddenDepRoots` に一致する名前のみ有効
  （`http`/`rust-embed` は禁止 root ではないが adapter 固有の
  backend 依存として grant 内で宣言を許容 — 禁止 root 以外の
  allowedDeps は検査対象外。設計を単純にするため allowedDeps は
  forbidden root 名のみ必須、それ以外の記述は無視される）。
- `role`: `driver-adapter` | `composition-root` | `driver-extension`。
- `required: false` は package 不在時に skip（未 merge driver）。

#### 検査 1: dependency graph（check id = `dep-graph`）

workspace member 各 package P について:

1. **direct-dep** (`forbidden-dep-direct`): P の manifest
   dependency（kind normal/build/dev 全て）の名前が forbidden
   root に一致 → P が非 allowlist または dep が grant.allowedDeps
   に無ければ violation。allowlist 内でも `optional != true` は
   `forbidden-dep-not-optional`。
2. **feature** (`forbidden-dep-feature`): P の `features` 表を
   展開し、feature 値 `"dep:<name>"`（推移的に feature→feature
   連鎖も辿る）が forbidden dep を有効化する場合、allowlist の
   `viaFeatures` に無ければ violation。`dep:` 形式以外の
   `"crate/feature"` 値で他 crate の toolkit 側 feature を
   有効化する場合も同様に検査（allowlist 外の crate 間 feature
   連鎖は resolve 側検査で担保）。
3. **transitive** (`forbidden-dep-transitive`): resolve graph を
   起点 P から BFS。辺の traversal 規則:
   - `cargo metadata` の resolve graph は未 activation の
     optional dep を含まない — resolve に現れる辺は全て
     activation 済み = 無条件到達として扱う（default feature
     で optional dep が有効化される経路も resolve に出るため
     誤って conditional 扱いしない）。
   - 起点 P からは normal+build+dev 辺を全て辿る（自身の test
     lane も保護対象）。それ以外の node からは normal+build の
     み（dev-dep は推移しない cargo 意味論）。
   - allowlisted adapter の outgoing 辺は展開しない（grant の
     sanctioned channel）。allowlisted root 自身の grant 済み
     dep 辺も到達判定から除外。
   - forbidden root package に到達 → violation（witness path 記録）。
   - 検査範囲は workspace member のみ（`workspace_members` 起点。
     member 外の path dep package は到達先として辿るが
     direct/feature 検査は行わない）。
4. **conditional** (`forbidden-dep-conditional`): optional 辺を
   1 本以上含む経路での forbidden 到達。resolve graph は未
   activation の optional dep を省略するため、optional 辺の
   target は manifest 宣言から解決する:
   - workspace packages で dep `name` 一致 → resolve unit。
   - なければ `dep.path` 先の manifest を `cargo metadata
     --no-deps` で読み依存を再帰走査（manifest-only unit）。
   - registry dep の未 activation target は解決不能で skip
     （既知の限界 — 検査限界節に明記）。
   - optional 辺を含まない経路のみの forbidden 到達は
     transitive 側が報告（重複しない）。
5. **dep-activation** (`forbidden-dep-adapter-feature` /
   `adapter-consumer`): dependency 宣言の `features` /
   `uses_default_features` 値が allowlisted adapter の
   `viaFeatures`（toolkit 有効化 feature 連鎖）を有効化する
   場合 → composition-root/driver-extension grant を持たない
   member なら violation。非 grant member の allowlisted
   adapter 依存自体も `adapter-consumer` violation。
4. **allowlist 整合** (`allowlist-*`):
   - `required: true` で package 不在 → `allowlist-missing`。
   - grant.package が allowlist 対象でない package に同名 grant
     → n/a（name 一致のみ）。
   - grant.allowedDeps のうち manifest に未宣言の名前 →
     `allowlist-grant-unused`（warning 級: findings 記録・verdict
     には `strict: true` の場合のみ影響。本 ruleset では strict）。
   - allowlisted adapter の dep 宣言辺は dep-graph 検査で
     exempt されるが、signature 検査では **exempt しない**。
     adapter 内部で backend を `use` するのは私的利用（`pub`
     item でないため検出されない）だが、adapter の **公開 API
     に backend 型が出ること自体が漏出** — grant は dep 宣言の
     許可であり、API 漏出の許可ではない（RF386 の設計でも
     GPUI 型は adapter の public API に出さない契約）。
     `ruviform-ui-webview` も同様 — 現行の公開面は
     `ruviform-ui` 契約型のみで backend 型を出さない。

#### 検査 2: public signature（check id = `signature`）

全 workspace member の lib target を走査（example/bin/test
target は検査対象外 — toolkit 利用は dep 検査で捕捉される）。
allowlisted adapter も対象 — adapter の **pub** signature に
backend 型が出ること自体を漏出として検出する（private 利用は
`pub` item でないため検出しない — `ruviform-ui-webview` の
`wry`/`tao` 私的利用は合法）。

exposure モデル（2-pass）: pass 1 で package の公開 type 名を
収集し、pass 2 で module visibility / `pub use` re-export 連鎖を
追跡して「crate 外に露出する item」のみを走査する。private mod
内 item は `pub use` re-export chain 経由で到達可能なときのみ
露出と見なす（`pub use` なし private mod の pub 表記 item は
crate 外に出ないため検出しない — 誤検出防止）。

- module tree 走査: crate root（metadata `targets[kind∋"lib"]`
  の `src_path`）から `mod` 宣言を辿る。`mod foo;` → `foo.rs` か
  `foo/mod.rs`。`#[path = "..."] mod foo;` は宣言 file 相対で
  解決。inline `mod foo { .. }` は本体を辿る。各 file の走査は
  到達 exposure（Full / 限定的 Names）を伴い、Names は union
  merge して exposure が広がった場合のみ再 scan する。
- alias 解決: `use wry::WebView as W;` のような alias は
  scope 内で `W` → `wry` に解決して検出（alias 迂回を塞ぐ）。
- cfg-gated item は保守的に走査する（cfg 評価はしない —
  `#[cfg]` 付き item も public 露出の可能性として検査。条件
  解釈の限界は検査限界節に記録）。
- flag 対象: `pub`（unrestricted）item の signature に現れる
  forbidden path root（`syn::Path` の先頭 segment ident）。
  - `pub fn`/`pub async fn`: inputs・output・generic bounds・
    where clause・const generic の型。
  - `pub struct`: `pub` field の型。`pub enum`: variant field
    の型（variant は enum の公開性を継承）。
  - `pub union`: `pub` field の型。`pub type`: aliased type。
  - `pub use`: use tree の path root（glob は root のみ評価）。
  - `pub trait`: supertraits・assoc fn sig・assoc type bound/
    default・assoc const 型。
  - `impl` block: inherent impl の `pub fn` sig。trait impl の
    assoc const/type/fn の型（trait impl は trait 側公開性に
    従うため全走査）。`pub extern crate`/`pub static`/`pub const`
    の型。
- `pub(crate)`/`pub(super)`/`pub(in)` item は対象外（crate 境界
  を越えない。`pub use` は `pub(crate)` item をより広く公開
  できない Rust 意味論に依拠）。
- finding record: `{code:"signature-leak", package, file, line,
  itemKind, itemName, pathRoot, path}`。

#### 限界（ruleset + test-spec に明記 — spec R4）

1. glob re-export（`pub use m::*`）は root のみ評価、再 export
   先の全展開は名解決なしには不可 → private-mod pub item の
   全走査で穴を塞ぐ保守設計。
2. local mod が forbidden root と同名の場合は誤検出となり得る
   （保守側。rename を要求）。
3. macro 内の型出現・生成コードは未走査。
4. bare OS handle 型名（`HWND` 等の import 後の裸名）は
   path root 検査の対象外 — 対応する OS/FFI crate 依存は
   dep-graph 側で捕捉（`windows`/`objc`/`x11` 系を将来
   必要に応じ forbiddenDepRoots に追加できる形）。
5. dev-dep 経由の private 利用は public API 漏出ではないが
   dep-graph 側で protected crate に対しては禁止。
6. conditional 到達の manifest 走査: registry dep の未
   activation target（`dep.path` を持たない optional dep）を
   `cargo metadata --no-deps` で解決することはできないため
   skip する — workspace 外 package への optional 辺は
   witness path を取れない（feature 有効化検査は
   `forbidden-dep-feature`/`forbidden-dep-adapter-feature` 側
   で担保する設計のため実害は限定的）。
7. cfg-gated item は保守的に全走査する（`#[cfg]` の条件評価は
   しない）。cfg で条件付きの forbidden signature は検出側に
   寄る — 誤検出は finding detail で確認できる。
8. signature 検査は `pub`（unrestricted）基準の exposure
   tracking で「crate 外に露出する item」を判定するため、
   `pub use` の glob 展開や複雑な re-export 連鎖では実際の
   rustc 公開性と誤差が生じ得る（保守側 = 過検出方向）。
9. 検査範囲は workspace member — `workspace_members` に無い
   package（外部 path dep 等）は direct/feature 検査対象外
   （到達経路としては辿る）。

#### Report 形式

```jsonc
{ "schemaVersion":1, "report":"rvf.driver-boundary/v1",
  "verdict":"pass|fail",
  "workspaceRoot":"...", "metadataSource":"cargo-metadata|fixture",
  "checks":[
    {"id":"dep-graph","status":"pass|fail","findings":[{code,package,dep|path,kind,detail}...]},
    {"id":"signature","status":"pass|fail","findings":[{code,package,file,line,pathRoot}...]},
    {"id":"allowlist","status":"pass|fail|warn","findings":[...]}
  ] }
```

### B. Case manifest（cases.json）

```jsonc
{
  "schemaVersion": 1, "manifest": "rvf.driver-lab.cases/v1",
  "vocabulary": ["pass","fail","not_run","unsupported","blocked"],
  "layers": ["boundary","fake","gpui","real-os","headless-ts"],
  "requiredCases": ["UI-DVR-01",...,"UI-DVR-08"],
  "cases": [{
    "id": "UI-DVR-05", "title": "Dock 2-phase move/rollback",
    "oracle": "<semantic expectation 1文>",
    "surfaces": ["window","dock"],
    "bindings": [
      {"layer":"fake","status":"pass",
       "mechanism":{"kind":"cargo-test","package":"ruviform-ui","target":"dock"}},
      {"layer":"fake","status":"pass",
       "mechanism":{"kind":"cargo-test","package":"ruviform-ui-test","target":"driver_contract","filter":"ui_dvr_05"}},
      {"layer":"gpui","status":"not_run","owner":"https://github.com/.../issues/386",
       "rerun":"ruviform-ui-gpui merge 後、同一 scenario を WindowDriver 経由で実行"},
      {"layer":"real-os","status":"not_run","owner":"https://github.com/.../issues/313"}
    ],
    "evidence": "target/driver-lab/evidence.jsonl"
  }]
}
```

整合性ルール（`driver_lab::validate` + consistency test）:
- id 一意・`UI-DVR-\d{2}` 形式・layers/vocabulary は宣言集合内。
- `status=="pass"` の binding は `mechanism` 必須（`cargo-test`
  `{package,target[,filter]}`、`mise-task`、`manual`）。
- `status!="pass"` は `owner`（Issue URL）+ `rerun` 条件必須。
- `mechanism.kind=="cargo-test"` は package/target が実在する
  （workspace metadata と tests/*.rs の存在で検証）。
- `filter` があれば対象 test file の source に同名 `fn` が存在
  （文字列一致ではなく、`cargo test -- --list` 実行をせずに検証
  できる静的な存在確認）。

### C. Story registry（stories.json）

```jsonc
{ "schemaVersion":1, "manifest":"rvf.driver-lab.stories/v1",
  "themes": ["synvelis-dark","audiolens-dark"],
  "stories":[{ "id":"LAB-SV-FADER-DOCK", "title":"Synvelis Fader + Tabs + Dock slice",
    "components":["Fader","Tabs","Dock"], "theme":"synvelis-dark",
    "scaleFactor":[1.0,2.0],  // 2 値必須
    // theme は scalar — Product Theme 1 つを選ぶ。matrix
    // （2 theme × 2 scaleFactor）は manifest.themes 宣言 +
    // stories 全体で担保する。
    "states":["idle","focus","drag","commit","stale","invalid"],
    "drivers":{"fake":{"status":"blocked","owner":".../issues/387"},
               "gpui":{"status":"blocked","owner":".../issues/386"},
               "real-os":{"status":"unsupported","owner":".../issues/313"}},
    "oracle":"<semantic expectation>", "evidence":"target/driver-lab/evidence.jsonl"}]}
```

### D. 実行可能 fixture（driver_contract.rs）

- `ui_dvr_01_boundary_*`: ruleset 自体の自己整合（allowlist が
  現行 workspace と整合）— guard 実実行は
  `check-ui-driver-boundary` task。
- `ui_dvr_05_dock_*`: `ruviform-ui` `DockWorkspace` 公開 API による
  2-phase move/rollback/ViewInstanceId 安定性の semantic fixture
  （tests/dock.rs と同契約を registry の oracle と照合）。
- `ui_dvr_07_capability_*`: `HeadlessIo`/`FakeHost` の
  `Capabilities`/`DeniedCapabilities` honest-reporting fixture。
- `ui_dvr_08_teardown_*`: `UiRuntime` + `FakeHost` で
  CloseRequested→job cancel→Destroyed 終端・subscription cleanup。
- `manifest_consistent`: cases.json/stories.json の validate。
- evidence: env `RUVIFORM_DRIVER_LAB_EVIDENCE` があれば各行を
  `{manifest:"rvf.driver-lab.evidence/v1",subject,layer,expected,
  observed,mechanism,detail}` の JSONL で append。`expected` は
  manifest 宣言 status、`observed` は fixture の実実行結果 —
  乖離は呼び出し側 assertion。相対 path は workspace root から
  解決（`cargo test` の CWD は package root）。env 未設定では
  write しない（`cargo test` を純粋に保つ）。

### E. mise tasks

```toml
[tasks.check-ui-driver-boundary]
run = "cargo run -p ruviform-ui-test --bin gui-boundary-guard -- --workspace-root ."
[tasks.test-ui-driver-contract]
run = "cargo test -p ruviform-ui-test --test driver_boundary --test driver_contract"
env = { RUVIFORM_DRIVER_LAB_EVIDENCE = "target/driver-lab/evidence.jsonl" }
```

## Non-Functional Constraints

- Guard は `cargo metadata` を `--frozen`/`--offline` 指定なしで
  実行（lockfile 整合は `mise run check` 等の正規 lane が担保）。
  fixture 検証は `--metadata` 注入で cargo 不要。
- 新規依存: `cargo_metadata`（metadata JSON の型付き parse）、
  `syn`（`full`+`visit`。signature 走査）。両者とも
  ruviform-ui-test の通常 dep（test-support crate のみ使用で
  本番 graph に入らない）。理由を Cargo.toml に記録。
- `#[non_exhaustive]` 方針・`#![warn(missing_docs)]`・
  日本語コメント文体は既存 crate に従う。
- `cargo test --workspace` で全 test が実行される（ignore なし、
  feature gate なし — headless で完結）。

## Risks and Trade-offs

- **signature 走査の保守設計（private mod 全走査）**: 実害のある
  誤検出は forbidden root と同名の local mod のみ — dep 検査で
  検出済みの状況と重複するため実害少。glob re-export の完全解決は
  断念し限界として明記（rustdoc JSON は将来拡張）。
- **metadata の optional/feature 判定**: `resolve` は feature 解決
  済みの有効 graph を返さないため、本実装は「宣言された optional
  辺 + features 表」から conditional 到達を自前で推定する。
  cargo の resolver 完全再現ではない（`default-features`/
  feature unification の edge case は限界記述）。
- **syn + cargo_metadata の dep 追加**: test-only crate への追加で
  本番 graph に波及しない。`syn` は既存 proc-macro dep 木に存在。
- **UI-DVR の未契約ケース**: `not_run`/`blocked` 記録で
  manifest を先に固定するため、契約 merge 時に binding を足すだけで
  回帰接続できる。

## Deferred Design Concerns

- rustdoc JSON/public-api 移行（spec の Deferred を継承）。
- `gpui` binding の実実行（`ruviform-ui-gpui` 到着後）。
- `driver_lab::run` による binding 自動実行ランナー（現状は
  binding を task/test に静的 bind。需要が出たら runner を検討 —
  YAGNI で現在未実装）。

## Verification Notes

- `mise run check-ui-driver-boundary` → live workspace で
  `verdict:"pass"`、webview 以外に GUI 依存なしを確認。
- `mise run test-ui-driver-contract` → boundary 正負 fixture +
  contract tests pass。
- `mise run verify-ci` → fmt-check/lint-ci/test-ci 全 pass。
- 手動 negative 実演: 一時的に common crate へ wry dep を足すと
  guard が fail することを fixture テストと併せて確認済みにする
  （実演は fixture で代替し、実 workspace を汚染しない）。
