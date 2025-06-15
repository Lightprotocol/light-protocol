import path from "path";
import fs from "fs";
import {
  killProcess,
  killProcessByPort,
  spawnBinary,
  waitForServers,
} from "./process";
import { LIGHT_PROVER_PROCESS_NAME } from "./constants";
import find from "find-process";

const KEYS_DIR = "proving-keys/";
const MAX_START_RETRIES = 3;

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

export async function startProver(
  proverPort: number,
  runMode: string | undefined,
  circuits: string[] | undefined = [],
  force: boolean = false,
  redisUrl?: string,
) {
  if (
    !force &&
    (await isProverRunningWithFlags(runMode, circuits, proverPort, redisUrl))
  ) {
    return;
  }

  console.log("Kill existing prover process...");
  await killProver();
  await killProcessByPort(proverPort);

  // Verify prover binary exists
  const proverPath = getProverPathByArch();
  if (!fs.existsSync(proverPath)) {
    throw new Error(
      `Prover binary not found at ${proverPath}. Please run: npx nx build @lightprotocol/zk-compression-cli`,
    );
  }

  const keysDir = path.join(__dirname, "../..", "bin", KEYS_DIR);

  // Verify proving keys exist
  if (!fs.existsSync(keysDir) || fs.readdirSync(keysDir).length === 0) {
    throw new Error(
      `Proving keys not found at ${keysDir}. Please run: ./prover/server/scripts/download_keys.sh light`,
    );
  }

  const args = ["start"];
  args.push("--keys-dir", keysDir);
  args.push("--prover-address", `0.0.0.0:${proverPort}`);
  if (runMode != null) {
    args.push("--run-mode", runMode);
  }

  for (const circuit of circuits) {
    args.push("--circuit", circuit);
  }

  if (runMode != null) {
    console.log(`Starting prover in ${runMode} mode...`);
  } else if (circuits && circuits.length > 0) {
    console.log(`Starting prover with circuits: ${circuits.join(", ")}...`);
  }

  if ((!circuits || circuits.length === 0) && runMode == null) {
    runMode = "rpc";
    args.push("--run-mode", runMode);
    console.log(`Starting prover with fallback ${runMode} mode...`);
  }

  if (redisUrl) {
    args.push("--redis-url", redisUrl);
  }

  let lastError: Error | undefined;
  for (let attempt = 1; attempt <= MAX_START_RETRIES; attempt++) {
    try {
      console.log(
        `Starting prover (attempt ${attempt}/${MAX_START_RETRIES})...`,
      );

      const proverProcess = spawnBinary(proverPath, args);

      await new Promise((resolve) => setTimeout(resolve, 1000));

      if (proverProcess.exitCode !== null) {
        throw new Error(
          `Prover process exited with code ${proverProcess.exitCode}`,
        );
      }

      try {
        await waitForServers([{ port: proverPort, path: "/" }]);
        console.log(`Prover started successfully!`);

        // Perform health check
        const healthy = await healthCheck(proverPort);
        if (!healthy) {
          console.warn(
            "Prover started but health check failed - it may still be initializing",
          );
        }

        return;
      } catch (error) {
        // Kill the process if it didn't start properly
        proverProcess.kill();
        throw error;
      }
    } catch (error) {
      lastError = error as Error;
      console.error(`Failed to start prover on attempt ${attempt}:`, error);

      if (attempt < MAX_START_RETRIES) {
        console.log(`Waiting 5 seconds before retry...`);
        await new Promise((resolve) => setTimeout(resolve, 5000));

        // Clean up before retry
        await killProver();
        await killProcessByPort(proverPort);
      }
    }
  }

  throw new Error(
    `Failed to start prover after ${MAX_START_RETRIES} attempts. Last error: ${lastError?.message}`,
  );
}

export function getProverNameByArch(): string {
  const platform = process.platform;
  const arch = process.arch;

  if (!platform || !arch) {
    throw new Error("Unsupported platform or architecture");
  }

  // Map Node.js arch names to our binary naming convention
  const archMap: Record<string, string> = {
    x64: "x64",
    arm64: "arm64",
    x86: "x86",
    aarch64: "arm64",
  };

  const mappedArch = archMap[arch] || arch;
  let binaryName = `prover-${platform}-${mappedArch}`;

  if (platform === "win32") {
    binaryName += ".exe";
  }
  return binaryName;
}

export function getProverPathByArch(): string {
  let binaryName = getProverNameByArch();
  // We need to provide the full path to the binary because it's not in the PATH.
  const binDir = path.join(__dirname, "../..", "bin");
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
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), timeout);

      const res = await fetch(`http://localhost:${port}/health`, {
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (res.ok) {
        const data = await res.text();
        console.log("Health check passed!", data);
        return true;
      } else {
        console.error(`Health check returned status ${res.status}`);
      }
    } catch (e: any) {
      if (e.name === "AbortError") {
        console.error(
          `Health check attempt ${i + 1} timed out after ${timeout}ms`,
        );
      } else {
        console.error(`Health check attempt ${i + 1} failed:`, e.message);
      }
    }
    if (i < retries - 1) {
      await new Promise((r) => setTimeout(r, timeout));
    }
  }
  console.log("Health check failed after all attempts.");
  return false;
}
