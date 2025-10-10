import typescript from "@rollup/plugin-typescript";
import dts from "rollup-plugin-dts";
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
import json from "@rollup/plugin-json";

const rolls = (fmt) => ({
  input: "src/index.ts",
  output: {
    dir: `dist/${fmt}`,
    format: fmt,
    entryFileNames: `[name].${fmt === "cjs" ? "cjs" : "js"}`,
    sourcemap: true,
  },
  external: [
    "@solana/web3.js",
    "@solana/spl-token",
    "@solana/codecs",
    "@lightprotocol/stateless.js",
    "@lightprotocol/hasher.rs",
    "litesvm",
    "buffer",
    "@coral-xyz/borsh",
  ],
  plugins: [
    typescript({
      target: fmt === "es" ? "ES2022" : "ES2017",
      outDir: `dist/${fmt}`,
      rootDir: "src",
    }),
    commonjs(),
    resolve({
      preferBuiltins: true,
    }),
    json(),
  ].filter(Boolean),
  onwarn(warning, warn) {
    if (warning.code !== "CIRCULAR_DEPENDENCY") {
      warn(warning);
    }
  },
});

const typesConfig = {
  input: "src/index.ts",
  output: [{ file: "dist/types/index.d.ts", format: "es" }],
  plugins: [dts()],
};

export default [rolls("cjs"), rolls("es"), typesConfig];
