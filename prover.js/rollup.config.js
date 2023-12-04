import typescript from "@rollup/plugin-typescript";
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
  external: ["@coral-xyz/anchor", "circomlibjs", "ffjavascript", "snarkjs"],
  plugins: [
    typescript({
      target: fmt === "es" ? "ES2022" : "ES2017",
      outDir: `dist/${fmt}`,
      rootDir: "src",
    }),
  ],
});

export default [rolls("umd"), rolls("cjs"), rolls("es")];
