import { defineConfig } from 'vitest/config';
import { resolve } from 'path';

export default defineConfig({
    logLevel: 'info',
    test: {
        // Use forks pool for better native module support (LiteSVM)
        // Threads pool can cause std::bad_alloc with non-thread-safe native code
        pool: 'forks',
        poolOptions: {
            forks: {
                maxForks: 1,
                minForks: 1,
                // Recycle worker after each test file to prevent GC corruption
                // This kills and restarts the process, wiping all memory
                memoryLimit: 1,
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
