import path from "path";
import {
  killProcess,
  killProcessByPort,
  spawnBinary,
  waitForServers,
} from "./process";
import { LIGHT_PROVER_PROCESS_NAME } from "./constants";
import find from "find-process";

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

export async function startProver(
  proverPort: number,
  runMode: string | undefined,
  circuits: string[] | undefined = [],
  force: boolean = false,
  redisUrl?: string,
) {
  if (
    !force &&
    (await isProverRunningWithFlags(runMode, circuits, proverPort))
  ) {
    return;
  }

  console.log("Kill existing prover process...");
  await killProver();
  await killProcessByPort(proverPort);

  const keysDir = path.join(__dirname, "../..", "bin", KEYS_DIR);
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

  spawnBinary(getProverPathByArch(), args);
  await waitForServers([{ port: proverPort, path: "/" }]);
  console.log(`Prover started successfully!`);
}

export function getProverNameByArch(): string {
  const platform = process.platform;
  const arch = process.arch;

  if (!platform || !arch) {
    throw new Error("Unsupported platform or architecture");
  }

  let binaryName = `prover-${platform}-${arch}`;

  if (platform.toString() === "windows") {
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
