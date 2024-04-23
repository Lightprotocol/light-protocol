import typescript from "@rollup/plugin-typescript";
import { wasm } from "@rollup/plugin-wasm";
import pkg from "./package.json";
import copy from "rollup-plugin-copy";

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
    inline !== "slim" && wasm({ targetEnv: "auto-inline" }),
    typescript({
      target: fmt === "es" ? "ES2022" : "ES2017",
      outDir: `dist/${outdir(fmt, platform, inline)}`,
      rootDir: "src",
    }),
    /// Note: This is a temporary hack to copy the wasm files to the dist folder
    /// Which then allows `stateless.js` to copy them to its own dist where then
    /// web-apps can consume them. We will remove this once we extract the
    /// test-helpers pkg into its own library to stop the bloat. This is
    /// dependent on fixing the photon indexer to return correct merkle proofs
    /// such that `stateless.js` doesn't require its own hasher. Long term, we
    /// need to optimize our hasher library regardless, to more efficiently
    /// support custom hashing schemas.
    copy({
      targets: [
        {
          src: "src/main/wasm/light_wasm_hasher_bg.wasm",
          dest: "dist/",
        },
        {
          src: "src/main/wasm-simd/hasher_wasm_simd_bg.wasm",
          dest: "dist/",
        },
      ],
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
