import { defineConfig } from 'vitest/config';

export default defineConfig({
    test: {
        include: ['tests/**/*.test.ts'],
        exclude: process.env.EXCLUDE_E2E ? ['tests/e2e/**'] : [],
        testTimeout: 30000,
        hookTimeout: 20000,
        reporters: ['verbose'],
    },
});
