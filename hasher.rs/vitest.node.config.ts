import { defineConfig, mergeConfig } from 'vitest/config';
import viteConfig from './vitest.config';

export default mergeConfig(viteConfig, defineConfig({
    resolve: {
        conditions: ["node-addons"]
    },
}));
