import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [tailwindcss()],
  build: {
    // The Wasm module is fetched at runtime, not inlined: inlining a binary
    // as base64 would inflate it by a third and block first paint on it.
    assetsInlineLimit: 0,
    target: "es2022",
  },
  worker: {
    format: "es",
  },
});
