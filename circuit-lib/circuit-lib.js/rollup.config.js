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
    sourcemap: true,
    globals: {
      "@coral-xyz/anchor": "anchor",
      circomlibjs: "circomlibjs",
      ffjavascript: "ffjavascript",
      snarkjs: "snarkjs",
      crypto: "crypto",
      assert: "assert",
      "@noble/hashes/sha512": "sha512",
      "@noble/hashes/utils": "utils",
      "@noble/curves/abstract/edwards": "edwards",
      "@noble/curves/abstract/modular": "modular",
    },
  },
  external: [
    "@coral-xyz/anchor",
    "circomlibjs",
    "ffjavascript",
    "snarkjs",
    "crypto",
    "assert",
    "@noble/hashes/sha512",
    "@noble/hashes/utils",
    "@noble/curves/abstract/edwards",
    "@noble/curves/abstract/modular"
  ],
  plugins: [
    typescript({
      target: fmt === "es" ? "ES2022" : "ES2020",
      outDir: `dist/${fmt}`,
      rootDir: "src",
    }),
    nodePolyfills({include: ["assert", "crypto"]})
  ],
});

export default [rolls("umd"), rolls("cjs"), rolls("es")];
