/* global process */
import typescript from '@rollup/plugin-typescript';
import nodePolyfills from 'rollup-plugin-polyfill-node';
import dts from 'rollup-plugin-dts';
import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import terser from '@rollup/plugin-terser';
import replace from '@rollup/plugin-replace';
import json from '@rollup/plugin-json';

const rolls = (fmt, env) => ({
    input: 'src/index.ts',
    output: {
        dir: `dist/${fmt}/${env}`,
        format: fmt,
        entryFileNames: `[name].${fmt === 'cjs' ? 'cjs' : 'js'}`,
        sourcemap: true,
    },
    external: ['@solana/web3.js'],
    plugins: [
        replace({
            preventAssignment: true,
            values: {
                __BUILD_VERSION__: JSON.stringify(
                    process.env.LIGHT_PROTOCOL_VERSION || 'V1',
                ),
            },
        }),
        typescript({
            target: fmt === 'es' ? 'ES2022' : 'ES2017',
            outDir: `dist/${fmt}/${env}`,
            rootDir: 'src',
        }),
        commonjs(),
        resolve({
            browser: env === 'browser',
            preferBuiltins: env === 'node',
        }),
        env === 'browser' ? nodePolyfills() : undefined,
        json(),
        terser({
            compress: {
                drop_console: false,
                drop_debugger: true,
                passes: 3,
                booleans_as_integers: true,
                keep_fargs: false,
                keep_fnames: false,
                keep_infinity: true,
                reduce_funcs: true,
                reduce_vars: true,
            },
            mangle: {
                toplevel: true,
            },
            output: {
                comments: false,
            },
        }),
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
    rolls('cjs', 'node'),
    rolls('es', 'browser'),
    typesConfig,
];
