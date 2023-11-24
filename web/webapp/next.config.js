const CopyPlugin = require("copy-webpack-plugin");

/** @type {import('next').NextConfig} */
const nextConfig = {
    
    // experimental: {
    //     outputFileTracingIncludes: {
    //         '/': ['./node_modules/**/*.wasm'],
    //     }
    // },    
    output: "export",
    distDir: "dist",
    cleanDistDir: false,
    webpack: (config, {isServer}) => {        
        config.plugins.push(new CopyPlugin({
            patterns: [
                { from: "public/wasm", to: "./static/wasm" },
            ],
        }))
        // Fixes npm packages that depend on `fs` module
        if (!isServer) {
            config.resolve.fallback = {
                fs: false,
                child_process: false,
                readline: false,
            };
        }        
        patchWasmModuleImport(config, isServer);
        return config;
    },
};

function patchWasmModuleImport(config, isServer) {
    config.experiments = Object.assign(config.experiments || {}, {
        asyncWebAssembly: true,
    });

    config.optimization.moduleIds = 'named';

    config.module.rules.push({
        test: /\.wasm$/,
        type: 'webassembly/async',
    });

    // TODO: improve this function -> track https://github.com/vercel/next.js/issues/25852
    if (isServer) {
        config.output.webassemblyModuleFilename = './../static/wasm/[modulehash].wasm';
    } else {
        config.output.webassemblyModuleFilename = 'static/wasm/[modulehash].wasm';
    }
}

module.exports = nextConfig;
