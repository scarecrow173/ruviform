/** NumericInput (text/IME/edit-gesture) and Transport (intent-only). */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import "./env.ts";
import { mount, unmount } from "./env.ts";
import { collectGestures, key, part, typesOf } from "./helpers.ts";
import {
  registerRvfElements,
  RVF_TRANSPORT_EVENT,
  type RvfControlElement,
  type RvfTransport,
} from "../src/index.ts";

registerRvfElements();

const DESCRIPTOR = { min: 0, max: 100, defaultValue: 20 } as const;

function makeNumeric(): RvfControlElement {
  const el = document.createElement("rvf-numeric-input") as RvfControlElement;
  el.descriptor = { ...DESCRIPTOR };
  document.body.append(el);
  return el;
}

function inputOf(el: RvfControlElement): HTMLInputElement {
  return part(el, "input") as unknown as HTMLInputElement;
}

describe("rvf-numeric-input", () => {
  it("role=spinbutton on the inner input with value ARIA", () => {
    const el = makeNumeric();
    const input = inputOf(el);
    assert.equal(input.getAttribute("role"), "spinbutton");
    assert.equal(input.getAttribute("aria-valuemin"), "0");
    assert.equal(input.getAttribute("aria-valuemax"), "100");
    assert.equal(input.getAttribute("aria-valuenow"), "20");
    unmount(el);
  });

  it("typing opens the edit gesture and previews parseable text", () => {
    const el = makeNumeric();
    const input = inputOf(el);
    const { events } = collectGestures(el);
    input.value = "45";
    input.dispatchEvent(new Event("input"));
    assert.deepEqual(typesOf(events), ["begin", "preview"]);
    const preview = events.find((e) => e.type === "preview");
    assert.equal(
      preview?.type === "preview" ? preview.value.plain : undefined,
      45,
    );
    unmount(el);
  });

  it("Enter commits parseable text", () => {
    const el = makeNumeric();
    const input = inputOf(el);
    const { events } = collectGestures(el);
    input.value = "45";
    input.dispatchEvent(new Event("input"));
    input.dispatchEvent(key("keydown", { key: "Enter" }));
    assert.deepEqual(typesOf(events), ["begin", "preview", "commit"]);
    assert.equal(el.value, 45);
    assert.equal(input.value, "45");
    unmount(el);
  });

  it("Escape cancels and restores the committed display", () => {
    const el = makeNumeric();
    const input = inputOf(el);
    const { events } = collectGestures(el);
    input.value = "45";
    input.dispatchEvent(new Event("input"));
    input.dispatchEvent(key("keydown", { key: "Escape" }));
    assert.deepEqual(typesOf(events), ["begin", "preview", "cancel"]);
    assert.equal(el.value, 20);
    assert.equal(input.value, "20");
    unmount(el);
  });

  it("invalid text on commit reports rejected, then blur cancels+reverts", () => {
    const el = makeNumeric();
    const input = inputOf(el);
    const { events } = collectGestures(el);
    input.value = "abc";
    input.dispatchEvent(new Event("input"));
    input.dispatchEvent(key("keydown", { key: "Enter" }));
    assert.ok(events.some((e) => e.type === "rejected"));
    // gesture still alive — user may fix the text
    input.dispatchEvent(new Event("focusout", { bubbles: true }));
    assert.ok(events.some((e) => e.type === "cancel"));
    assert.equal(el.value, 20);
    assert.equal(input.value, "20");
    unmount(el);
  });

  it("IME composition suspends dispatch until compositionend", () => {
    const el = makeNumeric();
    const input = inputOf(el);
    const { events } = collectGestures(el);
    input.dispatchEvent(new Event("compositionstart"));
    input.value = "4";
    input.dispatchEvent(new Event("input"));
    assert.equal(events.length, 0);
    input.value = "45";
    input.dispatchEvent(new Event("compositionend"));
    input.dispatchEvent(key("keydown", { key: "Enter" }));
    assert.ok(events.some((e) => e.type === "commit"));
    assert.equal(el.value, 45);
    unmount(el);
  });

  it("ArrowUp/Down adjust atomically without opening the edit gesture", () => {
    const el = makeNumeric();
    const input = inputOf(el);
    const { events } = collectGestures(el);
    input.dispatchEvent(key("keydown", { key: "ArrowUp" }));
    assert.deepEqual(typesOf(events), ["begin", "commit"]);
    assert.ok(el.value > 20);
    unmount(el);
  });

  it("blur commits parseable text", () => {
    const el = makeNumeric();
    const input = inputOf(el);
    input.value = "30";
    input.dispatchEvent(new Event("input"));
    input.dispatchEvent(new Event("focusout", { bubbles: true }));
    assert.equal(el.value, 30);
    unmount(el);
  });
});

describe("rvf-transport", () => {
  it("emits rvf-transport intents with action + sequence", () => {
    const el = mount("rvf-transport", { "control-id": "main" }) as RvfTransport;
    const seen: Array<{ action: string; sequence: number }> = [];
    el.addEventListener(RVF_TRANSPORT_EVENT, (e) => {
      const d = (e as CustomEvent).detail;
      seen.push({ action: d.action, sequence: d.sequence });
    });
    const play = part(el, "play") as HTMLButtonElement;
    const stop = part(el, "stop") as HTMLButtonElement;
    play.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    stop.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    assert.deepEqual(seen, [
      { action: "play", sequence: 1 },
      { action: "stop", sequence: 2 },
    ]);
    unmount(el);
  });

  it("capabilities mark impossible actions disabled and silent", () => {
    const el = mount("rvf-transport") as RvfTransport;
    el.capabilities = { resume: false };
    const resume = part(el, "resume") as HTMLButtonElement;
    assert.equal(resume.disabled, true);
    const seen: string[] = [];
    el.addEventListener(RVF_TRANSPORT_EVENT, (e) =>
      seen.push((e as CustomEvent).detail.action),
    );
    resume.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    assert.equal(seen.length, 0); // disabled button does not fire
    unmount(el);
  });

  it("state attribute lights the matching button", () => {
    const el = mount("rvf-transport", { state: "playing" }) as RvfTransport;
    assert.ok((part(el, "play") as HTMLElement).hasAttribute("data-active"));
    assert.equal(
      (part(el, "stop") as HTMLElement).hasAttribute("data-active"),
      false,
    );
    el.setAttribute("state", "paused");
    assert.ok((part(el, "pause") as HTMLElement).hasAttribute("data-active"));
    unmount(el);
  });

  it("has no transport model: no state changes on its own intents", () => {
    const el = mount("rvf-transport") as RvfTransport;
    (part(el, "play") as HTMLButtonElement).dispatchEvent(
      new MouseEvent("click", { bubbles: true }),
    );
    // intent emitted, but `state` is consumer-driven and untouched
    assert.equal(el.state, "stopped");
    unmount(el);
  });
});
