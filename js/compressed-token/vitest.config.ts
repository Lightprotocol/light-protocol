import { defineConfig } from 'vitest/config';
import { resolve } from 'path';

export default defineConfig({
    logLevel: 'info',
    test: {
        // Use vmForks pool to enable memoryLimit for worker recycling
        // This prevents GC corruption from std::bad_alloc issues
        pool: 'vmForks',
        // Run all tests sequentially (no parallel test files)
        fileParallelism: false,
        poolOptions: {
            vmForks: {
                maxForks: 1,
                minForks: 1,
                // Recycle worker when it exceeds 100MB to prevent GC corruption
                memoryLimit: '100MB',
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
