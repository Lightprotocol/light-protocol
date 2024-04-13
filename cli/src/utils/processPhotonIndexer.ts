import which from "which";
import { sleep } from "@lightprotocol/stateless.js";
import { spawnBinary, killProcessByName } from "./process";
import { INDEXER_PROCESS_NAME } from "./constants";

export async function startIndexer() {
  console.log("Kill existing indexer process...");
  await killProcessByName(INDEXER_PROCESS_NAME);
  const resolvedOrNull = which.sync("photon", { nothrow: true });
  if (resolvedOrNull === null) {
    const message =
      "Photon indexer not found. Please install it by running `cargo install photon-indexer --version 0.11.0`";
    console.log(message);
    throw new Error(message);
  } else {
    console.log("Starting indexer...");
    spawnBinary("photon", false);
    console.log("Indexer started successfully!");
    await sleep(5000);
  }
}
