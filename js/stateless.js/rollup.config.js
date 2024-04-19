import typescript from '@rollup/plugin-typescript';
import nodePolyfills from 'rollup-plugin-polyfill-node';
import resolve from '@rollup/plugin-node-resolve';
import dts from 'rollup-plugin-dts';
import babel from '@rollup/plugin-babel';
import replace from '@rollup/plugin-replace';
import commonjs from '@rollup/plugin-commonjs';
import json from '@rollup/plugin-json';

const basePlugins = [
    typescript({ tsconfig: './tsconfig.json' }),
    babel({
        babelHelpers: 'bundled',
        extensions: ['.ts'],
    }),
];

const nodeConfig = {
    input: 'src/index.ts',
    output: [
        { file: 'dist/cjs/index.cjs', format: 'cjs', sourcemap: true },
        { file: 'dist/es/index.js', format: 'es', sourcemap: true },
    ],
    external: [
        'bs58',
        'node-fetch',
        '@solana/web3.js',
        '@coral-xyz/anchor',
        'tweetnacl',
    ],
    plugins: [
        resolve({
            preferBuiltins: true,
        }),
        replace({
            'process.env.BROWSER': JSON.stringify(false),
            preventAssignment: true,
        }),
        ...basePlugins,
    ],
};

const browserConfig = {
    input: 'src/index.ts',
    output: [{ file: 'dist/browser/index.js', format: 'es', sourcemap: true }],
    external: [
        // 'bs58',
        // '@solana/web3.js',
        // '@coral-xyz/anchor',
        // 'tweetnacl',
    ],
    plugins: [
        resolve({
            mainFields: ['browser', 'module', 'main'],
            browser: true,
            preferBuiltins: false,
        }),
        ...basePlugins,
        nodePolyfills(),
        commonjs(),
        replace({
            'process.env.BROWSER': JSON.stringify(true),
            preventAssignment: true,
        }),
        json(),
    ],
};

const typesConfig = {
    input: 'src/index.ts',
    output: [{ file: 'dist/types/index.d.ts', format: 'es' }],
    plugins: [dts()],
};

export default [nodeConfig, browserConfig, typesConfig];
