/**
 * DAW-shaped consumer fixture page (Issue #310 — "Consumer A").
 *
 * Mounts {@link mountDawFixture}: mixer-strip fader + transport +
 * automation knob inside one `mixer.strip.1` view on a
 * DemoControlGateway. Same Harness API surface as every other page —
 * a consumer-shaped fixture is data, not a different protocol.
 */

import { mountDawFixture } from "@rvf/showcase";
import { installRvfPage } from "@rvf/browser-test-page";

const host = document.getElementById("app");
if (host === null) throw new Error("daw page: #app missing");

const h = mountDawFixture(host, {
  flushWith: (fn) => fn(),
});

installRvfPage({
  gateway: h.gateway,
  receipts: h,
  unmount: () => h.unmount(),
});
