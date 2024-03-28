import typescript from '@rollup/plugin-typescript';
import pkg from './package.json';
import nodePolyfills from 'rollup-plugin-polyfill-node';

const rolls = fmt => ({
  input: 'src/index.ts',
  output: {
    dir: 'dist',
    format: fmt,
    entryFileNames: `${fmt}/[name].${fmt === 'cjs' ? 'cjs' : 'js'}`,
    name: pkg.name,
    globals: {
      '@coral-xyz/anchor': 'anchor',
      '@coral-xyz/anchor/dist/cjs/utils/bytes': 'bytes',
      '@solana/web3.js': 'web3.js',
      '@solana/spl-account-compression': 'spl-account-compression',
      '@metaplex-foundation/beet': 'beet',
      '@metaplex-foundation/beet-solana': 'beet-solana',
      '@lightprotocol/stateless.js': 'stateless.js',
      '@lightprotocol/hasher.rs': 'hasher.rs',
      '@solana/spl-token': 'spl-token',
      buffer: 'buffer',
      crypto: 'crypto',
      superstruct: 'superstruct',
      tweetnacl: 'tweetnacl',
    },
  },
  external: [
    '@solana/web3.js',
    '@solana/spl-account-compression',
    '@solana/spl-token',
    '@coral-xyz/anchor',
    '@coral-xyz/anchor/dist/cjs/utils/bytes',
    '@lightprotocol/stateless.js',
    '@lightprotocol/hasher.rs',
    '@metaplex-foundation/beet',
    '@metaplex-foundation/beet-solana',
    'buffer',
    'superstruct',
    'tweetnacl',
  ],
  plugins: [
    typescript({
      target: fmt === 'es' ? 'ES2022' : 'ES2017',
      outDir: `dist/${fmt}`,
      rootDir: 'src',
    }),
    nodePolyfills({ include: ['buffer', 'crypto'] }),
  ],
});

export default [rolls('umd'), rolls('cjs'), rolls('es')];
