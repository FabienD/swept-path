import { describe, expect, it, vi } from "vitest";
import type { SolveRequest } from "../domain/types";
import { CANCELLED, SolverClient } from "./client";

/** A worker that records what it is sent and never answers on its own. */
class SilentWorker {
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;
  posted: unknown[] = [];
  terminated = false;

  postMessage(message: unknown) {
    this.posted.push(message);
  }
  terminate() {
    this.terminated = true;
  }
  /** Lets a test play the worker's reply. */
  reply(message: unknown) {
    this.onmessage?.({ data: message } as MessageEvent);
  }
}

const request = {} as SolveRequest;

describe("solver client", () => {
  it("resolves when the worker answers", async () => {
    const worker = new SilentWorker();
    const client = new SolverClient(() => worker as unknown as Worker);

    const pending = client.solve(request);
    worker.reply({ kind: "solved", id: 1, response: { alternatives: [], budget_exhausted: false } });

    await expect(pending).resolves.toEqual({ alternatives: [], budget_exhausted: false });
  });

  it("rejects the pending search when cancelled", async () => {
    // Terminating a worker leaves its promise pending for ever, so anything
    // awaiting it — a `finally` clearing a busy flag, say — never runs.
    const worker = new SilentWorker();
    const client = new SolverClient(() => worker as unknown as Worker);

    const pending = client.solve(request);
    client.cancel();

    await expect(pending).rejects.toMatchObject({ code: CANCELLED });
    expect(worker.terminated).toBe(true);
  });

  it("rejects the previous search when a new one starts", async () => {
    const workers: SilentWorker[] = [];
    const client = new SolverClient(() => {
      const worker = new SilentWorker();
      workers.push(worker);
      return worker as unknown as Worker;
    });

    const first = client.solve(request);
    const second = client.solve(request);

    await expect(first).rejects.toMatchObject({ code: CANCELLED });
    expect(workers[0]?.terminated).toBe(true);

    workers[1]?.reply({ kind: "solved", id: 2, response: { alternatives: [], budget_exhausted: true } });
    await expect(second).resolves.toMatchObject({ budget_exhausted: true });
  });

  it("ignores replies meant for an earlier request", async () => {
    const worker = new SilentWorker();
    const client = new SolverClient(() => worker as unknown as Worker);

    const pending = client.solve(request);
    const settled = vi.fn();
    void pending.then(settled, settled);

    worker.reply({ kind: "solved", id: 999, response: { alternatives: [], budget_exhausted: false } });
    await Promise.resolve();

    expect(settled).not.toHaveBeenCalled();
  });

  it("cancelling twice is harmless", () => {
    const client = new SolverClient(() => new SilentWorker() as unknown as Worker);
    expect(() => {
      client.cancel();
      client.cancel();
    }).not.toThrow();
  });
});
