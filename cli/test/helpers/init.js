const fs = require("fs");
const path = require("path");
const yaml = require("js-yaml");

process.env.TS_NODE_PROJECT = path.resolve("tsconfig.test.json");
process.env.NODE_ENV = "development";

function isLocalnet() {
  const homeDir = process.env.HOME || process.env.USERPROFILE;
  const configPath = path.join(
    homeDir,
    ".config",
    "solana",
    "cli",
    "config.yml"
  );
  const configFile = fs.readFileSync(configPath, "utf8");
  const config = yaml.load(configFile);

  if (config && typeof config === "object" && "json_rpc_url" in config) {
    const rpcUrl = config["json_rpc_url"];
    return rpcUrl.includes("localhost") || rpcUrl.includes("127.0.0.1");
  }

  throw Error("Failed to determine Solana cluster type");
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
