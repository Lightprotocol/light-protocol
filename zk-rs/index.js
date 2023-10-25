// Import our outputted wasm ES6 module
// Which, export default's, an initialization function
import init from "./pkg/zk_rs.js";

const runWasm = async () => {
  // Instantiate our wasm module
  const zkrs = await init("./pkg/zk_rs_bg.wasm");
  const result = zkrs.add(24, 24);
  console.log("result: ", result);
  const hash = zkrs.hash("priv_key", "commitment", 123);
  console.log("hash: ", hash);
  // Set the result onto the body
  document.body.textContent = `hash: ${hash}`;
};
runWasm();