import path from "path";
import fs from "fs";
import {
  killProcess,
  killProcessByPort,
  spawnBinary,
  waitForServers,
} from "./process";
import { LIGHT_PROVER_PROCESS_NAME, BASE_PATH } from "./constants";
import { downloadProverBinary } from "./downloadProverBinary";

const KEYS_DIR = "proving-keys/";

export async function killProver() {
  await killProcess(getProverNameByArch());
  await killProcess(LIGHT_PROVER_PROCESS_NAME);
}

/**
 * Ensures the prover binary exists, downloading it if necessary
 */
async function ensureProverBinary(): Promise<void> {
  const binaryPath = getProverPathByArch();
  const binaryName = getProverNameByArch();

  if (fs.existsSync(binaryPath)) {
    return;
  }

  console.log("Prover binary not found. Downloading...");

  try {
    await downloadProverBinary(binaryPath, binaryName);
  } catch (error) {
    throw new Error(
      `Failed to download prover binary: ${error instanceof Error ? error.message : String(error)}\n` +
        `Please download manually from: https://github.com/Lightprotocol/light-protocol/releases`,
    );
  }
}

export async function startProver(proverPort: number, redisUrl?: string) {
  await ensureProverBinary();

  await killProver();
  await killProcessByPort(proverPort);

  const keysDir = path.join(path.resolve(__dirname, BASE_PATH), KEYS_DIR);
  const args = ["start"];

  args.push("--keys-dir", keysDir);
  args.push("--prover-address", `0.0.0.0:${proverPort}`);
  args.push("--auto-download", "true");

  if (redisUrl) {
    args.push("--redis-url", redisUrl);
  }

  spawnBinary(getProverPathByArch(), args);
  await waitForServers([{ port: proverPort, path: "/" }]);
  console.log(`Prover started successfully!`);
}

export function getProverNameByArch(): string {
  const nodePlatform = process.platform;
  const nodeArch = process.arch;

  if (!nodePlatform || !nodeArch) {
    throw new Error("Unsupported platform or architecture");
  }

  let goPlatform: string = nodePlatform;
  let goArch: string = nodeArch;

  if (nodeArch === "x64") {
    goArch = "amd64";
  }
  if (nodePlatform === "win32") {
    goPlatform = "windows";
  }

  let binaryName = `prover-${goPlatform}-${goArch}`;

  if (goPlatform === "windows") {
    binaryName += ".exe";
  }
  return binaryName;
}

export function getProverPathByArch(): string {
  let binaryName = getProverNameByArch();
  const binDir = path.resolve(__dirname, BASE_PATH);
  binaryName = path.join(binDir, binaryName);

  return binaryName;
}

export async function healthCheck(
  port: number,
  retries = 3,
  timeout = 3000,
): Promise<boolean> {
  const fetch = (await import("node-fetch")).default;
  for (let i = 0; i < retries; i++) {
    try {
      const res = await fetch(`http://localhost:${port}/health`);
      if (res.ok) {
        console.log("Health check passed!");
        return true;
      }
    } catch (e) {
      console.error("Health check error:", e);
    }
    await new Promise((r) => setTimeout(r, timeout));
  }
  console.log("Health check failed after all attempts.");
  return false;
}
