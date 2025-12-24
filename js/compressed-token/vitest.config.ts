import { defineConfig } from 'vitest/config';
import { resolve } from 'path';

export default defineConfig({
    logLevel: 'info',
    test: {
        include: process.env.EXCLUDE_E2E
            ? []
            : ['src/**/__tests__/*.test.ts', 'tests/**/*.test.ts'],
        includeSource: ['src/**/*.{js,ts}'],
        exclude: ['src/program.ts'],
        // e2e tests share a single local validator instance; running files in parallel can
        // overflow on-chain queues and lead to nondeterministic ProgramError failures.
        fileParallelism: false,
        testTimeout: 350000,
        hookTimeout: 100000,
        reporters: ['verbose'],
        globalSetup: './tests/setup/version-check.ts',
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
