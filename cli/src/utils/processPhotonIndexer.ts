import which from "which";
import { killProcess, spawnBinary, waitForServers } from "./process";
import {
  INDEXER_PROCESS_NAME,
  PHOTON_VERSION,
  USE_PHOTON_FROM_GIT,
  PHOTON_GIT_REPO,
  PHOTON_GIT_COMMIT,
} from "./constants";
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
  if (USE_PHOTON_FROM_GIT && PHOTON_GIT_COMMIT) {
    return `\nLatest Photon indexer not found. Please install it by running: "cargo install --git ${PHOTON_GIT_REPO} --rev ${PHOTON_GIT_COMMIT} --locked"`;
  } else if (USE_PHOTON_FROM_GIT) {
    return `\nLatest Photon indexer not found. Please install it by running: "cargo install --git ${PHOTON_GIT_REPO} --locked"`;
  } else {
    return `\nLatest Photon indexer not found. Please install it by running: "cargo install photon-indexer --version ${PHOTON_VERSION} --locked"`;
  }
}

export async function startIndexer(
  rpcUrl: string,
  indexerPort: number,
  grpcPort: number = 50051,
  checkPhotonVersion: boolean = true,
  photonDatabaseUrl?: string,
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
      "--grpc-port",
      grpcPort.toString(),
    ];
    if (photonDatabaseUrl) {
      args.push("--db-url", photonDatabaseUrl);
    }
    spawnBinary(INDEXER_PROCESS_NAME, args);
    await waitForServers([{ port: indexerPort, path: "/getIndexerHealth" }]);
    console.log("Indexer started successfully!");
  }
}

export async function killIndexer() {
  await killProcess(INDEXER_PROCESS_NAME);
}
