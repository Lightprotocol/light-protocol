import { defineConfig } from 'vitest/config';
import { resolve } from 'path';

export default defineConfig({
  logLevel: 'info',
  test: {
    include: process.env.EXCLUDE_E2E
      ? []
      : ['src/**/__tests__/*.test.ts', 'tests/**/*.test.ts'],
    includeSource: ['src/**/*.{js,ts}'],
    exclude: ['**/src/state/hash.ts'],
    testTimeout: 350000,
  },
  define: {
    'import.meta.vitest': false,
  },
  build: {
    lib: {
      formats: ['es', 'cjs'],
      entry: resolve(__dirname, 'src/index.ts'),
      fileName: 'index',
    },
  },
});
