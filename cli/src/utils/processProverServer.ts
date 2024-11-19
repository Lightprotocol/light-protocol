import path from "path";
import {
  executeCommand,
  killProcess,
  killProcessByPort,
  spawnBinary,
  waitForServers,
} from "./process";
import { LIGHT_PROVER_PROCESS_NAME } from "./constants";

const KEYS_DIR = "proving-keys/";

export async function killProver() {
  await killProcess(getProverNameByArch());
  await killProcess(LIGHT_PROVER_PROCESS_NAME);
}

export async function startProver(
  proverPort: number,
  runMode: string | undefined,
  circuits: string[] | undefined = [],
) {
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
