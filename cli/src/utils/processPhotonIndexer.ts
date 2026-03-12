import which from "which";
import { killProcess, spawnBinary, waitForServers } from "./process";
import { INDEXER_PROCESS_NAME } from "./constants";
import {
  PHOTON_VERSION,
  PHOTON_GIT_REPO,
  PHOTON_GIT_COMMIT,
} from "./photonVersion.generated";
import { exec } from "node:child_process";
import * as util from "node:util";
import { exit } from "node:process";

const execAsync = util.promisify(exec);

async function isExpectedPhotonVersion(
  requiredVersion: string,
): Promise<boolean> {
  try {
    const { stdout } = await execAsync("photon --version");
    const version = stdout.trim();
    return version.includes(requiredVersion);
  } catch (error) {
    console.error("Error checking Photon version:", error);
    return false;
  }
}

function getPhotonInstallMessage(): string {
  return `\nPhoton indexer ${PHOTON_VERSION} (commit ${PHOTON_GIT_COMMIT}) not found. Please install it by running: "cargo install --git ${PHOTON_GIT_REPO} --rev ${PHOTON_GIT_COMMIT} --locked --force"`;
}

export async function startIndexer(
  rpcUrl: string,
  indexerPort: number,
  checkPhotonVersion: boolean = true,
  photonDatabaseUrl?: string,
  proverUrl?: string,
  startSlot?: number,
) {
  await killIndexer();
  const resolvedOrNull = which.sync("photon", { nothrow: true });
  if (
    resolvedOrNull === null ||
    (checkPhotonVersion && !(await isExpectedPhotonVersion(PHOTON_VERSION)))
  ) {
    console.log(getPhotonInstallMessage());
    return exit(1);
  } else {
    console.log("Starting indexer...");
    const args: string[] = [
      "--port",
      indexerPort.toString(),
      "--rpc-url",
      rpcUrl,
    ];
    if (photonDatabaseUrl) {
      args.push("--db-url", photonDatabaseUrl);
    }
    if (proverUrl) {
      args.push("--prover-url", proverUrl);
    }
    if (startSlot !== undefined) {
      args.push("--start-slot", startSlot.toString());
    }

    const env = { ...process.env };
    if (
      env.PHOTON_INDEXING_COMMITMENT === undefined &&
      (rpcUrl.includes("127.0.0.1") ||
        rpcUrl.includes("localhost") ||
        rpcUrl.includes("0.0.0.0"))
    ) {
      env.PHOTON_INDEXING_COMMITMENT = "finalized";
    }

    spawnBinary(INDEXER_PROCESS_NAME, args, env);
    await waitForServers([{ port: indexerPort, path: "/getIndexerHealth" }]);
    console.log("Indexer started successfully!");
  }
}

export async function killIndexer() {
  await killProcess(INDEXER_PROCESS_NAME);
}
