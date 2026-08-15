import type {
  SceneDto,
  SolveRequest,
  SolveResponse,
  WorkerOut,
} from "../domain/types";

/** Error code reported to whoever was awaiting an abandoned search. */
export const CANCELLED = "cancelled";

/** How a worker is created. Injectable so the client can be tested. */
export type WorkerFactory = () => Worker;

function defaultFactory(): Worker {
  return new Worker(new URL("./solver.worker.ts", import.meta.url), {
    type: "module",
  });
}

/**
 * Talks to the solver worker.
 *
 * Cancellation is termination: starting a new search kills the worker still
 * running the old one. The core has no notion of being interrupted and does
 * not need one — which is why it stayed free of any clock.
 *
 * Terminating a worker does not settle the promise waiting on it, though, so
 * the client rejects it explicitly. Without that, anything awaiting the
 * search — a `finally` clearing a busy flag, say — would never run, and the
 * interface would stay stuck on a calculation that had already been thrown
 * away.
 */
export class SolverClient {
  #worker: Worker | null = null;
  #abandon: ((reason: unknown) => void) | null = null;
  #nextId = 0;
  readonly #factory: WorkerFactory;

  constructor(factory: WorkerFactory = defaultFactory) {
    this.#factory = factory;
  }

  /** Sends one message and resolves on its reply, abandoning any search in
   *  flight. */
  #exchange<T>(
    build: (id: number) => unknown,
    take: (out: WorkerOut) => T | undefined,
    watch?: (out: WorkerOut) => void,
  ): Promise<T> {
    this.cancel();
    const worker = this.#factory();
    this.#worker = worker;
    const id = ++this.#nextId;

    return new Promise<T>((resolve, reject) => {
      this.#abandon = reject;
      const settle = (act: () => void) => {
        this.#abandon = null;
        act();
      };

      worker.onmessage = (event: MessageEvent<WorkerOut>) => {
        const out = event.data;
        if (out.id !== id) return;
        if (out.kind === "failed") {
          settle(() => {
            reject(out.error);
          });
          return;
        }
        // Watched before taken: a message that settles nothing still says
        // something. Only while this search is the current one — a terminated
        // worker can still deliver what it had already queued, and that must
        // not drive the interface of the search that replaced it.
        if (this.#worker === worker) watch?.(out);
        const value = take(out);
        if (value !== undefined) {
          settle(() => {
            resolve(value);
          });
        }
      };
      worker.onerror = () => {
        settle(() => {
          reject({ code: "worker_failed", field: null });
        });
      };
      worker.postMessage(build(id));
    });
  }

  /** Runs a search, reporting planner progress if a watcher is given. */
  solve(
    request: SolveRequest,
    onProgress?: (moves: number, expanded: number, budget: number) => void,
  ): Promise<SolveResponse> {
    return this.#exchange(
      (id) => ({ kind: "solve", id, request }),
      (out) => (out.kind === "solved" ? out.response : undefined),
      (out) => {
        if (out.kind === "progress") {
          onProgress?.(out.moves, out.expanded, out.budget);
        }
      },
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

  /** Whether a search is currently running. */
  get busy(): boolean {
    return this.#abandon !== null;
  }

  /**
   * Kills the worker and rejects whatever was awaiting it.
   *
   * Safe to call when nothing is running.
   */
  cancel(): void {
    this.#worker?.terminate();
    this.#worker = null;
    const abandon = this.#abandon;
    this.#abandon = null;
    abandon?.({ code: CANCELLED, field: null });
  }
}
