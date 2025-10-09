import path from "path";
import fs from "fs";
import {
  killProcess,
  killProcessByPort,
  spawnBinary,
  waitForServers,
} from "./process";
import { LIGHT_PROVER_PROCESS_NAME, BASE_PATH } from "./constants";
import find from "find-process";
import { downloadProverBinary } from "./downloadProverBinary";

const KEYS_DIR = "proving-keys/";

export async function killProver() {
  await killProcess(getProverNameByArch());
  await killProcess(LIGHT_PROVER_PROCESS_NAME);
}

export async function isProverRunningWithFlags(
  runMode?: string,
  circuits?: string[],
  proverPort?: number,
  redisUrl?: string,
): Promise<boolean> {
  // Use find-process to get prover processes by name pattern
  const proverProcesses = await find("name", "prover-");

  const expectedArgs = [];
  if (runMode) {
    expectedArgs.push("--run-mode", runMode);
  }
  if (Array.isArray(circuits)) {
    for (const c of circuits) {
      expectedArgs.push("--circuit", c);
    }
  }
  if (proverPort) {
    expectedArgs.push("--prover-address", `0.0.0.0:${proverPort}`);
  }
  if (redisUrl) {
    expectedArgs.push("--redis-url", redisUrl);
  }

  let found = false;
  for (const proc of proverProcesses) {
    if (
      proc.cmd &&
      (proc.cmd.includes("prover-") || proc.name.startsWith("prover-"))
    ) {
      console.log("\n[Prover Process Detected]");
      console.log(`  PID: ${proc.pid}`);
      console.log(`  Command: ${proc.cmd}`);
      let matches = true;
      for (const arg of expectedArgs) {
        if (!proc.cmd.includes(arg)) {
          matches = false;
          break;
        }
      }
      if (matches) {
        found = true;
        console.log(
          "\x1b[32m✔ Prover is already running with the same configuration.\x1b[0m",
        );
        console.log(
          "  To restart the prover, stop the process above or use the --force flag.\n",
        );
        break;
      } else {
        const missing = proc.cmd
          ? expectedArgs.filter((arg) => !proc.cmd!.includes(arg))
          : [];
        if (missing.length > 0) {
          console.log(
            `  (Not a match for current request. Missing args: ${missing.join(", ")})`,
          );
        }
      }
    }
  }
  if (!found) {
    console.log(
      "\x1b[33mNo running prover found with the requested configuration.\x1b[0m",
    );
  }
  return found;
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
        `Please download manually from: https://github.com/Lightprotocol/light-protocol/releases`
    );
  }
}

export async function startProver(
  proverPort: number,
  runMode: string | undefined,
  circuits: string[] | undefined = [],
  force: boolean = false,
  redisUrl?: string,
) {
  await ensureProverBinary();

  if (
    !force &&
    (await isProverRunningWithFlags(runMode, circuits, proverPort))
  ) {
    return;
  }

  console.log("Kill existing prover process...");
  await killProver();
  await killProcessByPort(proverPort);

  const keysDir = path.join(path.resolve(__dirname, BASE_PATH), KEYS_DIR);
  const args = ["start"];

  // Handle fallback to local-rpc mode if no mode or circuits specified
  if ((!circuits || circuits.length === 0) && runMode == null) {
    runMode = "local-rpc";
    console.log(`Starting prover with fallback ${runMode} mode...`);
  }

  // Add run-mode first to avoid flag parsing issues
  if (runMode != null) {
    args.push("--run-mode", runMode);
  }

  args.push("--keys-dir", keysDir);
  args.push("--prover-address", `0.0.0.0:${proverPort}`);
  args.push("--auto-download", "true");

  for (const circuit of circuits) {
    args.push("--circuit", circuit);
  }

  if (runMode != null) {
    console.log(`Starting prover in ${runMode} mode...`);
  } else if (circuits && circuits.length > 0) {
    console.log(`Starting prover with circuits: ${circuits.join(", ")}...`);
  }

  if (redisUrl) {
    args.push("--redis-url", redisUrl);
  }

  spawnBinary(getProverPathByArch(), args);
  await waitForServers([{ port: proverPort, path: "/" }]);
  console.log(`Prover started successfully!`);
}

export function getProverNameByArch(): string {
  let platform = process.platform;
  let arch = process.arch;

  if (!platform || !arch) {
    throw new Error("Unsupported platform or architecture");
  }

  if (arch === "x64") {
    arch = "amd64";
  }
  if (platform === "win32") {
    platform = "windows";
  }

  let binaryName = `prover-${platform}-${arch}`;

  if (platform === "windows") {
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
