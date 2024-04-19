import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import replace from '@rollup/plugin-replace';
import babel from '@rollup/plugin-babel';
import terser from '@rollup/plugin-terser';
import typescript from '@rollup/plugin-typescript';
import json from '@rollup/plugin-json';
import polyfillNode from 'rollup-plugin-polyfill-node';
import dts from 'rollup-plugin-dts';

const basePlugins = [
    typescript({ tsconfig: './tsconfig.json' }),
    commonjs(),
    babel({
        babelHelpers: 'bundled',
        extensions: ['.ts'],
    }),
    json(),
    terser(),
    replace({
        'process.env.NODE_ENV': JSON.stringify(process.env.NODE_ENV),
        preventAssignment: true,
    }),
];

const nodeConfig = {
    input: 'src/index.ts',
    output: [
        { file: 'dist/cjs/index.cjs', format: 'cjs', sourcemap: true },
        { file: 'dist/es/index.js', format: 'es', sourcemap: true },
    ],
    plugins: [
        ...basePlugins,
        resolve({
            preferBuiltins: true,
        }),
        replace({
            'process.env.BROWSER': JSON.stringify(false),
            preventAssignment: true,
        }),
    ],
};

const browserConfig = {
    input: 'src/index.ts',
    output: [{ file: 'dist/browser/index.js', format: 'es', sourcemap: true }],
    plugins: [
        ...basePlugins,
        resolve({
            browser: true,
            preferBuiltins: false,
        }),
        polyfillNode(),
        replace({
            'process.env.BROWSER': JSON.stringify(true),
            preventAssignment: true,
        }),
    ],
};

const typesConfig = {
    input: 'src/index.ts',
    output: [{ file: 'dist/types/index.d.ts', format: 'es' }],
    plugins: [dts()],
};

export default [nodeConfig, browserConfig, typesConfig];
