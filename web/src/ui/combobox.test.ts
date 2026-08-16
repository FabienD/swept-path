import { describe, expect, it } from "vitest";
import { NONE, commit, nextHighlight } from "./combobox";

describe("moving the highlight", () => {
  it("enters the list from the end the arrow points at", () => {
    // Down from nothing selects the first, Up the last. Anything else makes
    // the first keypress feel like it did nothing.
    expect(nextHighlight(NONE, 5, 1)).toBe(0);
    expect(nextHighlight(NONE, 5, -1)).toBe(4);
  });

  it("steps one at a time", () => {
    expect(nextHighlight(0, 5, 1)).toBe(1);
    expect(nextHighlight(3, 5, -1)).toBe(2);
  });

  it("wraps at both ends", () => {
    expect(nextHighlight(4, 5, 1)).toBe(0);
    expect(nextHighlight(0, 5, -1)).toBe(4);
  });

  it("has nowhere to go in an empty list", () => {
    // Filtering to nothing and then pressing an arrow must not point at an
    // option that is not there.
    expect(nextHighlight(NONE, 0, 1)).toBe(NONE);
    expect(nextHighlight(2, 0, -1)).toBe(NONE);
  });
});

describe("settling on a choice", () => {
  const matches = ["a", "b", "c"];

  it("takes the highlighted option when there is one", () => {
    expect(commit(matches, 1, undefined)).toBe("b");
  });

  it("accepts a name typed in full without touching the arrows", () => {
    // Typing a whole name and pressing Enter is a choice. Refusing it for
    // want of an arrow keypress would be pedantry.
    expect(commit(matches, NONE, "c")).toBe("c");
  });

  it("prefers the highlight over the typed text", () => {
    expect(commit(matches, 0, "c")).toBe("a");
  });

  it("settles on nothing when the text names nothing known", () => {
    // Not an error: it is someone entering a vehicle the table has never
    // heard of, which is what the field accepting free text is for.
    expect(commit(matches, NONE, undefined)).toBeNull();
    expect(commit([], NONE, undefined)).toBeNull();
  });
});
