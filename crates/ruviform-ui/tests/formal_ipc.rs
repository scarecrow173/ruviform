//! Issue #806 slice 1 (D-layer formal verification) — `rvf.ipc.v1`
//! wire codec property suite plus `#[cfg(kani)]` bounded proof
//! harnesses.
//!
//! Target contracts (product source of truth:
//! `crates/ruviform-ui/src/ipc.rs`):
//!
//! - `parse_intent(&str) -> Result<Intent, ErrorBody>` never panics;
//!   `Err` codes stay in the stable set
//!   {`codes::MESSAGE_TOO_LARGE`, `codes::MALFORMED_MESSAGE`}, and a
//!   body longer than `MAX_INTENT_BYTES` is `MESSAGE_TOO_LARGE`
//!   before any parse attempt.
//! - `valid_op(&str) -> bool` is a total function over arbitrary
//!   input and accepts exactly the documented shape (non-empty
//!   `[a-z0-9_.-]` segments joined by `.`, `<= MAX_OP_LEN` bytes).
//! - `Event::to_json` always emits a parseable versioned envelope
//!   (`v == SCHEMA_VERSION`) preserving the variant kind and fields.
//!
//! The `#[cfg(kani)]` module at the bottom carries the remaining bounded
//! proof for the event contract (run via `mise run test-formal-kani`).
//! The `valid_op` byte-core proofs and envelope size/field decision-core
//! proofs live beside the production predicates in
//! `crates/ruviform-ui/src/ipc.rs`; bounds and assumptions are documented in
//! `docs/specs/formal-verification.md`.

use proptest::prelude::*;
use ruviform_ui::ipc::MAX_OP_LEN;
use ruviform_ui::{
    ErrorBody, Event, Intent, JobState, LifecycleState, MAX_INTENT_BYTES, SCHEMA_VERSION, codes,
    parse_intent, valid_op,
};
use serde_json::{Value, json};

/// Semantic JSON equality for round-trip assertions: objects and
/// arrays recurse; `Number` is exact for integers and within 1 ulp
/// for floats. `serde_json`'s default parser can round a decimal
/// 1 ulp off the shortest-representation f64 (`float_roundtrip` is
/// not enabled) — a wire-format parser-precision limit, not a
/// contract violation.
fn json_value_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            if x == y {
                true
            } else if let (Some(xf), Some(yf)) = (x.as_f64(), y.as_f64()) {
                (xf - yf).abs() <= xf.abs().max(yf.abs()) * f64::EPSILON
            } else {
                false
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| json_value_eq(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| json_value_eq(v, w)))
        }
        _ => a == b,
    }
}

/// Arbitrary `parse_intent` body — mostly arbitrary Unicode text,
/// plus raw bytes pushed through `from_utf8_lossy` for binary-ish
/// inputs. Bounded well below `MAX_INTENT_BYTES`; the cap path gets
/// its own dedicated case.
fn arb_body() -> impl Strategy<Value = String> {
    prop_oneof![
        8 => prop::collection::vec(any::<char>(), 0..=2048).prop_map(String::from_iter),
        2 => prop::collection::vec(any::<u8>(), 0..=2048)
            .prop_map(|v| String::from_utf8_lossy(&v).into_owned()),
    ]
}

/// Arbitrary JSON value, depth/size-bounded so `params`/`data`
/// payloads stay small.
fn arb_json_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(Value::from),
        // finite only — NaN/infinity are not representable in JSON
        (-1e6..1e6f64).prop_map(Value::from),
        ".{0,32}".prop_map(Value::String),
    ];
    leaf.prop_recursive(3, 16, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
            prop::collection::btree_map(".{0,16}", inner, 0..4).prop_map(|m| {
                m.into_iter()
                    .collect::<serde_json::Map<String, Value>>()
                    .into()
            }),
        ]
    })
}

/// Ops satisfying the `valid_op` contract by construction: 2–4
/// non-empty `[a-z0-9_-]` segments joined by `.` (always within
/// `MAX_OP_LEN`).
fn arb_valid_op() -> impl Strategy<Value = String> {
    prop::collection::vec("[a-z0-9_-]{1,8}", 2..=4).prop_map(|segs| segs.join("."))
}

