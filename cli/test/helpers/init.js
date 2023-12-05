const { execSync } = require("child_process");
const path = require("path");

process.env.TS_NODE_PROJECT = path.resolve("tsconfig.test.json");
process.env.NODE_ENV = "development";

function isLocalnet() {
  const output = execSync("solana config get", { encoding: "utf8" });
  return output.includes("localhost") || output.includes("127.0.0.1");
}

// For localnet, enable atomic transactions if they weren't explicitly
// disabled.
if (
  isLocalnet() &&
  process.env.LIGHT_PROTOCOL_ATOMIC_TRANSACTIONS !== "false"
) {
  process.env.LIGHT_PROTOCOL_ATOMIC_TRANSACTIONS = "true";
}

global.oclif = global.oclif || {};
global.oclif.columns = 80;
