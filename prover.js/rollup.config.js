import typescript from "@rollup/plugin-typescript";
import pkg from "./package.json";

const rolls = (fmt) => ({
    input: 'src/index.ts',
    output: {
        dir: "dist",
        format: fmt,
        entryFileNames: `${fmt}/[name].${
            fmt === "cjs" ? "cjs" : "js"
        }`,
        name: pkg.name,
        globals: {
            "os": "os",
            "@coral-xyz/anchor": "anchor",
        }
    },
    external: ["os", "@coral-xyz/anchor", "circomlibjs", "ffjavascript", "snarkjs"],
    plugins: [
        typescript({
            target: fmt === "es" ? "ES2022" : "ES2017",
            outDir: `dist/${fmt}`,
            rootDir: "src",
        })
    ],
});

export default [
    rolls("umd"),
    rolls("cjs"),
    rolls("es")
];