/// Ops whose shape breaks the `valid_op` contract in exactly one
/// way — always rejected.
fn arb_broken_op() -> impl Strategy<Value = String> {
    prop_oneof![
        // uppercase is outside the allowed character set
        arb_valid_op().prop_map(|mut op| {
            op.insert(op.len() / 2, 'A');
            op
        }),
        // leading dot → empty first segment
        arb_valid_op().prop_map(|op| format!(".{op}")),
        // trailing dot → empty last segment
        arb_valid_op().prop_map(|op| format!("{op}.")),
        // doubled dots → empty inner segments
        arb_valid_op().prop_map(|op| op.replace('.', "..")),
        // over the byte cap
        arb_valid_op().prop_map(|mut op| {
            op.push_str(&"a".repeat(MAX_OP_LEN));
            op
        }),
        // no `.` separator at all
        arb_valid_op().prop_map(|op| op.replace('.', "-")),
        // forbidden ASCII character
        arb_valid_op().prop_map(|mut op| {
            op.insert(op.len() / 2, '@');
            op
        }),
        // non-ASCII is outside the allowed character set
        arb_valid_op().prop_map(|op| format!("あ{op}")),
    ]
}

/// Schema-valid intents — `Intent::new` fills `v`/`kind`, so a
/// serialized value always satisfies the envelope gates.
fn arb_intent() -> impl Strategy<Value = Intent> {
    (".{0,32}", arb_valid_op(), arb_json_value())
        .prop_map(|(id, op, params)| Intent::new(id, op, params))
}

fn envelope_json(v: u32, kind: &str, op: &str) -> String {
    serde_json::to_string(&json!({
        "v": v,
        "kind": kind,
        "id": "i",
        "op": op,
        "params": {},
    }))
    .unwrap()
}

fn padded_valid_envelope_body(pad_len: usize) -> String {
    let prefix = "{\"v\":1,\"kind\":\"intent\",\"id\":\"i\",\"op\":\"a.b\",\"params\":{\"pad\":\"";
    let suffix = "\"}}";
    format!("{prefix}{}{suffix}", "x".repeat(pad_len))
}

fn exact_cap_pad_len() -> usize {
    let prefix = "{\"v\":1,\"kind\":\"intent\",\"id\":\"i\",\"op\":\"a.b\",\"params\":{\"pad\":\"";
    let suffix = "\"}}";
    MAX_INTENT_BYTES - prefix.len() - suffix.len()
}

fn exact_cap_valid_envelope_body() -> String {
    padded_valid_envelope_body(exact_cap_pad_len())
}

/// Arbitrary `ErrorBody` payloads for `Event::Result`/`Job`/`Error`
/// error fields.
fn arb_error_body() -> impl Strategy<Value = ErrorBody> {
    (".{0,24}", ".{0,64}").prop_map(|(code, message)| ErrorBody {
        code,
        message,
        details: None,
    })
}

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        (
            ".{0,32}",
            any::<bool>(),
            prop::option::of(arb_json_value()),
            prop::option::of(arb_error_body())
        )
            .prop_map(|(id, ok, data, error)| Event::Result {
                id,
                ok,
                data,
                error
            }),
        (
            any::<u64>(),
            prop_oneof![
                Just(JobState::Progress),
                Just(JobState::Finished),
                Just(JobState::Cancelled),
                Just(JobState::Failed)
            ],
            prop::option::of(any::<u64>()),
            prop::option::of(any::<u64>()),
            prop::option::of(".{0,16}"),
            prop::option::of(arb_json_value()),
            prop::option::of(arb_error_body())
        )
            .prop_map(
                |(job, state, done, total, detail, data, error)| Event::Job {
                    job,
                    state,
                    done,
                    total,
                    detail,
                    data,
                    error
                }
            ),
        (any::<u64>(), arb_json_value()).prop_map(|(seq, data)| Event::Notify { seq, data }),
        Just(LifecycleState::Closing).prop_map(|state| Event::Lifecycle { state }),
        arb_error_body().prop_map(|error| Event::Error { error }),
    ]
}

