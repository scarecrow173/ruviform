//! Per-window wry/tao wiring shared by the session loop
//! ([`crate::session::run_multi`]).
//!
//! Structure (inherited from `poc/ui`, where every mechanism was
//! measured):
//!
//! - **Wake-up**: `WebView2` callbacks do not wake a `WaitUntil`
//!   event loop, so IPC/page-load handlers push onto a queue and nudge
//!   an `EventLoopProxy`. `Event::UserEvent` drains the queue.
//! - **Resize follow**: the webview is `build()`ed (not
//!   `build_as_child` — `WebView2` 155 did not propagate child-container
//!   resizes into the JS viewport) and `set_bounds` re-applied on every
//!   `Resized`/`ScaleFactorChanged`.
//! - **Contract mapping** (Issue #423, migration table H3/H4): page
//!   load becomes [`WindowLifecycle::Ready`]; an IPC body becomes a
//!   typed [`WindowLifecycle::Action`]. The raw string and the page URL
//!   never enter the driver-neutral contract.

use std::sync::Arc;

use dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use ruviform_ui::ipc::{ErrorBody, parse_intent};
use ruviform_ui::{AssetSource, DataRegistry, UiError, WindowLifecycle};
use tao::window::Window;
use wry::{PageLoadEvent, Rect, WebContext, WebView, WebViewBuilder};

/// One decoded wry callback. `Err` is a malformed IPC body; the
/// session answers it on the originating window with `Event::error`.
pub(crate) type Inbound = Result<WindowLifecycle, ErrorBody>;

/// Decodes a `window.ipc.postMessage` body into a typed action.
pub(crate) fn inbound_message(body: &str) -> Inbound {
    parse_intent(body).map(WindowLifecycle::Action)
}

/// Wires the `rvf` scheme, IPC handler, page-load handler, navigation
/// policy, bridge script and initial bounds into a webview build.
///
/// `enqueue` is the per-window sink the session wraps with its own
/// routing; it is called from backend callbacks on the UI thread.
pub(crate) fn build_webview(
    window: &Window,
    web_context: &mut WebContext,
    url: &str,
    devtools: bool,
    assets: Arc<dyn AssetSource>,
    data: DataRegistry,
    enqueue: impl Fn(Inbound) + Clone + 'static,
) -> Result<WebView, UiError> {
    let ipc_enqueue = enqueue.clone();
    let load_enqueue = enqueue;
    WebViewBuilder::new_with_web_context(web_context)
        .with_url(url.to_string())
        .with_initialization_script(ruviform_ui::BRIDGE_JS)
        .with_custom_protocol("rvf".to_string(), move |_id, request| {
            crate::scheme::serve(&assets, &data, &request)
        })
        .with_ipc_handler(move |request: http::Request<String>| {
            ipc_enqueue(inbound_message(request.body()));
        })
        .with_on_page_load_handler(move |event, _url| {
            // The page URL is the `rvf://app` origin; it is not part of
            // the neutral `Ready` (table H3).
            if matches!(event, PageLoadEvent::Finished) {
                load_enqueue(Ok(WindowLifecycle::Ready));
            }
        })
        .with_navigation_handler(|url| {
            // Only the app origin (and its Windows http://rvf.* rewrite)
            // may navigate; everything else is rejected.
            url.starts_with("rvf://app")
                || url.starts_with("http://rvf.app")
                || url == "about:blank"
        })
        .with_devtools(devtools)
        .with_bounds(full_bounds(window))
        .build(window)
        .map_err(|e| UiError::backend_unavailable(format!("webview init failed: {e}")))
}

pub(crate) fn full_bounds(window: &Window) -> Rect {
    bounds_for(window.inner_size())
}

pub(crate) fn bounds_for(size: PhysicalSize<u32>) -> Rect {
    Rect {
        position: Position::Physical(PhysicalPosition::new(0, 0)),
        size: Size::Physical(size),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_ui::codes;

    #[test]
    fn ipc_body_maps_to_typed_action() {
        let body = r#"{"v":1,"kind":"intent","id":"i-1","op":"demo.sleep","params":{}}"#;
        match inbound_message(body) {
            Ok(WindowLifecycle::Action(intent)) => {
                assert_eq!(intent.id, "i-1");
                assert_eq!(intent.op, "demo.sleep");
            }
            other => panic!("expected Action, got {other:?}"),
        }
    }

    #[test]
    fn malformed_ipc_body_is_an_adapter_error() {
        match inbound_message("not json") {
            Err(e) => assert_eq!(e.code, codes::MALFORMED_MESSAGE),
            other => panic!("expected malformed error, got {other:?}"),
        }
    }
}
