import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Include both src and tests directories
    include: ["src/**/__tests__/*.test.ts", "tests/**/*.test.ts"],
  },
});
