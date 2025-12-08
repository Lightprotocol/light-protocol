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
import axios from "axios";

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
    return `\nPhoton indexer ${PHOTON_VERSION} (commit ${PHOTON_GIT_COMMIT}) not found. Please install it by running: "cargo install --git ${PHOTON_GIT_REPO} --rev ${PHOTON_GIT_COMMIT} --locked --force"`;
  } else if (USE_PHOTON_FROM_GIT) {
    return `\nPhoton indexer ${PHOTON_VERSION} not found. Please install it by running: "cargo install --git ${PHOTON_GIT_REPO} --locked --force"`;
  } else {
    return `\nPhoton indexer ${PHOTON_VERSION} not found. Please install it by running: "cargo install photon-indexer --version ${PHOTON_VERSION} --locked --force"`;
  }
}

async function waitForIndexerSync(
  rpcUrl: string,
  indexerPort: number,
  timeoutMs: number = 60000,
): Promise<void> {
  const startTime = Date.now();
  const interval = 500;

  while (Date.now() - startTime < timeoutMs) {
    try {
      const [validatorSlotRes, indexerSlotRes] = await Promise.all([
        axios.post(
          rpcUrl,
          { jsonrpc: "2.0", id: 1, method: "getSlot", params: [] },
          { timeout: 5000 },
        ),
        axios.post(
          `http://127.0.0.1:${indexerPort}`,
          { jsonrpc: "2.0", id: 1, method: "getIndexerSlot", params: [] },
          { timeout: 5000 },
        ),
      ]);

      const validatorSlot = validatorSlotRes.data?.result;
      const indexerSlot = indexerSlotRes.data?.result;

      if (
        typeof validatorSlot === "number" &&
        typeof indexerSlot === "number"
      ) {
        const slotDiff = validatorSlot - indexerSlot;
        if (slotDiff <= 5) {
          console.log(
            `Indexer synced (validator: ${validatorSlot}, indexer: ${indexerSlot})`,
          );
          return;
        }
        console.log(
          `Waiting for indexer sync... (validator: ${validatorSlot}, indexer: ${indexerSlot}, diff: ${slotDiff})`,
        );
      }
    } catch {
      // Ignore errors during sync check, just retry
    }

    await new Promise((resolve) => setTimeout(resolve, interval));
  }

  throw new Error(
    `Indexer failed to sync with validator within ${timeoutMs / 1000}s`,
  );
}

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

    spawnBinary(INDEXER_PROCESS_NAME, args);
    await waitForServers([{ port: indexerPort, path: "/getIndexerHealth" }]);
    await waitForIndexerSync(rpcUrl, indexerPort);
    console.log("Indexer started successfully!");
  }
}

export async function killIndexer() {
  await killProcess(INDEXER_PROCESS_NAME);
}
