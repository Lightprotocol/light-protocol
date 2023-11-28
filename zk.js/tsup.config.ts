import { defineConfig, Options } from 'tsup';

export default defineConfig(options => {
    const commonOptions: Partial<Options> = {
        entry: {
            'zk.js': 'src/index.ts'
        },
        // external: ['@coral-xyz/anchor'],
        // noExternal: ['@coral-xyz/anchor', 'zlib'],
        sourcemap: true,
        minify: false,
        ...options
    }

    return [
        // Modern ESM
        {
            ...commonOptions,
            format: ['esm'],
            outExtension: () => ({ js: '.mjs' }),
            // experimentalDts: true,
            dts: true,
            clean: true
        },
        // Support Webpack 4 by pointing `"module"` to a file with a `.js` extension
        // and optional chaining compiled away
        {
            ...commonOptions,
            entry: {
                'zk.js.legacy-esm': 'src/index.ts'
            },
            format: ['esm'],
            outExtension: () => ({ js: '.js' }),
            target: 'es2017'
        },
        // Browser-ready ESM, production + minified
        {
            ...commonOptions,
            entry: {
                'zk.js.browser': 'src/index.ts'
            },
            define: {
                'process.env.NODE_ENV': JSON.stringify('production')
            },
            format: ['esm'],
            outExtension: () => ({ js: '.mjs' }),
            // experimentalDts: true,
        },
        {
            ...commonOptions,
            format: 'cjs',
            outDir: './dist/cjs/',
            outExtension: () => ({ js: '.cjs' })
        }
    ]
})