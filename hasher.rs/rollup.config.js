import typescript from "@rollup/plugin-typescript";
import { wasm } from "@rollup/plugin-wasm";
import pkg from "./package.json";

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
            "os": "os",
            "@coral-xyz/anchor": "anchor",
        }
    },
    external: ["os", "@coral-xyz/anchor"],
    plugins: [
        inline !== "slim" &&
        wasm({ targetEnv: "auto-inline" }),
        typescript({
            target: fmt === "es" ? "ES2022" : "ES2017",
            outDir: `dist/${outdir(fmt, platform, inline)}`,
            rootDir: "src",
        })
    ],
});

export default [
    rolls("umd", "browser", "fat"),
    rolls("cjs", "browser", "fat"),
    rolls("es", "browser", "fat"),
    rolls("cjs", "browser", "slim"),
    rolls("es", "browser", "slim"),
];
