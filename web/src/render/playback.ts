/**
 * Playing a manoeuvre back at a steady pace.
 *
 * The store already carries a `position` from 0 to 1 and the plan redraws
 * from it, so playing a manoeuvre is a matter of advancing that number over
 * time. What this module supplies is the part that is not obvious: *which*
 * number, at *which* moment.
 *
 * Position resolves on distance travelled, never on pose index. The two are
 * not the same thing here: `sample_arc` in the core treats its step as an
 * upper bound (`count = ceil(distance / step)`, then an even spread inside
 * the segment), so spacing is uniform within a segment but varies between
 * them, anywhere from half a step to a full one. Reading by index would make
 * playback crawl through finely sampled segments and rush the coarse ones —
 * and it is what makes the scrubber's half-way point not the half-way point
 * of the trip.
 *
 * Everything here is pure, so it is tested without a DOM. Only the clock that
 * drives it lives in `main.ts`.
 */
import type { PoseDto } from "../domain/types";

/**
 * How fast playback covers ground, in metres per second.
 *
 * ARBITRARY. Near the pace of a real manoeuvre, which is what makes a tight
 * passage feel tight rather than look tight.
 */
export const PLAYBACK_SPEED_M_S = 1.2;

/**
 * Bounds on how long a playback runs, in seconds.
 *
 * ARBITRARY. Proportional time is the honest reading — a longer trip takes
 * longer — but unbounded it makes a five-move entry a chore and a one-metre
 * nudge a blink.
 */
export const MIN_DURATION_S = 4;
export const MAX_DURATION_S = 20;

/**
 * How long playback holds still at a change of direction, in seconds.
 *
 * ARBITRARY. This is the moment the driver came to see — where they have to
 * stop, change gear and start again — and a continuous reading slides over it
 * without ever showing it.
 */
export const REVERSAL_PAUSE_S = 0.4;

export interface Timeline {
  /** Distance from the start at each pose, in metres. Starts at zero. */
  readonly marks: readonly number[];
  /** Distance travelled in all, in metres. */
  readonly length: number;
  /**
   * Where the gear changes, as fractions of the trip.
   *
   * Fractions rather than indices, because that is what playback and the
   * position both speak in.
   */
  readonly reversals: readonly number[];
}

/** Measures a path: how far along each pose sits, and where the gear changes. */
export function timelineOf(poses: readonly PoseDto[]): Timeline {
  if (poses.length < 2) {
    return { marks: poses.length === 1 ? [0] : [], length: 0, reversals: [] };
  }

  const marks: number[] = [0];
  const changes: number[] = [];
  let travelled = 0;
  for (let i = 1; i < poses.length; i++) {
    const from = poses[i - 1]!;
    const to = poses[i]!;
    // Straight-line distance between successive poses. They are close enough
    // together that the arc between them differs by far less than a pixel.
    travelled += Math.hypot(to.x - from.x, to.y - from.y);
    marks.push(travelled);
    // `reverse` is which gear, not which way along the path: a metre driven
    // backwards is still a metre of playback. Only the change matters here.
    if (to.reverse !== from.reverse) changes.push(travelled);
  }

  return {
    marks,
    length: travelled,
    reversals: travelled > 0 ? changes.map((d) => d / travelled) : [],
  };
}

/** The pose sitting at `position` along the trip, as an index. */
export function poseAt(timeline: Timeline, position: number): number {
  const last = timeline.marks.length - 1;
  if (last <= 0) return 0;

  const wanted = Math.min(Math.max(position, 0), 1) * timeline.length;

  // Binary search for the last mark at or before the distance wanted. The
  // marks ascend, and a path runs to a few hundred poses, so this keeps
  // playback's per-frame cost off the path length.
  let low = 0;
  let high = last;
  while (low < high) {
    const middle = Math.ceil((low + high) / 2);
    if (timeline.marks[middle]! <= wanted) low = middle;
    else high = middle - 1;
  }
  return low;
}

/** How long the driving part of a playback takes, in seconds. */
export function playbackDuration(length: number): number {
  const wanted = length / PLAYBACK_SPEED_M_S;
  return Math.min(Math.max(wanted, MIN_DURATION_S), MAX_DURATION_S);
}

/** How long a whole playback takes, pauses included, in seconds. */
export function totalDuration(timeline: Timeline): number {
  return (
    playbackDuration(timeline.length) + timeline.reversals.length * REVERSAL_PAUSE_S
  );
}

/**
 * How far along the trip playback has got after `elapsed` seconds.
 *
 * Time spent paused does not advance the position, so the pauses lengthen the
 * playback rather than compress the driving between them.
 */
export function positionAt(timeline: Timeline, elapsed: number): number {
  const duration = playbackDuration(timeline.length);
  if (duration <= 0) return 1;

  let paused = 0;
  for (const fraction of timeline.reversals) {
    // When this reversal is reached, counting the pauses already served.
    const arrival = fraction * duration + paused * REVERSAL_PAUSE_S;
    if (elapsed < arrival) break;
    if (elapsed < arrival + REVERSAL_PAUSE_S) return fraction;
    paused += 1;
  }

  const driven = elapsed - paused * REVERSAL_PAUSE_S;
  return Math.min(Math.max(driven / duration, 0), 1);
}

/**
 * The moment playback reaches `position` — the inverse of [`positionAt`].
 *
 * What lets a paused playback resume where it stopped instead of starting
 * over, which is the difference between pausing to look at something and
 * losing your place.
 */
export function elapsedFor(timeline: Timeline, position: number): number {
  const clamped = Math.min(Math.max(position, 0), 1);
  // Only the pauses already served count. A position sitting exactly on a
  // reversal lands at the start of its pause, not the end of it.
  let paused = 0;
  for (const fraction of timeline.reversals) {
    if (fraction >= clamped) break;
    paused += 1;
  }
  return clamped * playbackDuration(timeline.length) + paused * REVERSAL_PAUSE_S;
}
