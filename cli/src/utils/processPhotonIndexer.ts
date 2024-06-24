import which from "which";
import { killProcess, spawnBinary, waitForServers } from "./process";
import { INDEXER_PROCESS_NAME, PHOTON_VERSION } from "./constants";
import { exec } from "node:child_process";
import * as util from "node:util";

export async function startIndexer(
  rpcUrl: string,
  indexerPort: number,
  checkPhotonVersion: boolean = true,
  photonDatabaseUrl?: string,
) {
  await killIndexer();
  const resolvedOrNull = which.sync("photon", { nothrow: true });
  if (
    resolvedOrNull === null ||
    (checkPhotonVersion && !(await isExpectedPhotonVersion(PHOTON_VERSION)))
  ) {
    const message = `Photon indexer not found. Please install it by running "cargo install photon-indexer --version ${PHOTON_VERSION} --locked"`;
    console.log(message);
    throw new Error(message);
  } else {
    console.log("Starting indexer...");
    let args: string[] = [];
    if (photonDatabaseUrl) {
      args = [
        "--db-url",
        photonDatabaseUrl,
        "--port",
        indexerPort.toString(),
        "--rpc-url",
        rpcUrl,
      ];
    }
    spawnBinary(INDEXER_PROCESS_NAME, args);
    await waitForServers([{ port: indexerPort, path: "/getIndexerHealth" }]);
    console.log("Indexer started successfully!");
  }
}

async function killIndexer() {
  await killProcess(INDEXER_PROCESS_NAME);
}

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
