import { defineConfig, Options } from 'tsup';

export default defineConfig(options => {
    const commonOptions: Partial<Options> = {
        entry: {
            'prover.js': 'src/index.ts'
        },
        sourcemap: true,
        ...options
    }

    return [
        // Modern ESM
        {
            ...commonOptions,
            format: ['esm'],
            outExtension: () => ({ js: '.mjs' }),
            clean: true
        },

        // Browser-ready ESM, production + minified
        {
            ...commonOptions,
            entry: {
                'prover.js.browser': 'src/index.ts'
            },
            define: {
                'process.env.NODE_ENV': JSON.stringify('production')
            },
            format: ['esm'],
            outExtension: () => ({ js: '.mjs' }),
            minify: true
        },
        {
            ...commonOptions,
            format: 'cjs',
            outDir: './dist/cjs/',
            dts: true,
            outExtension: () => ({ js: '.cjs' })
        }
    ]
})