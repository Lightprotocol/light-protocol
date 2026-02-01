import which from "which";
import { killProcess, spawnBinary, waitForServers } from "./process";
import { FORESTER_PROCESS_NAME } from "./constants";
import { exec } from "node:child_process";
import * as util from "node:util";
import { exit } from "node:process";
import * as fs from "fs";
import * as path from "path";

const execAsync = util.promisify(exec);

async function isForesterInstalled(): Promise<boolean> {
  try {
    const resolvedOrNull = which.sync("forester", { nothrow: true });
    return resolvedOrNull !== null;
  } catch (error) {
    return false;
  }
}

function getForesterInstallMessage(): string {
  return `\nForester not found. Please install it by running: "cargo install --git https://github.com/Lightprotocol/light-protocol forester --locked --force"`;
}

export interface ForesterConfig {
  rpcUrl: string;
  wsRpcUrl: string;
  indexerUrl: string;
  proverUrl: string;
  payer: string;
  foresterPort: number;
  compressiblePdaPrograms?: string[];
}

/**
 * Starts the forester service for auto-compression of compressible accounts.
 *
 * @param config - Forester configuration
 */
export async function startForester(config: ForesterConfig) {
  await killForester();

  if (!(await isForesterInstalled())) {
    console.log(getForesterInstallMessage());
    return exit(1);
  }

  console.log("Starting forester...");

  const args: string[] = [
    "start",
    "--rpc-url",
    config.rpcUrl,
    "--ws-rpc-url",
    config.wsRpcUrl,
    "--indexer-url",
    config.indexerUrl,
    "--prover-url",
    config.proverUrl,
    "--payer",
    config.payer,
    "--api-server-port",
    config.foresterPort.toString(),
    "--enable-compressible",
  ];

  // Add compressible PDA programs if specified
  if (config.compressiblePdaPrograms && config.compressiblePdaPrograms.length > 0) {
    for (const program of config.compressiblePdaPrograms) {
      args.push("--compressible-pda-program", program);
    }
  }

  spawnBinary(FORESTER_PROCESS_NAME, args);
  await waitForServers([{ port: config.foresterPort, path: "/health" }]);
  console.log("Forester started successfully!");
}

export async function killForester() {
  await killProcess(FORESTER_PROCESS_NAME);
}

/**
 * Gets the payer keypair as a JSON array string for forester.
 * Reads from ~/.config/solana/id.json or SOLANA_PAYER environment variable.
 *
 * @returns JSON array string of the keypair bytes
 */
export function getPayerForForester(): string {
  // Check for SOLANA_PAYER environment variable first
  if (process.env.SOLANA_PAYER) {
    return process.env.SOLANA_PAYER;
  }

  // Default to standard Solana keypair location
  const homeDir = process.env.HOME || process.env.USERPROFILE || "";
  const keypairPath = path.join(homeDir, ".config", "solana", "id.json");

  if (fs.existsSync(keypairPath)) {
    const keypairData = fs.readFileSync(keypairPath, "utf-8");
    return keypairData.trim();
  }

  throw new Error(
    "No payer keypair found. Set SOLANA_PAYER environment variable or create ~/.config/solana/id.json",
  );
}
