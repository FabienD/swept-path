/// <reference lib="webworker" />
import init, {
  max_gate_angle,
  min_road,
  solve,
} from "../generated/swept_wasm.js";
import type { ErrorDto, WorkerIn, WorkerOut } from "../domain/types";

let ready: Promise<unknown> | null = null;

/** Loads the Wasm module once, lazily. */
function ensureReady(): Promise<unknown> {
  ready ??= init();
  return ready;
}

/** Anything the boundary throws that is not already an ErrorDto. */
function asError(thrown: unknown): ErrorDto {
  if (
    typeof thrown === "object" &&
    thrown !== null &&
    "code" in thrown &&
    typeof (thrown as ErrorDto).code === "string"
  ) {
    return thrown as ErrorDto;
  }
  return { code: "unexpected", field: null };
}

self.onmessage = async (event: MessageEvent<WorkerIn>) => {
  const message = event.data;
  const post = (out: WorkerOut) => {
    self.postMessage(out);
  };

  try {
    await ensureReady();
    switch (message.kind) {
      case "solve":
        post({
          kind: "solved",
          id: message.id,
          response: solve(message.request),
        });
        break;
      case "minRoad":
        post({
          kind: "minRoad",
          id: message.id,
          response: min_road(message.request),
        });
        break;
      case "maxGateAngle":
        post({
          kind: "maxGateAngle",
          id: message.id,
          radians: max_gate_angle(message.scene),
        });
        break;
    }
  } catch (thrown) {
    post({ kind: "failed", id: message.id, error: asError(thrown) });
  }
};
