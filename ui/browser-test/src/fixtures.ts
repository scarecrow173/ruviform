/**
 * Playwright `test` extension (Issue #310).
 *
 * Consumers and ruviform specs alike use the stock `test` API — the
 * `rvf` fixture is the only added surface: `rvf.open("daw")` opens a
 * fixture page under the deterministic clock + hermetic serve layer
 * and returns the {@link HarnessSession}. No bespoke E2E DSL.
 */

import { test as base } from "@playwright/test";
import {
  HarnessSession,
  type HarnessPageName,
  type OpenHarnessOptions,
} from "./harness.ts";

export interface RvfFixture {
  /** Open a fixture page and wait for `__rvf.ready`. */
  open(
    name: HarnessPageName,
    opts?: OpenHarnessOptions,
  ): Promise<HarnessSession>;
}

export interface RvfWorkerFixtures {
  rvf: RvfFixture;
}

export const test = base.extend<RvfWorkerFixtures>({
  rvf: async ({ page }, use, testInfo) => {
    const sessions: HarnessSession[] = [];
    const fixture: RvfFixture = {
      async open(name, opts = {}) {
        const s = await HarnessSession.open(page, name, opts);
        sessions.push(s);
        return s;
      },
    };
    await use(fixture);
    // Diagnostics on failure: aborted/unmapped requests explain most
    // "page didn't mount" cases without a screenshot.
    const aborted = sessions.flatMap((s) => [...s.serveStats.aborted]);
    if (aborted.length > 0 && testInfo.status !== "passed") {
      await testInfo.attach("rvf-serve-aborted", {
        body: aborted.join("\n"),
        contentType: "text/plain",
      });
    }
  },
});

export { expect } from "@playwright/test";
