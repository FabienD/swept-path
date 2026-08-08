import type {
  SceneDto,
  SolveRequest,
  SolveResponse,
  WorkerOut,
} from "../domain/types";

/**
 * Talks to the solver worker.
 *
 * Cancellation is termination: starting a new search kills the worker still
 * running the old one. The core has no notion of being interrupted and does
 * not need one — which is why it stayed free of any clock.
 */
export class SolverClient {
  #worker: Worker | null = null;
  #nextId = 0;

  #spawn(): Worker {
    return new Worker(new URL("./solver.worker.ts", import.meta.url), {
      type: "module",
    });
  }

  /** Sends one message and resolves on its reply, abandoning any search in
   *  flight. */
  #exchange<T>(
    build: (id: number) => unknown,
    take: (out: WorkerOut) => T | undefined,
  ): Promise<T> {
    this.cancel();
    const worker = this.#spawn();
    this.#worker = worker;
    const id = ++this.#nextId;

    return new Promise<T>((resolve, reject) => {
      worker.onmessage = (event: MessageEvent<WorkerOut>) => {
        const out = event.data;
        if (out.id !== id) return;
        if (out.kind === "failed") {
          reject(out.error);
          return;
        }
        const value = take(out);
        if (value !== undefined) resolve(value);
      };
      worker.onerror = () => {
        reject({ code: "worker_failed", field: null });
      };
      worker.postMessage(build(id));
    });
  }

  /** Runs a search. */
  solve(request: SolveRequest): Promise<SolveResponse> {
    return this.#exchange(
      (id) => ({ kind: "solve", id, request }),
      (out) => (out.kind === "solved" ? out.response : undefined),
    );
  }

  /** The narrowest carriageway admitting a one-move forward entry. */
  minRoad(request: SolveRequest): Promise<number | null> {
    return this.#exchange(
      (id) => ({ kind: "minRoad", id, request }),
      (out) => (out.kind === "minRoad" ? out.response : undefined),
    );
  }

  /** The widest opening angle the leaves can hold, in radians. */
  maxGateAngle(scene: SceneDto): Promise<number> {
    return this.#exchange(
      (id) => ({ kind: "maxGateAngle", id, scene }),
      (out) => (out.kind === "maxGateAngle" ? out.radians : undefined),
    );
  }

  /** Kills the worker, abandoning whatever it was doing. */
  cancel(): void {
    this.#worker?.terminate();
    this.#worker = null;
  }
}
