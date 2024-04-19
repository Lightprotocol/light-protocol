import typescript from "@rollup/plugin-typescript";
import { wasm } from "@rollup/plugin-wasm";
import pkg from "./package.json";
import resolve from "@rollup/plugin-node-resolve";

const outdir = (fmt, platform, inline) => {
  return `${platform}${inline ? `-${inline}` : ""}/${fmt}`;
};

const rolls = (fmt, platform, inline) => ({
  input: `src/main/index_${platform}${inline ? `_${inline}` : ""}.ts`,
  output: {
    dir: "dist",
    format: fmt,
    entryFileNames: `${outdir(fmt, platform, inline)}/[name].${
      fmt === "cjs" ? "cjs" : "js"
    }`,
    name: pkg.name,
    globals: {
      os: "os",
      "@coral-xyz/anchor": "anchor",
    },
  },
  external: ["os", "@coral-xyz/anchor"],
  plugins: [
    resolve(),
    /// TODO: test: inline test for now.
    /// We're never inlining for browser compat with wasm-pack wasm-pack expects
    /// a separate wasm file to be in the same directory if async.
    // inline !== "slim" && wasm({ targetEnv: "auto", max-size:0 }),
    inline !== "slim" && wasm({ targetEnv: "auto-inline" }),
    inline === "slim" && wasm({ targetEnv: "auto", maxSize: 0 }),
    typescript({
      target: fmt === "es" ? "ES2022" : "ES2017",
      outDir: `dist/${outdir(fmt, platform, inline)}`,
      rootDir: "src",
    }),
  ],
});

export default [
  rolls("umd", "browser", "fat"),
  rolls("cjs", "browser", "fat"),
  rolls("es", "browser", "fat"),
  rolls("cjs", "browser", "slim"),
  rolls("es", "browser", "slim"),
];
