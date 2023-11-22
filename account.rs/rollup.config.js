import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
// import { terser } from "rollup-plugin-terser";
import typescript from '@rollup/plugin-typescript';
import multi from "@rollup/plugin-multi-entry";
import wasm from "@rollup/plugin-wasm";
import pkg from "./package.json";
import rust from "@wasm-tool/rollup-plugin-rust";

export default {
  input: ["src/index.ts"],
  output: {
    sourcemap: false,
    format: "cjs",
    name: pkg.name,
    file: "dist/bundle.cjs.js",
    globals: { "@coral-xyz/anchor": "@coral-xyz/anchor" }
  },
  external: ["@coral-xyz/anchor", "@noble/hashes"],
  plugins: [
    multi(),
    commonjs({
      include: [
        "src/**/*.js",
        "src/**/*.ts",
        "node_modules/**"
      ]
    }),
    resolve({
      browser: true,
      extensions: [".js", ".ts", ".wasm"]
    }),
    wasm(
      { targetEnv: "auto-inline" }
      ),
    rust(),
    typescript()
    // If we're building for prod, minify with terser()
  ],
  watch: {
    clearScreen: false
  }
};
