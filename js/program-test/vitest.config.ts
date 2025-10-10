import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // litesvm fails with bad alloc if not configured
    // Use threads pool instead of forks to avoid native addon corruption
    // Threads share the same V8 isolate and native addon context
    pool: "threads",
    // Run all tests sequentially (no parallel test files)
    fileParallelism: false,
    poolOptions: {
      threads: {
        // Run all tests sequentially in a single thread
        singleThread: true,
      },
    },
    exclude: ["**/node_modules/**", "**/dist/**"],
  },
});
