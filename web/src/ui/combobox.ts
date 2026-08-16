/**
 * The part of a combobox that is not the DOM.
 *
 * Written by hand rather than left to `<datalist>`, so the behaviour has to be
 * written too — and a hand-made combobox is the component most often got
 * wrong. Keeping the state here means the awkward parts (what the arrows do at
 * the ends, what Enter commits, what an unknown name means) are decided in
 * one place and tested without a browser. `main.ts` is left with listeners and
 * attributes.
 */

/** No option highlighted. Typing leaves the list in this state on purpose. */
export const NONE = -1;

/**
 * Where the highlight lands after pressing an arrow.
 *
 * Wraps at both ends, and enters the list from the matching end: pressing
 * Down with nothing highlighted goes to the first option, Up to the last.
 * Returns [`NONE`] when there is nothing to move through, so a caller never
 * has to special-case an empty list.
 */
export function nextHighlight(current: number, count: number, delta: number): number {
  if (count <= 0) return NONE;
  if (current === NONE) return delta > 0 ? 0 : count - 1;
  return (((current + delta) % count) + count) % count;
}

/**
 * What pressing Enter, or leaving the field, should settle on.
 *
 * The highlighted option if there is one. Otherwise the typed text, but only
 * when it names exactly one thing: an unambiguous name typed in full is a
 * choice, and refusing it because the visitor never touched the arrows would
 * be pedantry.
 *
 * `null` means the text names nothing known — which is not an error. It is
 * someone entering a vehicle the table has never heard of, which is the whole
 * point of the field accepting free text.
 */
export function commit<T>(
  matches: readonly T[],
  highlighted: number,
  exact: T | undefined,
): T | null {
  const picked = matches[highlighted];
  if (picked !== undefined) return picked;
  return exact ?? null;
}
