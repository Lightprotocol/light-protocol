import { defineConfig, mergeConfig } from 'vitest/config';

export default defineConfig({
    test: {
        globals: true,
        threads: false
    }
});
