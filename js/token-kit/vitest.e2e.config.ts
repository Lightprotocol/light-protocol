import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
    resolve: {
        alias: {
            // Resolve to source so compressed-token and token-kit share a
            // single featureFlags instance (the dist bundles each get their
            // own copy with __BUILD_VERSION__ baked to V1).
            '@lightprotocol/stateless.js': path.resolve(
                __dirname,
                '../stateless.js/src/index.ts',
            ),
            '@lightprotocol/compressed-token': path.resolve(
                __dirname,
                '../compressed-token/src/index.ts',
            ),
        },
    },
    test: {
        include: ['tests/e2e/**/*.test.ts'],
        fileParallelism: false,
        testTimeout: 120_000,
        hookTimeout: 60_000,
        reporters: ['verbose'],
        env: {
            LIGHT_PROTOCOL_VERSION: 'V2',
            LIGHT_PROTOCOL_BETA: 'true',
        },
    },
});
