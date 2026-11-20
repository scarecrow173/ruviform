# Drive a real OS window under test

The windowed counterpart of the headless journey: `windowed_test_app`
opens real GPUI windows and serves the same `rvf.uitest.v1` closed op
set, so a driver can run the full sequence — handshake, observation,
fixtures, clock, faults, real screenshot capture, close-policy
drives, graceful shutdown — against actual OS windows. It needs an
interactive desktop session; everywhere else it fails closed.

## Sub-features

- `windowed-lane` — the canned `windowed_uitest` lane: spawn →
  hello → observe → fixture → clock → ipc_emit → faults →
  screenshot → close → shutdown, verdict JSON to
  `evidence/issue-313-windowed-uitest/verdict-windowed-uitest.json`.
- `windowed-drive` — `rvf-ctl drive --app` runs the same closed op
  set against the windowed AUT directly.
- `windowed-capture` — `test.observe.screenshot` returns real
  `rgba-raw` pixels where the OS capture backend exists.
- `windowed-close` — the `windows` fixture's `close_policy` plus
  `test.window.request_close` exercises veto and default-approve on
  real windows.

## How to get to it (user POV)

- Canonical lane (interactive desktop required):
  `mise run test-ui-windowed-uitest` — sets `RVF_WINDOWED_UITEST=1`,
  runs `cargo test -p ruviform-ui-gpui --features uitest --test
  windowed_uitest -- --test-threads=1`, writes the verdict JSON.
- Direct build:
  `cargo build -p ruviform-ui-gpui --features uitest --bin
  windowed_test_app`, then `target/debug/rvf-ctl drive --app
  target/debug/windowed_test_app --app-args "--windows 1" <step>...`.

## Driving it with rvf-ctl + the mise lane

Preconditions:

- An interactive desktop session (real OS windows). Headless hosts
  record every case `not_run` + `environment_blocked`, exit `0`.
- `windowed_test_app` built with the `uitest` feature and
  `rvf-ctl`/`dummy_app` built as in the baseline.

- **Run the lane.** `mise run test-ui-windowed-uitest`. Exit `0`;
  `evidence/issue-313-windowed-uitest/verdict-windowed-uitest.json`
  reports `verdict:"pass"`, `desktop_session:true`, backend
  `gpui-windowed`, and per-op verdicts for `hello`, `windows`,
  `fixture.prepare`, `receipts`, `clock.advance`, `ipc_emit`,
  `fault.set`, `fault.window_crash`, `screenshot`, `request_close`,
  `shutdown`.
- **Drive the AUT yourself.** `rvf-ctl drive --app
  target/debug/windowed_test_app --app-args "--windows 1"
  'observe.windows' 'fixture.prepare={"name":"windows","params":{"count":1,"width":320,"height":240}}'
  'observe.screenshot={"window":"last"}' 'observe.windows' --out
  <report>`. The screenshot step returns `format:"rgba-raw"` with
  `decoded_bytes == width*height*4` — real pixels, not a stub.
- **Close a real window.** Under `close_policy:"veto"`,
  `'window.request_close={"window":<id>}'` leaves it alive; after
  `close_policy:"unanswered"` the next request destroys it —
  `window.destroyed` attaches to the step's `notices`.
- **Teardown.** The drive's own `test.shutdown` +
  `checks["child-exit"]` prove the real process is gone; the verdict
  file and drive report survive.

## Gotchas

- No interactive session ⇒ `hello`/`windows` record
  `environment_blocked` and the lane still exits `0` — that is the
  fail-closed environment verdict, not a green run. Never cite it
  as execution evidence.
- A full-size window's RGBA reply can exceed the 1 MiB frame cap —
  create a small capture window
  (`{"count":1,"width":320,"height":240}`) before screenshotting.
- `RVF_WINDOWED_UITEST=1` is the lane's opt-in gate; without it the
  test body is a no-op and writes nothing.
- Always `--test-threads=1` — parallel windowed drives race on the
  same desktop.
- `screenshot` is `unsupported` on a platform without a capture
  backend — an honest verdict, not a failure.
- `--fail-init` (via `--app-args`) exercises the refused handshake;
  its failure is `environment_blocked`-or-`fail`, per the same
  mapping the lane uses.
