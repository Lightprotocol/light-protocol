import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
import terser from "@rollup/plugin-terser";
import typescript from "@rollup/plugin-typescript";
import json from "@rollup/plugin-json";
import polyfills from "rollup-plugin-polyfill-node";
import pkg from "./package.json" assert { type: "json" };

export default {
  input: "src/index.ts", // your main TS source file
  output: [
    // {
    //   file: "lib/esm/index.js", // output file
    //   format: "esm", // ES module format
    //   sourcemap: true,
    // },
    // {
    //   file: "lib/cjs/index.js", // output file
    //   format: "cjs", // CommonJS format
    //   sourcemap: true,
    // },
    {
      file: "lib/browser/index.js",
      format: "es",
      //   name: "zk.js",
      sourcemap: true,
    },
  ],
  onwarn: function (warning, warn) {
    if (warning.code === "EVAL") return;
    if (warning.code === "THIS_IS_UNDEFINED") return;
    if (warning.code === "CIRCULAR_DEPENDENCY") return;
    warn(warning);
  },
  external: Object.keys(pkg.dependencies),
  plugins: [
    commonjs(), // so Rollup can convert CommonJS to ES modules
    typescript({
      tsconfig: "./tsconfig.base.json",
      moduleResolution: "node",
      outDir: "",
      target: "es2019",
      outputToFilesystem: false,
    }),
    resolve({
      browser: true,
      extensions: [".js", ".ts"],
      dedupe: ["bn.js", "buffer"],
      preferBuiltins: false,
    }), // so Rollup can find external modules
    json(),
    terser(), // minify the output
    polyfills(),
  ],
};
