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
  // Temporary fix for the case when prover is instantiated via prover.sh:
  await killProcess(LIGHT_PROVER_PROCESS_NAME);
}

export async function startProver(
  proverPort: number,
  proveCompressedAccounts: boolean,
  proveNewAddresses: boolean,
) {
  if (!proveCompressedAccounts && !proveNewAddresses) {
    console.log(
      "No flags provided. Please provide at least one flag to start the prover.",
    );
    process.exit(1);
  }
  console.log("Kill existing prover process...");
  await killProver();
  await killProcessByPort(proverPort);

  const keysDir = path.join(__dirname, "../..", "bin", KEYS_DIR);
  const args = ["start"];
  args.push(`--inclusion=${proveCompressedAccounts ? "true" : "false"}`);
  args.push(`--non-inclusion=${proveNewAddresses ? "true" : "false"}`);
  args.push("--keys-dir", keysDir);
  args.push("--prover-address", `0.0.0.0:${proverPort}`);

  console.log("Starting prover...");
  spawnBinary(getProverNameByArch(), args);
  await waitForServers([{ port: proverPort, path: "/" }]);
  console.log("Prover started successfully!");
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

  // We need to provide the full path to the binary because it's not in the PATH.
  const binDir = path.join(__dirname, "../..", "bin");
  binaryName = path.join(binDir, binaryName);

  return binaryName;
}
