import typescript from '@rollup/plugin-typescript';
import pkg from './package.json' assert { type: 'json' };
import nodePolyfills from 'rollup-plugin-polyfill-node';
import dts from 'rollup-plugin-dts';
import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';

/// TODO: hasher.rs is only required to build your own Merkle tree. So we should
/// move it to a dedicated testing lib that exposes test-rpc and manual poseidon
/// hashing. This way we can remove hasher.rs from the bundle.
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
                      '@solana/web3.js': 'web3.js',
                      '@coral-xyz/anchor': 'anchor',
                  },
              }
            : {}),
    },
    external: ['@solana/web3.js', '@coral-xyz/anchor'],
    plugins: [
        typescript({
            target: fmt === 'es' ? 'ES2022' : 'ES2017',
            outDir: `dist/${fmt}`,
            rootDir: 'src',
        }),
        commonjs(),
        /// TODO: distinguish between node and browser
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
