import { defineConfig } from "vitest/config";

// Unit tests run in a plain Node environment. The core logic we test
// (the spaced-repetition scheduler and cloze parsing) is pure and has no
// dependency on Tauri or the DOM, so no browser environment is needed.
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
    globals: false,
  },
});
