import resolve from '@rollup/plugin-node-resolve';
import typescript from '@rollup/plugin-typescript';
import pkg from './package.json';
import nodePolyfills from 'rollup-plugin-polyfill-node';
import commonjs from '@rollup/plugin-commonjs';

const rolls = fmt => ({
    input: 'src/index.ts',
    output: {
        dir: 'dist',
        format: fmt,
        entryFileNames: `${fmt}/[name].${fmt === 'cjs' ? 'cjs' : 'js'}`,
        name: pkg.name,
        globals: {
            '@coral-xyz/anchor': 'anchor',
            '@solana/web3.js': 'web3.js',
            buffer: 'Buffer',
            crypto: 'Crypto',
            superstruct: 'superstruct',
            tweetnacl: 'tweetnacl',
        },
    },
    external: [
        '@solana/web3.js',
        '@coral-xyz/anchor',
        'superstruct',
        'tweetnacl',
    ],
    plugins: [
        resolve({
            mainFields: ['browser', 'module', 'main'],
            browser: true,
            extensions: ['.mjs', '.js', '.json', '.ts'],
            preferBuiltins: false,
        }),
        commonjs(),
        typescript({
            target: fmt === 'es' ? 'ES2022' : 'ES2017',
            outDir: `dist/${fmt}`,
            rootDir: 'src',
        }),
        nodePolyfills(),
    ],
});

export default [rolls('umd'), rolls('cjs'), rolls('es')];
