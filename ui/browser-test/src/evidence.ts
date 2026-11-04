/**
 * Evidence reporter (Issue #310): writes `test-results/evidence.json`
 * at the end of every run.
 *
 * Records the environment (OS, Node, Playwright version, browser
 * projects actually used), the git revision, the deterministic seed,
 * and each test's outcome — pass / failed / skipped (not-run). This
 * is the browser-lane evidence surface the Cross-Consumer Gate
 * (#294) consumes; a browser PASS here is evidence for the browser
 * lane only, never for OS WebView / Native GPU / product E2E.
 */

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type {
  FullResult,
  Reporter,
  Suite,
  TestCase,
  TestResult,
} from "@playwright/test/reporter";

import { RVF_DEFAULT_SEED, RVF_FIXED_NOW } from "./harness.ts";

interface TestEvidence {
  readonly title: string;
  readonly file: string;
  readonly project: string;
  /** pass | failed | timedOut | skipped (not-run) | interrupted. */
  readonly status: string;
  readonly expected: boolean;
  readonly durationMs: number;
  readonly errors: string[];
}

export interface EvidenceFile {
  readonly lane: "browser";
  readonly startedAt: string;
  readonly finishedAt: string;
  readonly env: {
    readonly os: string;
    readonly arch: string;
    readonly node: string;
    readonly playwright: string;
    readonly browsers: string[];
  };
  readonly git: { readonly revision: string };
  readonly clock: { readonly fixedNow: string };
  readonly seed: string;
  readonly tests: TestEvidence[];
  readonly summary: { pass: number; fail: number; skipped: number };
}

function gitRevision(): string {
  try {
    return execFileSync("git", ["rev-parse", "HEAD"], {
      encoding: "utf-8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return "unknown";
  }
}

export default class EvidenceReporter implements Reporter {
  private suite?: Suite;
  private readonly startedAt = new Date().toISOString();
  private readonly tests: TestEvidence[] = [];
  private projects: string[] = [];

  onBegin(_config: unknown, suite: Suite): void {
    this.suite = suite;
  }

  onTestEnd(test: TestCase, result: TestResult): void {
    const project = test.parent
      .project()
      ?.name ?? "default";
    if (!this.projects.includes(project)) this.projects.push(project);
    this.tests.push({
      title: test.titlePath().slice(3).join(" › ") || test.title,
      file: path.basename(test.location.file),
      project,
      status: result.status,
      expected: result.status === test.expectedStatus,
      durationMs: result.duration,
      errors: result.errors.map((e) => (e.message ?? String(e)).slice(0, 500)),
    });
  }

  onEnd(_result: FullResult): void {
    const outDir = path.resolve(
      path.dirname(fileURLToPath(import.meta.url)),
      "../test-results",
    );
    mkdirSync(outDir, { recursive: true });
    const summary = { pass: 0, fail: 0, skipped: 0 };
    for (const t of this.tests) {
      if (t.status === "skipped") summary.skipped++;
      else if (t.expected && t.status === "passed") summary.pass++;
      else summary.fail++;
    }
    const evidence: EvidenceFile = {
      lane: "browser",
      startedAt: this.startedAt,
      finishedAt: new Date().toISOString(),
      env: {
        os: `${os.type()} ${os.release()}`,
        arch: os.arch(),
        node: process.version,
        playwright: playwrightVersion(),
        browsers: [...this.projects].sort(),
      },
      git: { revision: gitRevision() },
      clock: { fixedNow: RVF_FIXED_NOW },
      seed: RVF_DEFAULT_SEED,
      tests: this.tests,
      summary,
    };
    writeFileSync(
      path.join(outDir, "evidence.json"),
      `${JSON.stringify(evidence, null, 2)}\n`,
    );
  }

  printsToStdio(): boolean {
    return false;
  }
}

function playwrightVersion(): string {
  try {
    const pkg = path.resolve(
      path.dirname(fileURLToPath(import.meta.url)),
      "../node_modules/@playwright/test/package.json",
    );
    return (
      JSON.parse(readFileSync(pkg, "utf-8")) as { version?: string }
    ).version ?? "unknown";
  } catch {
    return "unknown";
  }
}
