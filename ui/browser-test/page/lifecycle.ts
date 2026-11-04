/**
 * Lifecycle fixture page (Issue #310): remount/reconnect semantics.
 *
 * Mounts the DAW-shaped fixture and exposes `mount()`/`unmount()`
 * through `__rvf` so specs can cycle full teardown → fresh mount and
 * verify subscription/command dedup across the boundary (the
 * StrictMode-equivalent scenario: detach cleans up once, re-attach
 * restores without double-subscribing or double-dispatching).
 *
 * Each remount creates a fresh gateway (as a real consumer remount
 * would) — the leak oracle compares `unsubscribed` against the old
 * gateway's `subscribeCalls`, which stay reachable via the spec's
 * captured reference.
 */

import { mountDawFixture, type DawFixture } from "@rvf/showcase";
import { installRvfPage } from "@rvf/browser-test-page";

const host = document.getElementById("app");
if (host === null) throw new Error("lifecycle page: #app missing");

let handle: DawFixture | undefined;

const doMount = (): void => {
  handle = mountDawFixture(host, {
    flushWith: (fn) => fn(),
  });
  installRvfPage({
    gateway: handle.gateway,
    receipts: handle,
    mount: doMount,
    unmount: doUnmount,
  });
};

const doUnmount = (): void => {
  handle?.unmount();
  handle = undefined;
};

doMount();
