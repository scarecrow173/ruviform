# Issue #262 GPUI Native First採用Gate verdict — 証拠照合・判定試行記録（2026-11-15）

`docs/tasks/issue-262-gpui-verdict.md` の文書・証拠スライスで試行した手法と結果の記録。コード変更を伴わないため、実験は「証拠の照合方法・verdict 分類・文書規約の適用」に限定される。正本の判定は `evidence/rf262-gpui-verdict/verdict-matrix.md`・`report.md`、契約は `docs/designs/issue-262-gpui-verdict.md`・`docs/test-specs/issue-262-gpui-verdict.md` を参照。

## 試行 1: 先行 PoC 証拠の SHA 照合と ancestor 判定

- **目的**: `evidence/rf388-*`/`rf389-*`/`rf313`/`rf243` の先行証拠を、HEAD `7aab6302` の証拠として使えるか判定する。
- **試行**: 各 report 記載の実行 SHA を `git merge-base` で HEAD の ancestor か確認し、差分が契約層に影響しないことを確認。CI 証拠は checkout artifact の実測 SHA と run URL を照合（rf388 report の手法を踏襲）。
- **観測**: 先行 PoC は全て HEAD の ancestor SHA で実行。HEAD `7aab6302` の CI run は未実施（POLICIES §5 統合受入は merge 後対象 SHA の CI 証拠）。
- **結論**: ancestor SHA の証拠として verdict-matrix に記録し、最新 SHA の Pass に読み替えない。HEAD run 未実施は `NotRun`（Pending）として明記。

## 試行 2: Gate D の機構層 Pass と画素層 Fail の分離

- **目的**: `evidence/rf388-native-dock-windowing/report.md` の 33 checks Pass と native surface 画素 `Unsupported（実測）` を、Dock verdict としてどう記録するか決める。
- **試行**: report の「重大な発見」（GPUI 0.2.2 `directx_renderer.rs:793` `CreateTargetForHwnd(topmost=true)` が child HWND を覆う、`WS_EX_NOREDIRECTIONBITMAP` で GDI fallback なし、対照実験で plugin 固有でなく全 child HWND が非合成）と、rf388-native-dock-layout の「Dock PASS 不付与」明記を照合。
- **観測**: HWND 機構（parentage/geometry/clip/focus/key/redock）は実測 Pass だが、plugin editor の画素は GPUI DComp に覆われ観測不能。Dock PASS（完了条件全体）は各 report が「不成立/Partial」と明記。
- **結論**: verdict-matrix で GD-D1（機構層 Pass）/GD-D2（画素層 Fail + #532 Blocker）/GD-D3（Dock PASS 未達）を別 Case に分離。機構 Pass を Dock PASS に昇格させない。test-spec R-GATE-D-01/03 で固定。

## 試行 3: `Unsupported`（宣言）と `NotRun`（未実施）の分類

- **目的**: POLICIES §3 の要求状態と実行結果の分離を、verdict-matrix の各 Case に適用する。
- **試行**: `WindowCapabilities`/`SlotCapabilities` の宣言値（`ime: Unsupported`、`native_surface_slot: BACKEND_AVAILABLE = cfg!(windows)`、focus release `Err(Unsupported)`）と、未測定能力（a11y/DPI 動的/性能/Consumer Oracle/CLAP/AU/L1 cross-process/他社 plugin）を分類。
- **観測**: 宣言済み非対応は `Unsupported`（Capability 欠落として追跡）、未実験・未測定は `NotRun`、実測不一致は `Fail`、Deferred 要求の NotRun は Pass にしない。
- **結論**: verdict-matrix の requirement/result 列で分離し、test-spec R-VERDICT-01〜04 で語彙を固定。

## 試行 4: Gate E Consumer Oracle の NotRun 固定

- **目的**: Synvelis #176・AudioLens #166 の製品 GUI 受入を、rf465 inventory の L1 互換記録からどう判定するか決める。
- **試行**: `consumer-verify.yml` が workflow_dispatch 専用で対象 SHA run がないことを確認。rf465 `inventory.json` の両 consumer pin・lane 並存記録と、AL #166/SV #176 が製品 Owner であることを照合。
- **観測**: consumer-verify 対象 SHA run なし。rf465 は L1 互換の傍証であり製品 Oracle の Pass ではない。
- **結論**: Gate E を `NotRun`/`Deferred` に固定。GE-E1〜E4 で consumer-verify・AL・SV・L1 傍証を別 Case に記録。

## 試行 5: HEAD SHA での証拠再実走（正規 lane）

- **目的**: 文書・証拠 slice の検証として、HEAD `7aab6302` 上で正規 UI lane を再実走し同一 SHA 補強証拠を得る。
- **試行**: `mise run test-ui-scenario-contract`/`check-ui-driver-boundary`/`test-ui-native-slot`/`test-ui-parity`/`test-ui-gpui` を順次実行（GPUI compile を含むため時間を要したが完了）。
- **観測**: 全 lane Pass。scenario-contract fake 16 + gpui 7、boundary verdict: pass、native-slot 13、parity 14、gpui lib 77 + 各 test target（windowed_test_app bin compile/link 含む）。
- **結論**: `evidence/rf262-gpui-verdict/verification.md` に実コマンド・exit・結果を記録。これは L1 契約層の補強証拠であり、実 GUI/実 plugin の Pass ではない。

## 試行 6: 文書チェーンの front matter・relation 規約適用

- **目的**: #553 等の先例と同じ規約で #262 の文書チェーン（spec/design/test-spec/plan/task/IR/exp）を作成する。
- **試行**: `docs/{specs,designs,test-specs,plans,tasks,impl/ir,impl/exp}/issue-262-gpui-verdict.md` の skeleton を確認し、`SPEC-`/`DESIGN-`/`TSPEC-`/`PLAN-`/`TASK-`/`IMPL-` ID を採番。relation は `implements`/`depends-on`/`derives-from`/`verified-by`/`references`/`changes` を相互整合。
- **観測**: status は spec=proposed・design=approved・test-spec=proposed・plan=completed・task=done・ir=completed。`docs/impl/exp` は `.md` 自由形式記録を許容（#553 等の先例）。
- **結論**: 文書チェーンを作成。`doc-maintenance index apply`/`sync_impl_index` で管理 index を再生成し `in-sync` を確認。
