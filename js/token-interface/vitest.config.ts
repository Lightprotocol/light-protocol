import { defineConfig } from 'vitest/config';
import { resolve } from 'path';

export default defineConfig({
    logLevel: 'info',
    test: {
        include: process.env.EXCLUDE_E2E
            ? ['tests/unit/**/*.test.ts']
            : ['tests/**/*.test.ts'],
        includeSource: ['src/**/*.{js,ts}'],
        fileParallelism: false,
        testTimeout: 350000,
        hookTimeout: 100000,
        reporters: ['verbose'],
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
