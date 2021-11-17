# Build wasm
cargo build --target=wasm32-unknown-unknown --release
# Create bindings in ../src/ for use in main.js
wasm-bindgen target/wasm32-unknown-unknown/release/webassembly.wasm --out-dir ../src/ --no-typescript --target nodejs