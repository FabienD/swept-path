/** A subscriber, called after any change that actually changed something. */
export type Listener = () => void;

/** The smallest observable store that does the job. */
export interface Store<T> {
  get(): Readonly<T>;
  /**
   * Merges a patch. Silent when nothing changes: the whole SVG is rebuilt on
   * every notification, so a spurious one costs real work.
   */
  set(patch: Partial<T>): void;
  subscribe(listener: Listener): () => void;
}

export function createStore<T extends object>(initial: T): Store<T> {
  let state = { ...initial };
  const listeners = new Set<Listener>();

  return {
    get: () => state,
    set(patch) {
      const changed = Object.entries(patch).some(
        ([key, value]) => state[key as keyof T] !== value,
      );
      if (!changed) return;
      state = { ...state, ...patch };
      for (const listener of listeners) listener();
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
