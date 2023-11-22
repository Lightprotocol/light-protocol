import { defineConfig } from 'vite';
import { resolve } from 'pathe';
import dts from 'vite-plugin-dts';
// import { visualizer } from 'rollup-plugin-visualizer';
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";
export default defineConfig ({
    plugins: [dts({
        insertTypesEntry: true
    }),
    wasm(),
    topLevelAwait()
    //visualizer()
    ],
    build: {
        lib: {
            entry: resolve(__dirname, 'src/index.ts'),
            name: 'accountwasm',
            fileName: 'accountwasm',
            formats: ['es']
        }
    },
    
});