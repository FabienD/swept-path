import { describe, expect, it } from "vitest";
import table from "../../../data/vehicles.json";
import { BODY_TO_TRACK_M, trackOf } from "./track";

/** Every entry whose maker published a front track, with its body width. */
const measured = table.vehicles.flatMap((entry) => {
  const track = entry.track_front.v;
  const width = entry.width.v;
  return track === null || width === null
    ? []
    : [{ label: `${entry.make} ${entry.model}`, track, width }];
});

describe("the track estimate", () => {
  it("has something to be held against", () => {
    expect(measured.length).toBeGreaterThanOrEqual(4);
  });

  it("lands within seven centimetres of every published track", () => {
    // The loosest is the 991 Carrera 4: a wide body over the standard front
    // track, so its gap is the widest in the table. Seven centimetres of track
    // is three and a half on the pivot radius, which the solver rounds away.
    for (const { label, track, width } of measured) {
      expect(Math.abs(trackOf(width) - track), label).toBeLessThanOrEqual(0.07);
    }
  });

  it("sits inside the spread rather than beside it", () => {
    // Guards the direction as well as the size: an estimate outside the range
    // of every measured gap would be wrong for all of them at once.
    const gaps = measured.map((m) => m.width - m.track);
    expect(BODY_TO_TRACK_M).toBeGreaterThanOrEqual(Math.min(...gaps));
    expect(BODY_TO_TRACK_M).toBeLessThanOrEqual(Math.max(...gaps));
  });
});