proptest! {
    /// For any in-cap input, `parse_intent` terminates without panic
    /// and `Err` codes stay in the stable two-code set. A parsed
    /// intent always carries the enforced envelope fields.
    #[test]
    fn parse_intent_arbitrary_error_set(body in arb_body()) {
        match parse_intent(&body) {
            Err(e) => prop_assert!(
                e.code == codes::MESSAGE_TOO_LARGE || e.code == codes::MALFORMED_MESSAGE,
                "error code {:?} is outside the stable set",
                e.code,
            ),
            Ok(intent) => {
                prop_assert_eq!(intent.v, SCHEMA_VERSION);
                prop_assert_eq!(intent.kind, "intent");
                prop_assert!(valid_op(&intent.op));
            }
        }
    }

    /// The size cap wins over every other gate: a body longer than
    /// `MAX_INTENT_BYTES` is `MESSAGE_TOO_LARGE` whether it is
    /// well-formed or not.
    #[test]
    fn parse_intent_oversize_is_too_large(well_formed in any::<bool>(), over in 1usize..=64) {
        let pad = "x".repeat(MAX_INTENT_BYTES + over);
        let body = if well_formed {
            format!("{{\"v\":1,\"kind\":\"intent\",\"id\":\"i\",\"op\":\"a.b\",\"params\":{{\"pad\":\"{pad}\"}}}}")
        } else {
            pad
        };
        match parse_intent(&body) {
            Err(e) => prop_assert_eq!(e.code, codes::MESSAGE_TOO_LARGE),
            Ok(_) => prop_assert!(false, "oversized body must not parse"),
        }
    }

    /// `valid_op` is total over arbitrary input, and whenever it
    /// accepts, the documented shape invariants hold.
    #[test]
    fn valid_op_total_and_sound(op in arb_body()) {
        if valid_op(&op) {
            prop_assert!(!op.is_empty());
            prop_assert!(op.len() <= MAX_OP_LEN);
            prop_assert!(op.contains('.'));
            prop_assert!(op
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()
                    || matches!(c, '.' | '_' | '-')));
            prop_assert!(op.split('.').all(|seg| !seg.is_empty()));
        }
    }

    /// Generator-valid ops are always accepted.
    #[test]
    fn valid_op_accepts_generated(op in arb_valid_op()) {
        prop_assert!(valid_op(&op), "generated valid op {op:?} rejected");
    }

    /// Contract-broken ops are rejected by `valid_op` — and by the
    /// `parse_intent` envelope gate that enforces the same shape.
    #[test]
    fn broken_op_rejected_everywhere(op in arb_broken_op()) {
        prop_assert!(!valid_op(&op), "broken op {op:?} accepted");
        let body = serde_json::to_string(&json!({
            "v": SCHEMA_VERSION, "kind": "intent", "id": "i", "op": op, "params": {},
        }))
        .unwrap();
        match parse_intent(&body) {
            Err(e) => prop_assert_eq!(e.code, codes::MALFORMED_MESSAGE),
            Ok(_) => prop_assert!(false, "intent with broken op {op:?} parsed"),
        }
    }

    /// Any schema-valid intent survives serialize → `parse_intent`
    /// with every field intact.
    #[test]
    fn intent_serialize_parse_roundtrip(intent in arb_intent()) {
        let body = serde_json::to_string(&intent).unwrap();
        let parsed = parse_intent(&body).expect("generated intent must parse");
        prop_assert_eq!(parsed.v, intent.v);
        prop_assert_eq!(parsed.kind, intent.kind);
        prop_assert_eq!(parsed.id, intent.id);
        prop_assert_eq!(parsed.op, intent.op);
        prop_assert!(json_value_eq(&parsed.params, &intent.params));
    }

    /// `Event::to_json` always yields a parseable `v=1` envelope that
    /// preserves the variant kind and its fields.
    #[test]
    fn event_to_json_versioned_envelope(ev in arb_event()) {
        let text = ev.to_json();
        let v: Value = serde_json::from_str(&text).expect("to_json output must parse");
        prop_assert_eq!(&v["v"], &json!(SCHEMA_VERSION));
        match &ev {
            Event::Result { id, ok, data, error } => {
                prop_assert_eq!(&v["kind"], &json!("result"));
                prop_assert_eq!(&v["id"], &json!(id));
                prop_assert_eq!(&v["ok"], &json!(ok));
                match data {
                    Some(d) => prop_assert!(json_value_eq(&v["data"], d)),
                    None => prop_assert!(v.get("data").is_none()),
                }
                match error {
                    Some(e) => prop_assert_eq!(&v["error"]["code"], &json!(&e.code)),
                    None => prop_assert!(v.get("error").is_none()),
                }
            }
            Event::Job { job, state, done, total, detail, data, error } => {
                prop_assert_eq!(&v["kind"], &json!("job"));
                prop_assert_eq!(&v["job"], &json!(job));
                let expected = match state {
                    JobState::Progress => "progress",
                    JobState::Finished => "finished",
                    JobState::Cancelled => "cancelled",
                    JobState::Failed => "failed",
                    _ => unreachable!("new JobState variant — extend the property"),
                };
                prop_assert_eq!(&v["state"], &json!(expected));
                prop_assert_eq!(v.get("done").is_some(), done.is_some());
                prop_assert_eq!(v.get("total").is_some(), total.is_some());
                match detail {
                    Some(d) => prop_assert_eq!(&v["detail"], &json!(d)),
                    None => prop_assert!(v.get("detail").is_none()),
                }
                prop_assert_eq!(v.get("data").is_some(), data.is_some());
                prop_assert_eq!(v.get("error").is_some(), error.is_some());
            }
            Event::Notify { seq, data } => {
                prop_assert_eq!(&v["kind"], &json!("notify"));
                prop_assert_eq!(&v["seq"], &json!(seq));
                prop_assert!(json_value_eq(&v["data"], data));
            }
            Event::Lifecycle { state } => {
                prop_assert_eq!(&v["kind"], &json!("lifecycle"));
                let expected = match state {
                    LifecycleState::Closing => "closing",
                    _ => unreachable!("new LifecycleState variant — extend the property"),
                };
                prop_assert_eq!(&v["state"], &json!(expected));
            }
            Event::Error { error } => {
                prop_assert_eq!(&v["kind"], &json!("error"));
                prop_assert_eq!(&v["error"]["code"], &json!(&error.code));
            }
            _ => prop_assert!(false, "new Event variant — extend the property"),
        }
    }
}

