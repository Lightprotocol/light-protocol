import typescript from '@rollup/plugin-typescript';
import pkg from './package.json' assert { type: 'json' };
import nodePolyfills from 'rollup-plugin-polyfill-node';
import dts from 'rollup-plugin-dts';
import resolve from '@rollup/plugin-node-resolve';

const rolls = fmt => ({
    input: 'src/index.ts',
    output: {
        dir: 'dist',
        format: fmt,
        entryFileNames: `${fmt}/[name].${fmt === 'cjs' ? 'cjs' : 'js'}`,
        name: pkg.name,
        ...(fmt === 'umd'
            ? {
                  globals: {
                      '@coral-xyz/anchor': 'anchor',
                      '@solana/web3.js': 'web3.js',
                      tweetnacl: 'tweetnacl',
                      bs58: 'bs58',
                      '@lightprotocol/hasher.rs': 'hasher.rs',
                      //   superstruct: 'superstruct',
                      //   '@noble/hashes/sha3': 'sha3',
                  },
              }
            : {}),
    },
    external: [
        '@solana/web3.js',
        '@coral-xyz/anchor',
        'tweetnacl',
        'bs58',
        '@lightprotocol/hasher.rs',
        // 'superstruct',
        // '@noble/hashes/sha3',
    ],
    plugins: [
        typescript({
            target: fmt === 'es' ? 'ES2022' : 'ES2017',
            outDir: `dist/${fmt}`,
            rootDir: 'src',
        }),
        /// add to buffer as external if remove
        /// or try distinguish between node and browser
        nodePolyfills(),
        resolve(),
    ],
});

const typesConfig = {
    input: 'src/index.ts',
    output: [{ file: 'dist/types/index.d.ts', format: 'es' }],
    plugins: [dts()],
};

export default [rolls('umd'), rolls('cjs'), rolls('es'), typesConfig];

// import typescript from '@rollup/plugin-typescript';
// import nodePolyfills from 'rollup-plugin-polyfill-node';
// import resolve from '@rollup/plugin-node-resolve';
// import dts from 'rollup-plugin-dts';
// import babel from '@rollup/plugin-babel';
// import replace from '@rollup/plugin-replace';
// import commonjs from '@rollup/plugin-commonjs';
// import json from '@rollup/plugin-json';

// const basePlugins = [
//     typescript({ tsconfig: './tsconfig.json' }),
//     babel({
//         babelHelpers: 'bundled',
//         extensions: ['.ts'],
//     }),
// ];

// const nodeConfig = {
//     input: 'src/index.ts',
//     output: [
//         { file: 'dist/cjs/index.cjs', format: 'cjs', sourcemap: true },
//         { file: 'dist/es/index.js', format: 'es', sourcemap: true },
//     ],
//     external: [
//         'bs58',
//         'node-fetch',
//         '@solana/web3.js',
//         '@coral-xyz/anchor',
//         'tweetnacl',
//     ],
//     plugins: [
//         resolve({
//             preferBuiltins: true,
//         }),
//         replace({
//             'process.env.BROWSER': JSON.stringify(false),
//             preventAssignment: true,
//         }),
//         ...basePlugins,
//     ],
// };

// const browserConfig = {
//     input: 'src/index.ts',
//     output: [{ file: 'dist/browser/index.js', format: 'es', sourcemap: true }],
//     external: [
//         // 'bs58',
//         // '@solana/web3.js',
//         // '@coral-xyz/anchor',
//         // 'tweetnacl',
//     ],
//     plugins: [
//         resolve({
//             mainFields: ['browser', 'module', 'main'],
//             browser: true,
//             preferBuiltins: false,
//         }),
//         ...basePlugins,
//         nodePolyfills(),
//         commonjs(),
//         replace({
//             'process.env.BROWSER': JSON.stringify(true),
//             preventAssignment: true,
//         }),
//         json(),
//     ],
// };

// const typesConfig = {
//     input: 'src/index.ts',
//     output: [{ file: 'dist/types/index.d.ts', format: 'es' }],
//     plugins: [dts()],
// };

// export default [nodeConfig, browserConfig, typesConfig];
