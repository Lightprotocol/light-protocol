# Usage in the browser

## Copying the circuits (.wasm and .zkey files)

This library includes a file that need to be served from your project's public directory. We provide a shell script to automate this step.

To run this script, execute the following command in your project's root directory:

`bash ./node_modules/@lightprotocol/zk.js/copy-circuits.sh`

This will copy all necessary .wasm and .zkey files to ./public/build-circuits/\*.wasm in your project.

This assumes that static files are served from /public/ such as in nextjs projects.
