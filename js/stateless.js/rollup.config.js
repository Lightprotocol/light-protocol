import typescript from '@rollup/plugin-typescript';
import nodePolyfills from 'rollup-plugin-polyfill-node';
import dts from 'rollup-plugin-dts';
import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import copy from 'rollup-plugin-copy';

const rolls = (fmt, env) => ({
    input: 'src/index.ts',
    output: {
        dir: `dist/${fmt}/${env}`,
        format: fmt,
        entryFileNames: `[name].${fmt === 'cjs' ? 'cjs' : 'js'}`,
        sourcemap: true,
    },
    external: ['@solana/web3.js', '@coral-xyz/anchor'],
    plugins: [
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
        /// Note: This is a temporary hack. Consuming browser apps need access
        /// to the wasm files next to the sdk bundle, for both browser and node.
        /// We will remove this once we've extracted test-helpers (test-rpc.ts,
        /// merkle-tree.ts, which require hasher.rs) into its own library.
        ...(fmt === 'es'
            ? [
                  copy({
                      targets: [
                          {
                              src: 'node_modules/@lightprotocol/hasher.rs/dist/hasher_wasm_simd_bg.wasm',
                              dest: `dist/${fmt}/${env}`,
                          },
                          {
                              src: 'node_modules/@lightprotocol/hasher.rs/dist/light_wasm_hasher_bg.wasm',
                              dest: `dist/${fmt}/${env}`,
                          },
                      ],
                  }),
              ]
            : []),
    ].filter(Boolean),
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
    rolls('es', 'node'),
    typesConfig,
];
