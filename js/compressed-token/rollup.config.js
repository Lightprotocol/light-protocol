import typescript from '@rollup/plugin-typescript';
import nodePolyfills from 'rollup-plugin-polyfill-node';
import dts from 'rollup-plugin-dts';
import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import alias from '@rollup/plugin-alias';
import json from '@rollup/plugin-json';

const rolls = (fmt, env) => ({
    input: 'src/index.ts',
    output: {
        dir: `dist/${fmt}/${env}`,
        format: fmt,
        entryFileNames: `[name].${fmt === 'cjs' ? 'cjs' : 'js'}`,
        sourcemap: true,
    },
    external: [
        '@solana/web3.js',
        '@coral-xyz/anchor',
        '@lightprotocol/stateless.js',
    ],
    plugins: [
        json(),
        typescript({
            target: fmt === 'es' ? 'ES2022' : 'ES2017',
            outDir: `dist/${fmt}/${env}`,
            rootDir: 'src',
        }),
        commonjs(),
        resolve({
            browser: env === 'browser',
            preferBuiltins: env === 'node',
            extensions: ['.mjs', '.js', '.json', '.ts'],
            mainFields: ['module', 'main', 'browser'],
        }),
        alias({
            entries: [
                {
                    find: 'crypto',
                    replacement:
                        env === 'browser' ? 'crypto-browserify' : 'crypto',
                },
            ],
        }),
        env === 'browser' ? nodePolyfills() : undefined,
    ].filter(Boolean),
    onwarn(warning, warn) {
        if (warning.code !== 'CIRCULAR_DEPENDENCY') {
            warn(warning);
        }
    },
});

const typesConfig = {
    input: 'src/index.ts',
    output: [{ file: 'dist/types/index.d.ts', format: 'es' }],
    plugins: [dts()],
};

export default [
    rolls('cjs', 'browser'),
    rolls('es', 'browser'),
    rolls('cjs', 'node'),
    typesConfig,
];
