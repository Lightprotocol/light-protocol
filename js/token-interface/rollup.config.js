import typescript from '@rollup/plugin-typescript';
import dts from 'rollup-plugin-dts';
import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';

const inputs = {
    index: 'src/index.ts',
    'instructions/index': 'src/instructions/index.ts',
    'kit/index': 'src/kit/index.ts',
    'nowrap/index': 'src/nowrap/index.ts',
};

const external = [
    '@coral-xyz/borsh',
    '@lightprotocol/stateless.js',
    '@solana/buffer-layout',
    '@solana/buffer-layout-utils',
    '@solana/compat',
    '@solana/instruction-plans',
    '@solana/kit',
    '@solana/spl-token',
    '@solana/web3.js',
    'bn.js',
    'buffer',
];

const jsConfig = format => ({
    input: inputs,
    output: {
        dir: `dist/${format}`,
        format,
        entryFileNames: `[name].${format === 'cjs' ? 'cjs' : 'js'}`,
        chunkFileNames: `[name]-[hash].${format === 'cjs' ? 'cjs' : 'js'}`,
        sourcemap: true,
    },
    external,
    plugins: [
        typescript({
            target: format === 'es' ? 'ES2022' : 'ES2017',
            outDir: `dist/${format}`,
        }),
        commonjs(),
        resolve({
            extensions: ['.mjs', '.js', '.json', '.ts'],
        }),
    ],
    onwarn(warning, warn) {
        if (warning.code !== 'CIRCULAR_DEPENDENCY') {
            warn(warning);
        }
    },
});

const dtsEntry = (input, file) => ({
    input,
    output: [{ file, format: 'es' }],
    external,
    plugins: [
        dts({
            respectExternal: true,
            tsconfig: './tsconfig.json',
        }),
    ],
});

export default [
    jsConfig('cjs'),
    jsConfig('es'),
    dtsEntry('src/index.ts', 'dist/types/index.d.ts'),
    dtsEntry('src/instructions/index.ts', 'dist/types/instructions/index.d.ts'),
    dtsEntry('src/kit/index.ts', 'dist/types/kit/index.d.ts'),
    dtsEntry('src/nowrap/index.ts', 'dist/types/nowrap/index.d.ts'),
];
