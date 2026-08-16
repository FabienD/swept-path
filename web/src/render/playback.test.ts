import { describe, expect, it } from "vitest";
import type { PoseDto } from "../domain/types";
import {
  MAX_DURATION_S,
  MIN_DURATION_S,
  REVERSAL_PAUSE_S,
  playbackDuration,
  poseAt,
  positionAt,
  timelineOf,
  totalDuration,
} from "./playback";

/** A pose at `x`, on the axis, facing along it. */
const at = (x: number, reverse = false): PoseDto => ({
  x,
  y: 0,
  heading: 0,
  reverse,
  clearance: 1,
  overhanging: false,
});

/** Poses one metre apart, as an evenly sampled segment would give. */
const even = (count: number) => Array.from({ length: count }, (_, i) => at(i));

describe("the timeline of a path", () => {
  it("measures the distance travelled, not the number of poses", () => {
    const timeline = timelineOf(even(5));
    expect(timeline.length).toBeCloseTo(4, 9);
    expect([...timeline.marks]).toEqual([0, 1, 2, 3, 4]);
  });

  it("counts a reversal as distance, not as going backwards", () => {
    // `reverse` says which gear, not which way along the path. A metre driven
    // backwards still takes a metre of playback.
    const timeline = timelineOf([at(0), at(1), at(0, true)]);
    expect(timeline.length).toBeCloseTo(2, 9);
  });

  it("has nothing to play when the path is empty or a single pose", () => {
    expect(timelineOf([]).length).toBe(0);
    expect(timelineOf([at(0)]).length).toBe(0);
  });

  it("marks where the gear changes, as a fraction of the trip", () => {
    // `reverse` is a property of the segment arrived on, so the change falls
    // on the first pose driven backwards — three metres in, out of four.
    const timeline = timelineOf([at(0), at(1), at(2), at(3, true), at(4, true)]);
    expect(timeline.reversals).toHaveLength(1);
    expect(timeline.reversals[0]).toBeCloseTo(0.75, 9);
  });
});

describe("finding the pose at a position", () => {
  it("reads the position as distance, not as an index", () => {
    // The reason this module exists. `sample_arc` treats its step as an upper
    // bound, so segments are sampled at different densities: here four poses
    // crowd the first metre and two span the last three. Half way through the
    // *trip* is three poses in by index but two metres in by distance.
    const crowded = [at(0), at(0.25), at(0.5), at(0.75), at(1), at(4)];
    const timeline = timelineOf(crowded);
    expect(timeline.length).toBeCloseTo(4, 9);

    // Half way by distance is two metres in: the last pose before it sits at
    // one metre, and the next one overshoots to four.
    const middle = poseAt(timeline, 0.5);
    expect(timeline.marks[middle]).toBeCloseTo(1, 9);
    expect(timeline.marks[middle + 1]).toBeCloseTo(4, 9);

    // What reading by index would have given: three quarters of a metre into
    // a four-metre trip, and called it the middle.
    const byIndex = Math.round(0.5 * (crowded.length - 1));
    expect(timeline.marks[byIndex]).toBeCloseTo(0.75, 9);
  });

  it("lands on the ends exactly", () => {
    const timeline = timelineOf(even(5));
    expect(poseAt(timeline, 0)).toBe(0);
    expect(poseAt(timeline, 1)).toBe(4);
  });

  it("clamps a position off either end", () => {
    const timeline = timelineOf(even(5));
    expect(poseAt(timeline, -2)).toBe(0);
    expect(poseAt(timeline, 7)).toBe(4);
  });

  it("stays on the single pose of a path that goes nowhere", () => {
    // A zero length would divide by zero and hand the renderer a NaN index.
    expect(poseAt(timelineOf([at(0)]), 0.5)).toBe(0);
  });
});

describe("how long a playback runs", () => {
  it("takes longer for a longer trip", () => {
    expect(playbackDuration(12)).toBeGreaterThan(playbackDuration(6));
  });

  it("never runs so short it cannot be followed, nor so long it bores", () => {
    expect(playbackDuration(0.5)).toBe(MIN_DURATION_S);
    expect(playbackDuration(500)).toBe(MAX_DURATION_S);
  });

  it("adds a pause for every gear change", () => {
    const straight = timelineOf(even(5));
    const shuffled = timelineOf([at(0), at(1), at(2, true), at(3)]);
    expect(totalDuration(straight)).toBeCloseTo(playbackDuration(straight.length), 9);
    expect(totalDuration(shuffled)).toBeCloseTo(
      playbackDuration(shuffled.length) + 2 * REVERSAL_PAUSE_S,
      9,
    );
  });
});

describe("where playback has got to at a given moment", () => {
  const timeline = timelineOf([at(0), at(1), at(2), at(3, true), at(4, true)]);

  it("starts at the beginning and ends at the end", () => {
    expect(positionAt(timeline, 0)).toBe(0);
    expect(positionAt(timeline, totalDuration(timeline))).toBe(1);
  });

  it("holds still through the pause at a gear change", () => {
    // The moment the driver actually wants to see, and the one a continuous
    // reading skips over.
    const duration = playbackDuration(timeline.length);
    const arrival = 0.75 * duration;
    expect(positionAt(timeline, arrival + 0.01)).toBeCloseTo(0.75, 9);
    expect(positionAt(timeline, arrival + REVERSAL_PAUSE_S - 0.01)).toBeCloseTo(0.75, 9);
  });

  it("resumes where it paused, without losing ground", () => {
    const duration = playbackDuration(timeline.length);
    const after = 0.75 * duration + REVERSAL_PAUSE_S + 0.01;
    expect(positionAt(timeline, after)).toBeGreaterThan(0.75);
  });

  it("never goes backwards as time passes", () => {
    let previous = -1;
    for (let t = 0; t <= totalDuration(timeline); t += 0.05) {
      const now = positionAt(timeline, t);
      expect(now).toBeGreaterThanOrEqual(previous);
      previous = now;
    }
  });

  it("runs at a steady pace between the pauses", () => {
    // Equal slices of time cover equal distances, which is the whole point of
    // resolving on distance rather than on pose index.
    const plain = timelineOf(even(9));
    const duration = playbackDuration(plain.length);
    const quarter = positionAt(plain, duration / 4);
    const half = positionAt(plain, duration / 2);
    expect(quarter).toBeCloseTo(0.25, 9);
    expect(half).toBeCloseTo(0.5, 9);
  });
});