/// Direct shell-to-core connection checks for the behavior-preserving
/// envelope decision-core refactor. The message assertions are a regression
/// oracle for current behavior only; they do not promote the messages or gate
/// priority to a new stable wire contract.
#[test]
fn envelope_gate_connection_regression() {
    let exact = exact_cap_valid_envelope_body();
    assert_eq!(exact.len(), MAX_INTENT_BYTES);
    let parsed = parse_intent(&exact).expect("exact-cap valid envelope must parse");
    assert_eq!(parsed.v, SCHEMA_VERSION);
    assert_eq!(parsed.kind, "intent");
    assert_eq!(parsed.op, "a.b");
    assert_eq!(
        parsed.params["pad"].as_str().unwrap().len(),
        exact_cap_pad_len()
    );

    let mut over_and_malformed = exact.clone();
    over_and_malformed.push('!');
    assert_eq!(over_and_malformed.len(), MAX_INTENT_BYTES + 1);
    let err = parse_intent(&over_and_malformed).unwrap_err();
    assert_eq!(err.code, codes::MESSAGE_TOO_LARGE);

    let small = envelope_json(SCHEMA_VERSION, "intent", "a.b");
    parse_intent(&small).expect("small valid envelope must parse");

    let err = parse_intent(&envelope_json(SCHEMA_VERSION + 1, "intent", "a.b")).unwrap_err();
    assert_eq!(err.code, codes::MALFORMED_MESSAGE);
    assert!(err.message.starts_with("unsupported schema version"));

    let err = parse_intent(&envelope_json(SCHEMA_VERSION, "result", "a.b")).unwrap_err();
    assert_eq!(err.code, codes::MALFORMED_MESSAGE);
    assert!(err.message.starts_with("unsupported kind"));

    let err = parse_intent(&envelope_json(SCHEMA_VERSION, "intent", "..")).unwrap_err();
    assert_eq!(err.code, codes::MALFORMED_MESSAGE);
    assert!(err.message.starts_with("invalid op name"));

    let err = parse_intent(&envelope_json(SCHEMA_VERSION + 1, "result", "..")).unwrap_err();
    assert_eq!(err.code, codes::MALFORMED_MESSAGE);
    assert!(err.message.starts_with("unsupported schema version"));

    let err = parse_intent(&envelope_json(SCHEMA_VERSION, "result", "..")).unwrap_err();
    assert_eq!(err.code, codes::MALFORMED_MESSAGE);
    assert!(err.message.starts_with("unsupported kind"));

    let err = parse_intent(&envelope_json(SCHEMA_VERSION, "intent", "..")).unwrap_err();
    assert_eq!(err.code, codes::MALFORMED_MESSAGE);
    assert!(err.message.starts_with("invalid op name"));
}

/// Bounded proofs of the remaining event contract, verified exhaustively
/// over symbolic input by Kani (`cargo kani --tests`; run via
/// `mise run test-formal-kani`). Bounds and assumptions:
/// `docs/specs/formal-verification.md`.
#[cfg(kani)]
mod proofs {
    use super::*;

    /// `to_json` always produces a parseable `v=1` envelope — shown
    /// for a `result` event with a symbolic correlation id. The
    /// unwind bound caps serde loops over the ~60-byte envelope.
    #[kani::proof]
    #[kani::unwind(80)]
    fn event_to_json_wellformed() {
        let id_bytes: [u8; 6] = kani::any();
        let Ok(id) = core::str::from_utf8(&id_bytes) else {
            return;
        };
        let event = Event::result_ok(id, Value::Null);
        let text = event.to_json();
        // `match`, not `.unwrap()` — Debug::fmt on error types recurses
        // through `dyn Error` and is a known CBMC cost bomb.
        let Ok(v) = serde_json::from_str::<Value>(&text) else {
            unreachable!("to_json output must parse")
        };
        assert!(v["v"] == json!(SCHEMA_VERSION));
        assert!(v["kind"] == json!("result"));
        assert!(v["ok"] == json!(true));
        assert!(v["id"] == json!(id));
    }
}
