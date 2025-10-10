import { defineConfig } from 'vitest/config';
import { resolve } from 'path';

export default defineConfig({
    logLevel: 'info',
    test: {
        // Use forks pool with singleFork to run all tests in one process
        pool: 'forks',
        // Run all tests sequentially (no parallel test files)
        fileParallelism: false,
        poolOptions: {
            forks: {
                // Run all tests sequentially in a single forked process
                singleFork: true,
                // Set max heap size to 4GB
                execArgv: ['--max-old-space-size=4096'],
            },
        },
        include: process.env.EXCLUDE_E2E
            ? []
            : ['src/**/__tests__/*.test.ts', 'tests/**/*.test.ts'],
        includeSource: ['src/**/*.{js,ts}'],
        exclude: ['src/program.ts'],
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
