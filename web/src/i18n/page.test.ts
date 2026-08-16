import { describe, expect, it } from "vitest";
// Vite's raw import, rather than node:fs: it needs no @types/node, and it
// reads the same file the build ships.
import page from "../../index.html?raw";
import { TEXT_KEYS } from "./dictionary";

/**
 * The page itself, read as text.
 *
 * There is no DOM in these tests and this batch does not add one. But the
 * markup carries two contracts the TypeScript compiler cannot see — which
 * key a label translates by, and which unit a field is in — and both fail
 * silently when broken: a missing key renders blank, a missing magnitude
 * stops converting and sends inches to a solver expecting metres.
 */
const attributes = (name: string): string[] => [
  ...page.matchAll(new RegExp(`${name}="([^"]+)"`, "g")),
].map((match) => match[1]!);

describe("the keys the page asks for", () => {
  it("all exist in the dictionary", () => {
    const known = new Set<string>(TEXT_KEYS);
    const asked = [
      ...attributes("data-i18n"),
      ...attributes("data-i18n-placeholder"),
      ...attributes("data-i18n-aria"),
    ];
    expect(asked.filter((key) => !known.has(key))).toEqual([]);
  });

  it("are enough that no French is left hard-coded in a label", () => {
    // Any label still holding its own text would stay French on an English
    // page — and it is the labels, not the sentences, that are read first.
    const hardCoded = [
      ...page.matchAll(/<(span|legend|option|summary)(?![^>]*data-i18n)[^>]*>([^<>]+)</g),
    ]
      .map((match) => match[2]!.trim())
      .filter((body) => /[a-zà-ÿ]{4,}/i.test(body))
      // The language names stay in their own language, as they should.
      .filter((body) => !["Français", "English"].includes(body));
    expect(hardCoded).toEqual([]);
  });
});

describe("the units the page declares", () => {
  it("marks every length field with what it measures", () => {
    // A field left unmarked is read as metres whatever the reader typed. The
    // two exceptions carry no length at all.
    // An angle, a percentage, and a position along the path: none is a length.
    const unitless = new Set(["hinge-depth", "open-angle", "scrub"]);
    const missing = [...page.matchAll(/<input([^>]*type="(?:number|range)"[^>]*)>/g)]
      .map((match) => match[1]!)
      .filter((tag) => !tag.includes("data-magnitude"))
      .map((tag) => /id="([^"]+)"/.exec(tag)?.[1] ?? "?")
      .filter((id) => !unitless.has(id));
    expect(missing).toEqual([]);
  });

  it("uses only magnitudes the converter knows", () => {
    const known = new Set(["clearance", "dimension", "distance"]);
    expect(attributes("data-magnitude").filter((m) => !known.has(m))).toEqual([]);
    expect(attributes("data-unit").filter((m) => !known.has(m))).toEqual([]);
  });
});
