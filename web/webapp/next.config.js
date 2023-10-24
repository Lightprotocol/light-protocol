/** @type {import('next').NextConfig} */

const nextConfig = {
  output: "export",
  distDir: "dist",
  webpack: (config, { isServer }) => {
    // Fixes npm packages that depend on `fs` module
    // if (!isServer) {
    config.resolve.fallback = {
      fs: false,
      child_process: false,
      readline: false,
    };
    // }

    return config;
  },
};

module.exports = nextConfig;
