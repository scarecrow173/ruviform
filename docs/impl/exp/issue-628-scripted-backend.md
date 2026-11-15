# Issue #628 scripted port-trait backend — 実装実験記録（2026-11-15）

`docs/tasks/issue-628-scripted-backend.md` の 2nd slice（scripted
port-trait backend）実装で試行・観察した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-628-scripted-backend.md`](../ir/issue-628-scripted-backend.md)、
イベントログは
[`issue-628-scripted-backend.jsonl`](issue-628-scripted-backend.jsonl)
を参照。

## 観察事項

- **`DeviceShared` borrow**: `d.spec.output.as_mut()` の mutable
  borrow が guard 越しに後続の `d.` read 全てをブロック（E0502
  多数）。open を「fault 消費（scoped mut)→ immutable read →
  slot 代入」の 3 相に分割して解消。`MutexGuard` の field-precise
  borrow は `as_mut`/`as_ref` の再取得ごとに guard 全体を borrow
  するため、フェーズ分離が必要だった。
- **prologue borrow**: `&mut core.invalidated` + `&mut
  core.error_callback` の同時引数渡しが E0499。prologue を方向別
  `&mut OutputCore`/`&mut InputCore` 直受けに変え、pump 内で
  `let core = &mut *guard` に rebind して plain struct field borrow
  に戻した。
- **`#[non_exhaustive]` match**: `BufferSize`/`SupportedBufferSize`
  に wildcard arm が必須 — 将来 variant は `UnsupportedConfig` /
  negotiation reject に割当（silent coerce しない）。
- **test 初回失敗 3 件は test 側バグ**: output-capable 数の数え違い、
  `is_output_open` が pump 時にのみ latch される仕様の誤読、
  drop 前の再 open で `DeviceBusy`。実装は契約どおりで修正は
  assertion 側。
- **`PumpReport`/`Box<dyn Stream>` は `PartialEq`/`Debug` 非実装**:
  Err 検証は `matches!`、stream open の Err は `matches!`（
  `unwrap_err` は `T: Debug` 要求で使用不可）。
- **clippy `-D warnings` 対応**: poison-tolerant `lock` helper を
  `pub(crate)` 化して全 lock 箇所に統一（redundant_closure 解消）、
  `pump_input` の `too_many_lines` は scenario fill を `play_scenario`
  に抽出、float 比較は `to_bits()` で bit-exact 検証に変更。
- **`rt_` alloc-free 実測**: steady-state pump（fault 無し・
  `record_output: false`）で allocation 0 を `assert_no_alloc` で
  確認。`collect_faults` の `Vec::new()` は push 時のみ alloc のため
  steady state では不発 — caller-owned scratch 化は将来の最適化
  余地として残す。
