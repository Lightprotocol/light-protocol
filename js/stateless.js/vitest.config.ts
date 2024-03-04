import { defineConfig } from "vitest/config";
import { resolve } from "path";

export default defineConfig({
  logLevel: "info",
  test: {
    // Include both src and tests directories as well as inline tests
    include: ["src/**/__tests__/*.test.ts", "tests/**/*.test.ts"],
    includeSource: ["src/**/*.{js,ts}"],
  },
  define: {
    "import.meta.vitest": false,
  },
  build: {
    lib: {
      formats: ["es", "cjs"],
      entry: resolve(__dirname, "src/index.ts"),
      fileName: "index",
    },
  },
});