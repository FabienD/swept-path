import { describe, expect, it, vi } from "vitest";
import { createStore } from "./store";

describe("store", () => {
  it("notifies subscribers when a field changes", () => {
    const store = createStore({ openingWidth: 2.4, busy: false });
    const seen = vi.fn();
    store.subscribe(seen);

    store.set({ openingWidth: 3.0 });

    expect(seen).toHaveBeenCalledTimes(1);
    expect(store.get().openingWidth).toBe(3.0);
    expect(store.get().busy).toBe(false);
  });

  it("stays silent when a set changes nothing", () => {
    // The whole SVG is rebuilt on every notification, so a spurious one
    // costs real work.
    const store = createStore({ openingWidth: 2.4 });
    const seen = vi.fn();
    store.subscribe(seen);

    store.set({ openingWidth: 2.4 });

    expect(seen).not.toHaveBeenCalled();
  });

  it("stops notifying after unsubscribe", () => {
    const store = createStore({ n: 0 });
    const seen = vi.fn();
    const stop = store.subscribe(seen);
    stop();

    store.set({ n: 1 });

    expect(seen).not.toHaveBeenCalled();
  });

  it("notifies every subscriber", () => {
    const store = createStore({ n: 0 });
    const first = vi.fn();
    const second = vi.fn();
    store.subscribe(first);
    store.subscribe(second);

    store.set({ n: 1 });

    expect(first).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledTimes(1);
  });
});
