import { defineConfig } from 'vitest/config';

export default defineConfig({
    test: {
        include: ['tests/e2e/**/*.test.ts'],
        fileParallelism: false,
        testTimeout: 120_000,
        hookTimeout: 60_000,
        reporters: ['verbose'],
    },
});
