import typescript from "@rollup/plugin-typescript";
import nodePolyfills from 'rollup-plugin-polyfill-node';
import pkg from "./package.json";

const rolls = (fmt) => ({
  input: "src/index.ts",
  output: {
    dir: "dist",
    format: fmt,
    entryFileNames: `${fmt}/[name].${fmt === "cjs" ? "cjs" : "js"}`,
    name: pkg.name,
    globals: {
      "@coral-xyz/anchor": "anchor",
      circomlibjs: "circomlibjs",
      ffjavascript: "ffjavascript",
      snarkjs: "snarkjs",      
    },
  },
  external: [
    "@coral-xyz/anchor", 
    "circomlibjs", 
    "ffjavascript", 
    "snarkjs",
    "@solana/web3.js",
    "@solana/spl-token",
    "tweetnacl",
    "axios",
    "chai",
    "fs",
    "@solana/spl-account-compression",
    "os",
    "child_process",
    "decimal.js",
    "case-anything",
    "@noble/hashes/sha256",
    "@coral-xyz/borsh",
    "@coral-xyz/anchor/dist/cjs/utils/bytes",
    "@lightprotocol/circuit-lib.js",
    "@lightprotocol/prover.js",
    "@lightprotocol/account.rs"


  ],
  plugins: [
    typescript({
      target: fmt === "es" ? "ES2022" : "ES2017",
      outDir: `dist/${fmt}`,
      rootDir: "src",
    }),
    nodePolyfills({include: ["fs", "child_process", "os", "assert"]}),
  ],
});

export default [rolls("umd"), rolls("cjs"), rolls("es")];
