import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [tailwindcss()],
  build: {
    // The Wasm module is fetched at runtime, not inlined: inlining a binary
    // as base64 would inflate it by a third and block first paint on it.
    assetsInlineLimit: 0,
    target: "es2022",
    rollupOptions: {
      // Three static pages rather than a router: the two prose pages need
      // neither the solver nor the Wasm module, and a visitor who follows a
      // link to read a disclaimer should not download a planner to do it.
      // Relative to the Vite root, which needs no node:path import.
      input: {
        main: "index.html",
        documentation: "documentation.html",
        disclaimer: "disclaimer.html",
      },
    },
  },
  worker: {
    format: "es",
  },
});
