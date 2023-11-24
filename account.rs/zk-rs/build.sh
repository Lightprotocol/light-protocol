rm -rf pkg/node
rm -rf pkg/bundler
rm -rf pkg/web

wasm-pack build --target bundler --out-dir pkg/bundler  
wasm-pack build --target nodejs --out-dir pkg/node
wasm-pack build --target web --out-dir pkg/web